//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Solve CSP to find ALL matches (exhaustive search)
//! 3. Yield matches
//!

use crate::RelationType;
use crate::bytes::Sym;
use crate::pattern::{
    BasePattern, Constraint, ConstraintValue, DirectedEdge, EdgeConstraint, Pattern,
};
use crate::query::{QueryError, compile_query};
use crate::tree::Word;
use crate::tree::{Tree, WordId};
use fastbit::{BitFixed, BitRead, BitWrite};
use std::collections::HashMap;
use std::sync::Arc;

pub type Bindings = HashMap<String, WordId>;
#[derive(Debug)]
pub struct Match {
    pub tree: Arc<Tree>,
    pub bindings: Bindings,
}

/// Check if a string (from string pool) matches a constraint value (literal or regex)
fn matches_constraint_value(tree: &Tree, str_id: Sym, value: &ConstraintValue) -> bool {
    match value {
        ConstraintValue::Literal(literal) => {
            tree.string_pool.compare_bytes(str_id, literal.as_bytes())
        }
        ConstraintValue::Regex(_pattern, regex) => {
            let bytes = tree.string_pool.resolve(str_id);
            if let Ok(s) = std::str::from_utf8(&bytes) {
                regex.is_match(s)
            } else {
                false // Invalid UTF-8 never matches
            }
        }
    }
}

/// Check if a tree word satisfies a pattern variable's constraint
fn satisfies_var_constraint(tree: &Tree, word: &Word, constraint: &Constraint) -> bool {
    match constraint {
        Constraint::Lemma(value) => matches_constraint_value(tree, word.lemma, value),
        Constraint::UPOS(value) => matches_constraint_value(tree, word.upos, value),
        Constraint::XPOS(value) => matches_constraint_value(tree, word.xpos, value),
        Constraint::Form(value) => matches_constraint_value(tree, word.form, value),
        Constraint::DepRel(value) => matches_constraint_value(tree, word.deprel, value),
        Constraint::Feature(key, value) => {
            let key_bytes = key.as_bytes();
            word.feats.iter().any(|(k, v)| {
                tree.string_pool.compare_bytes(*k, key_bytes)
                    && matches_constraint_value(tree, *v, value)
            })
        }
        Constraint::Misc(key, value) => {
            let key_bytes = key.as_bytes();
            word.misc.iter().any(|(k, v)| {
                tree.string_pool.compare_bytes(*k, key_bytes)
                    && matches_constraint_value(tree, *v, value)
            })
        }
        Constraint::And(constraints) => constraints
            .iter()
            .all(|constraint| satisfies_var_constraint(tree, word, constraint)),
        Constraint::Or(constraints) => constraints
            .iter()
            .any(|constraint| satisfies_var_constraint(tree, word, constraint)),
        Constraint::Not(inner_constraint) => {
            !satisfies_var_constraint(tree, word, inner_constraint)
        }
        Constraint::Any => true, // No filtering
        Constraint::IsChild(label) => {
            if let Some(cv) = label {
                word.head.is_some() && matches_constraint_value(tree, word.deprel, cv)
            } else {
                word.head.is_some()
            }
        }
        Constraint::HasChild(label) => {
            if let Some(cv) = label {
                word.children.iter().any(|&child_id| {
                    matches_constraint_value(tree, tree.words[child_id].deprel, cv)
                })
            } else {
                !word.children.is_empty()
            }
        }
    }
}

fn satisfies_arc_constraint(
    tree: &Tree,
    from_word_id: WordId,
    to_word_id: WordId,
    edge_constraint: &EdgeConstraint,
) -> bool {
    let satisfies_constraint = match edge_constraint.relation {
        RelationType::Child => {
            tree.check_rel(from_word_id, to_word_id)
                && edge_constraint.label.as_ref().is_none_or(|cv| {
                    let actual_deprel = tree.word(to_word_id).unwrap().deprel;
                    matches_constraint_value(tree, actual_deprel, cv)
                })
        }
        RelationType::Precedes => from_word_id < to_word_id,
        RelationType::ImmediatelyPrecedes => to_word_id == from_word_id + 1,
        RelationType::PrecedesWithin(n) => {
            from_word_id < to_word_id && to_word_id - from_word_id <= n
        }
    };

    if edge_constraint.negated {
        !satisfies_constraint
    } else {
        satisfies_constraint
    }
}

fn has_any_match(tree: &Tree, pattern: &BasePattern, initial_bindings: &Bindings) -> bool {
    !solve_with_bindings(tree, pattern, initial_bindings, true).is_empty()
}

/// Process OPTIONAL blocks: extend base bindings with cross-product of all extensions.
/// Each OPTIONAL is evaluated independently against base_bindings.
/// Returns all combinations of optional extensions (or just base if none match).
fn process_optionals(
    tree: &Tree,
    base_bindings: Bindings,
    optional_patterns: &[BasePattern],
) -> Vec<Bindings> {
    let extension_sets: Vec<Vec<Bindings>> = optional_patterns
        .iter()
        .map(|optional| solve_with_bindings(tree, optional, &base_bindings, false))
        .collect();

    let mut results = vec![base_bindings];

    // Compute cross-product of all extensions

    for extensions in extension_sets {
        if extensions.is_empty() {
            // No match for this OPTIONAL - keep results unchanged
            continue;
        }
        // Replace each current result with extended versions
        let mut new_results = Vec::new();
        for result in &results {
            for ext in &extensions {
                let mut combined = result.clone();
                // Merge in the new bindings from this OPTIONAL
                for (k, v) in ext {
                    if !combined.contains_key(k) {
                        combined.insert(k.clone(), *v);
                    }
                }
                new_results.push(combined);
            }
        }
        results = new_results;
    }

    results
}

/// Search with pre-bound variables from initial_bindings.
/// Returns all possible bindings (including initial bindings), or just the first if first_only.
fn solve_with_bindings(
    tree: &Tree,
    pattern: &BasePattern,
    initial_bindings: &Bindings,
    first_only: bool,
) -> Vec<Bindings> {
    let num_words = tree.words.len();
    let mut assign: Vec<Option<WordId>> = vec![None; pattern.var_constraints.len()];
    let mut assigned_words: BitFixed<u64> = BitFixed::new(num_words);

    // Pre-assign from initial_bindings and validate constraints on pre-bound variables.
    // Every pre-bound word is excluded from new variables, whether or not this pattern
    // mentions its variable.
    for (var_name, &word_id) in initial_bindings {
        assigned_words.set(word_id);
        if let Some(&var_id) = pattern.var_ids.get(var_name) {
            // Check that pre-bound variable satisfies its constraints in this pattern
            let word = &tree.words[word_id];
            let constr = &pattern.var_constraints[var_id];
            if !satisfies_var_constraint(tree, word, constr) {
                return Vec::new(); // Pre-bound variable fails constraint, no solutions possible
            }
            assign[var_id] = Some(word_id);
        }
    }

    // Check edges between pre-bound variables. The DFS only checks an edge when it
    // assigns one of its endpoints, so edges whose endpoints are both pre-bound
    // would otherwise never be checked.
    for ec in &pattern.edge_constraints {
        if let (Some(from), Some(to)) = (
            assign[pattern.var_ids[&ec.from]],
            assign[pattern.var_ids[&ec.to]],
        ) && !satisfies_arc_constraint(tree, from, to, ec)
        {
            return Vec::new();
        }
    }

    // Initialize domains (node consistency)
    let mut domains: Vec<BitFixed<u64>> = vec![BitFixed::new(num_words); pattern.var_names.len()];
    for (var_id, constr) in pattern.var_constraints.iter().enumerate() {
        if assign[var_id].is_some() {
            continue; // Already validated above
        }
        for (word_id, word) in tree.words.iter().enumerate() {
            if !assigned_words.test(word_id) && satisfies_var_constraint(tree, word, constr) {
                domains[var_id].set(word_id);
            }
        }
        if domains[var_id].count_ones() == 0 {
            return Vec::new(); // no solution possible
        }
    }

    dfs(
        tree,
        pattern,
        &assign,
        &domains,
        &assigned_words,
        first_only,
    )
}

/// Find all matches of a pattern in a tree
pub fn find_all_matches(tree: Tree, pattern: &Pattern) -> Vec<Match> {
    let tree = Arc::new(tree);
    find_bindings(&tree, pattern, false)
        .into_iter()
        .map(|bindings| Match {
            tree: Arc::clone(&tree),
            bindings,
        })
        .collect()
}

/// Find the bindings of all matches of a pattern in a borrowed tree
pub fn find_all_bindings(tree: &Tree, pattern: &Pattern) -> Vec<Bindings> {
    find_bindings(tree, pattern, false)
}

/// Check if a tree has at least one match
pub fn tree_matches(tree: &Tree, pattern: &Pattern) -> bool {
    !find_bindings(tree, pattern, true).is_empty()
}

fn find_bindings(tree: &Tree, pattern: &Pattern, first_only: bool) -> Vec<Bindings> {
    let empty_bindings = Bindings::new();
    let base_matches =
        solve_with_bindings(tree, &pattern.match_pattern, &empty_bindings, first_only);

    let mut results = Vec::new();
    for base_bindings in base_matches {
        let rejected = pattern
            .except_patterns
            .iter()
            .any(|except| has_any_match(tree, except, &base_bindings));

        if rejected {
            continue;
        }

        if first_only {
            // Skip optionals for existence check - just return first valid match
            results.push(base_bindings);
            return results;
        }

        results.extend(process_optionals(
            tree,
            base_bindings,
            &pattern.optional_patterns,
        ));
    }

    results
}

fn dfs(
    tree: &Tree,
    pattern: &BasePattern,
    assign: &[Option<WordId>],
    domains: &[BitFixed<u64>],
    assigned_words: &BitFixed<u64>,
    first_only: bool,
) -> Vec<Bindings> {
    // No more variables to assign
    if assign.iter().all(|word_id| word_id.is_some()) {
        let mut solution = Bindings::new();
        for (var_id, word_id) in assign.iter().copied().flatten().enumerate() {
            solution.insert(pattern.var_names[var_id].clone(), word_id);
        }
        return vec![solution];
    }

    // Select an unassigned variable with Minimum Remaining Values (MRV)
    let next_var = (0..pattern.var_names.len())
        .filter(|&var_id| assign[var_id].is_none())
        .min_by_key(|&var_id| domains[var_id].count_ones())
        .unwrap();

    let mut solutions: Vec<Bindings> = Vec::new();

    // Try each candidate word for this variable (iterate over set bits in the domain bitset)
    for word_id in domains[next_var].iter() {
        // AllDifferent: Check if word_id is already assigned to another variable using bitset (O(1))
        if assigned_words.test(word_id) {
            continue;
        }

        // Early prune: Check arc consistency with already-assigned neighbors
        if !check_arc_consistency(tree, pattern, assign, next_var, word_id) {
            continue;
        }

        let mut new_assign = assign.to_vec();
        let mut new_domains = domains.to_vec();

        // Assign var <- word_id and update bitset
        new_assign[next_var] = Some(word_id);
        let mut new_assigned_words = assigned_words.clone();
        new_assigned_words.set(word_id);

        // AllDifferent: Remove word_id from all variable domains
        for domain in &mut new_domains {
            domain.reset(word_id);
        }
        if !(0..pattern.var_constraints.len())
            .all(|var_id| new_assign[var_id].is_some() || new_domains[var_id].count_ones() > 0)
        {
            continue;
        }

        // Forward-check: Propagate along edge constraints touching next_var
        // if !forward_check(
        //     tree,
        //     pattern,
        //     next_var,
        //     word_id,
        //     &mut new_assign,
        //     &mut new_domains,
        // ) {
        //     continue;
        // }

        // Recurse - go on to next variable
        solutions.extend(dfs(
            tree,
            pattern,
            &new_assign,
            &new_domains,
            &new_assigned_words,
            first_only,
        ));

        if first_only && !solutions.is_empty() {
            return solutions;
        }
    }
    solutions
}

#[allow(dead_code)]
fn forward_check(
    tree: &Tree,
    pattern: &BasePattern,
    next_var: usize,
    word_id: WordId,
    new_assign: &mut [Option<WordId>],
    new_domains: &mut [BitFixed<u64>],
) -> bool {
    // Propagate along edge constraints incident to next_var
    for directed_edge in &pattern.incident_edges[next_var] {
        let (ec, neighbor_var_id) = match *directed_edge {
            DirectedEdge::Out(eid) => {
                let ec = &pattern.edge_constraints[eid];
                (ec, pattern.var_ids[&ec.to])
            }
            DirectedEdge::In(eid) => {
                let ec = &pattern.edge_constraints[eid];
                (ec, pattern.var_ids[&ec.from])
            }
        };
        if new_assign[neighbor_var_id].is_some() {
            continue;
        }
        for w in new_domains[neighbor_var_id].iter().collect::<Vec<_>>() {
            let ok = match *directed_edge {
                DirectedEdge::Out(_) => satisfies_arc_constraint(tree, word_id, w, ec),
                DirectedEdge::In(_) => satisfies_arc_constraint(tree, w, word_id, ec),
            };
            if !ok {
                new_domains[neighbor_var_id].reset(w);
            }
        }
        if new_domains[neighbor_var_id].count_ones() == 0 {
            return false;
        }
    }
    true
}

fn check_arc_consistency(
    tree: &Tree,
    pattern: &BasePattern,
    assign: &[Option<WordId>],
    next_var: usize,
    word_id: WordId,
) -> bool {
    // Check arc consistency with already-assigned neighbors (early prune)
    for directed_edge in &pattern.incident_edges[next_var] {
        match *directed_edge {
            DirectedEdge::Out(edge_id) => {
                let ec = &pattern.edge_constraints[edge_id];
                let target_var_id = pattern.var_ids[&ec.to];
                if assign[target_var_id]
                    .is_some_and(|tw| !satisfies_arc_constraint(tree, word_id, tw, ec))
                {
                    return false;
                }
            }
            DirectedEdge::In(edge_id) => {
                let ec = &pattern.edge_constraints[edge_id];
                let source_var_id = pattern.var_ids[&ec.from];
                if assign[source_var_id]
                    .is_some_and(|sw| !satisfies_arc_constraint(tree, sw, word_id, ec))
                {
                    return false;
                }
            }
        }
    }
    true
}

/// Search a tree with a pre-compiled pattern
pub fn search_tree(tree: Tree, pattern: &Pattern) -> Vec<Match> {
    find_all_matches(tree, pattern)
}

/// Search a tree with a query string
pub fn search_tree_query(tree: Tree, query: &str) -> Result<Vec<Match>, QueryError> {
    let pattern = compile_query(query)?;
    Ok(find_all_matches(tree, &pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conllu::TreeIterator;
    use std::collections::BTreeMap;

    type Solution = BTreeMap<String, WordId>;

    macro_rules! sol {
        ( $( $key:literal => $val:expr ),* $(,)? ) => {
            Solution::from([ $( ($key.to_string(), $val), )* ])
        };
    }

    /// All solutions for `query`, in a canonical order
    fn solutions(tree: &Tree, query: &str) -> Vec<Solution> {
        let pattern = compile_query(query).unwrap();
        let mut sols: Vec<Solution> = find_all_bindings(tree, &pattern)
            .into_iter()
            .map(|b| b.into_iter().collect())
            .collect();
        sols.sort();
        sols
    }

    /// The (sorted) words bound by a single-variable query
    fn words(tree: &Tree, query: &str) -> Vec<WordId> {
        solutions(tree, query)
            .into_iter()
            .map(|s| {
                assert_eq!(s.len(), 1, "{query}: {s:?}");
                s.into_values().next().unwrap()
            })
            .collect()
    }

    /// helped(0) us(1) to(2) win(3)
    /// helped -obj-> us, helped -xcomp-> win, win -mark-> to
    fn help_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"helped", b"help", b"VERB", b"VBD", None, b"root");
        tree.add_minimal_word(1, b"us", b"we", b"PRON", b"PRP", Some(0), b"obj");
        tree.add_minimal_word(2, b"to", b"to", b"PART", b"TO", Some(3), b"mark");
        tree.add_minimal_word(3, b"win", b"win", b"VERB", b"VB", Some(0), b"xcomp");
        tree.compile_tree();
        tree
    }

    /// saw(0) John(1) running(2) quickly(3)
    /// saw -nsubj-> John, saw -xcomp-> running, running -advmod-> quickly
    fn saw_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"saw", b"see", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"John", b"John", b"PROPN", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"running", b"run", b"VERB", b"_", Some(0), b"xcomp");
        tree.add_minimal_word(3, b"quickly", b"quickly", b"ADV", b"_", Some(2), b"advmod");
        tree.compile_tree();
        tree
    }

    /// was(0) running(1) ,(2) with features and misc
    fn feature_tree() -> Tree {
        let conllu = "1\twas\tbe\tVERB\t_\tNumber=Sing|Tense=Past\t0\troot\t_\tSpaceAfter=No\n\
                      2\trunning\trun\tVERB\t_\tTense=Pres|VerbForm=Part\t1\txcomp\t_\t_\n\
                      3\t,\t,\tPUNCT\t_\t_\t1\tpunct\t_\t_\n";
        TreeIterator::from_string(conllu).next().unwrap().unwrap()
    }

    fn one_word_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"word", b"word", b"NOUN", b"_", None, b"root");
        tree.compile_tree();
        tree
    }

    #[test]
    fn test_node_constraints() {
        let help = help_tree();
        let feats = feature_tree();
        let cases: &[(&Tree, &str, &[WordId])] = &[
            (&help, r#"V [lemma="help"]"#, &[0]),
            (&help, r#"V [upos="VERB"]"#, &[0, 3]),
            (&help, r#"W [form="to"]"#, &[2]),
            (&help, r#"W [xpos="TO"]"#, &[2]),
            (&help, r#"X [deprel="xcomp"]"#, &[3]),
            (&help, r#"N [upos="NOUN"]"#, &[]),
            (&help, "X []", &[0, 1, 2, 3]),
            (&help, r#"V [lemma="help" & upos="VERB"]"#, &[0]),
            (&help, r#"W [upos!="VERB"]"#, &[1, 2]),
            (&help, r#"W [upos="VERB" & xpos!="VBD"]"#, &[3]),
            (&help, r#"N [upos="NOUN" | upos="PRON"]"#, &[1]),
            (&help, r#"N [upos="NOUN" | upos="ADJ"]"#, &[]),
            // Regexes are anchored around the whole alternation
            (&help, r#"W [lemma=/w.*/]"#, &[1, 3]),
            (&help, r#"W [upos=/VERB|PRON/]"#, &[0, 1, 3]),
            (&help, r#"W [form=/h|w/]"#, &[]),
            (&help, r#"W [form=/.*el.*/]"#, &[0]),
            (&help, r#"W [lemma!=/w.*/]"#, &[0, 2]),
            (&help, r#"V [upos="VERB" & lemma=/h.*/]"#, &[0]),
            // Edges to `_`
            (&help, "W []; _ -> W", &[1, 2, 3]),
            (&help, "W []; _ !-> W", &[0]),
            (&help, "W []; _ !-[obj]-> W", &[0, 2, 3]),
            (&help, "W []; _ -/obj|xcomp/-> W", &[1, 3]),
            (&help, "W []; W -> _", &[0, 3]),
            (&help, "W []; W -[mark]-> _", &[3]),
            // Features and misc (keys and values are case-sensitive)
            (&feats, r#"V [feats.Tense="Past"]"#, &[0]),
            (
                &feats,
                r#"V [feats.Tense="Past" & feats.Number="Sing"]"#,
                &[0],
            ),
            (&feats, r#"V [feats.Tense=/.*/]"#, &[0, 1]),
            (&feats, r#"V [feats.Tense="Fut"]"#, &[]),
            (&feats, r#"V [feats.tense="Past"]"#, &[]),
            (&feats, r#"V [feats.Tense="past"]"#, &[]),
            (&feats, r#"V [upos="VERB" & feats.Tense!="Past"]"#, &[1]),
            (&feats, r#"V [misc.SpaceAfter="No"]"#, &[0]),
            (&feats, r#"V [misc.SpaceAfter="Yes"]"#, &[]),
        ];

        for (tree, body, expected) in cases {
            assert_eq!(
                words(tree, &format!("MATCH {{ {body}; }}")),
                *expected,
                "{body}"
            );
        }
    }

    #[test]
    fn test_edges() {
        let help = help_tree();

        // Every assignment is found, including symmetric ones
        let mut coord = Tree::default();
        coord.add_minimal_word(0, b"and", b"and", b"CCONJ", b"_", None, b"root");
        coord.add_minimal_word(1, b"cats", b"cat", b"NOUN", b"_", Some(0), b"conj");
        coord.add_minimal_word(2, b"dogs", b"dog", b"NOUN", b"_", Some(0), b"conj");
        coord.compile_tree();
        assert_eq!(
            solutions(
                &coord,
                "MATCH { C []; N1 []; N2 []; C -[conj]-> N1; C -[conj]-> N2; }"
            ),
            [
                sol! { "C" => 0, "N1" => 1, "N2" => 2 },
                sol! { "C" => 0, "N1" => 2, "N2" => 1 },
            ]
        );

        assert_eq!(
            solutions(
                &help,
                r#"MATCH { V1 [lemma="help"]; V2 []; T []; V1 -> V2; V2 -> T; }"#
            ),
            [sol! { "V1" => 0, "V2" => 3, "T" => 2 }]
        );
        assert_eq!(
            solutions(
                &saw_tree(),
                r#"MATCH { V1 [upos="VERB"]; S []; V2 [upos="VERB"]; V1 -[nsubj]-> S; V1 -> V2; }"#
            ),
            [sol! { "V1" => 0, "S" => 1, "V2" => 2 }]
        );
        assert_eq!(
            solutions(&help, r#"MATCH { V []; O [lemma=/w.*/]; V -[obj]-> O; }"#),
            [sol! { "V" => 0, "O" => 1 }]
        );

        // Negated edges (variables are still all-different)
        assert_eq!(
            solutions(
                &help,
                r#"MATCH { V [upos="VERB"]; T [lemma="to"]; V !-> T; }"#
            ),
            [sol! { "V" => 0, "T" => 2 }]
        );
        assert_eq!(
            solutions(&help, r#"MATCH { V [lemma="help"]; W []; V !-[obj]-> W; }"#),
            [sol! { "V" => 0, "W" => 2 }, sol! { "V" => 0, "W" => 3 }]
        );
        assert_eq!(
            solutions(
                &help,
                "MATCH { V []; Y []; W []; V -[xcomp]-> Y; V !-[obj]-> W; }"
            ),
            [sol! { "V" => 0, "Y" => 3, "W" => 2 }]
        );

        // Regex labels
        assert_eq!(
            solutions(&help, "MATCH { V []; C []; V -/obj|xcomp/-> C; }"),
            [sol! { "V" => 0, "C" => 1 }, sol! { "V" => 0, "C" => 3 }]
        );
        assert_eq!(
            solutions(&help, "MATCH { V []; C []; V -/ob|xc/-> C; }"),
            []
        );
        assert_eq!(
            solutions(&help, "MATCH { V []; C []; V -/x.*/-> C; }"),
            [sol! { "V" => 0, "C" => 3 }]
        );
        assert_eq!(
            solutions(
                &help,
                r#"MATCH { V [lemma="help"]; C []; V !-/obj|xcomp/-> C; }"#
            ),
            [sol! { "V" => 0, "C" => 2 }]
        );

        assert_eq!(
            solutions(&one_word_tree(), "MATCH { P []; C []; P -> C; }"),
            []
        );

        // A pattern with no variables matches once
        assert_eq!(solutions(&help, "MATCH { }"), [sol! {}]);
    }

    #[test]
    fn test_precedence() {
        let help = help_tree();
        let pair = |op: &str, a: &str, b: &str| {
            solutions(
                &help,
                &format!(r#"MATCH {{ A [lemma="{a}"]; B [lemma="{b}"]; A {op} B; }}"#),
            )
        };

        assert_eq!(pair("<<", "help", "win"), [sol! { "A" => 0, "B" => 3 }]);
        assert_eq!(pair("<<", "win", "help"), []);
        assert_eq!(pair("<", "to", "win"), [sol! { "A" => 2, "B" => 3 }]);
        assert_eq!(pair("<", "help", "win"), []);
        // <n< is inclusive, and the window may extend past the sentence
        assert_eq!(pair("<2<", "help", "win"), []);
        assert_eq!(pair("<3<", "help", "win"), [sol! { "A" => 0, "B" => 3 }]);
        assert_eq!(pair("<100<", "help", "win"), [sol! { "A" => 0, "B" => 3 }]);
        assert_eq!(pair("<3<", "win", "help"), []);

        // All ordered pairs at distance 1 (3), or at distance 1 or 2 (3 + 2)
        let count =
            |op: &str| solutions(&help, &format!("MATCH {{ A []; B []; A {op} B; }}")).len();
        assert_eq!(count("<"), 3);
        assert_eq!(count("<1<"), 3);
        assert_eq!(count("<2<"), 5);
        assert_eq!(
            solutions(&one_word_tree(), "MATCH { A []; B []; A << B; }"),
            []
        );

        assert_eq!(
            solutions(
                &help,
                r#"MATCH { A [lemma="help"]; B [lemma="we"]; C [lemma="to"]; A << B; B << C; }"#
            ),
            [sol! { "A" => 0, "B" => 1, "C" => 2 }]
        );

        // Combined with dependency edges
        assert_eq!(
            solutions(&help, "MATCH { A []; B []; A -[xcomp]-> B; A << B; }"),
            [sol! { "A" => 0, "B" => 3 }]
        );
        assert_eq!(
            solutions(&help, "MATCH { A []; B []; A -[xcomp]-> B; B << A; }"),
            []
        );
    }

    #[test]
    fn test_except() {
        let tree = saw_tree();
        let verbs =
            |except: &str| words(&tree, &format!(r#"MATCH {{ V [upos="VERB"]; }} {except}"#));

        assert_eq!(verbs(r#"EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }"#), [0]);
        assert_eq!(
            verbs(r#"EXCEPT { N [upos="NOUN"]; V -[obj]-> N; }"#),
            [0, 2]
        );
        assert!(verbs("EXCEPT { V -> _; }").is_empty());
        // Any EXCEPT block can reject
        assert!(
            verbs(
                r#"EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }
                   EXCEPT { C [upos="VERB"]; V -[xcomp]-> C; }"#
            )
            .is_empty()
        );
        assert_eq!(
            verbs(
                r#"EXCEPT { C [upos="VERB"]; M [upos="ADV"]; V -[xcomp]-> C; C -[advmod]-> M; }"#
            ),
            [2]
        );
        assert_eq!(
            solutions(
                &tree,
                r#"MATCH { V [upos="VERB"]; S [upos="PROPN"]; V -[nsubj]-> S; }
                   EXCEPT { C [upos="VERB"]; V -[xcomp]-> C; }"#
            ),
            []
        );
    }

    #[test]
    fn test_optional() {
        let tree = saw_tree();
        let subj = r#"OPTIONAL { S [upos="PROPN"]; V -[nsubj]-> S; }"#;

        // Bound when found, absent when not
        assert_eq!(
            solutions(&tree, &format!(r#"MATCH {{ V [lemma="see"]; }} {subj}"#)),
            [sol! { "V" => 0, "S" => 1 }]
        );
        assert_eq!(
            solutions(&tree, &format!(r#"MATCH {{ V [lemma="run"]; }} {subj}"#)),
            [sol! { "V" => 2 }]
        );

        // Blocks are independent
        assert_eq!(
            solutions(
                &tree,
                &format!(
                    r#"MATCH {{ V [lemma="see"]; }} {subj}
                       OPTIONAL {{ N [upos="NOUN"]; V -[obj]-> N; }}
                       OPTIONAL {{ C [upos="VERB"]; V -[xcomp]-> C; }}"#
                )
            ),
            [sol! { "V" => 0, "S" => 1, "C" => 2 }]
        );

        // Combined with EXCEPT
        assert_eq!(
            solutions(
                &tree,
                &format!(
                    r#"MATCH {{ V [upos="VERB"]; }}
                       EXCEPT {{ M [upos="ADV"]; V -[advmod]-> M; }} {subj}"#
                )
            ),
            [sol! { "V" => 0, "S" => 1 }]
        );

        // Multiple matches in several blocks give the cross-product
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"helped", b"help", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"He", b"he", b"PRON", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"us", b"we", b"PRON", b"_", Some(0), b"obj");
        tree.add_minimal_word(3, b"quickly", b"quickly", b"ADV", b"_", Some(0), b"advmod");
        tree.add_minimal_word(4, b"again", b"again", b"ADV", b"_", Some(0), b"advmod");
        tree.compile_tree();
        assert_eq!(
            solutions(
                &tree,
                r#"MATCH { V [lemma="help"]; }
                   OPTIONAL { P [upos="PRON"]; V -> P; }
                   OPTIONAL { A [upos="ADV"]; V -> A; }"#
            ),
            [
                sol! { "V" => 0, "P" => 1, "A" => 3 },
                sol! { "V" => 0, "P" => 2, "A" => 3 },
                sol! { "V" => 0, "P" => 1, "A" => 4 },
                sol! { "V" => 0, "P" => 2, "A" => 4 },
            ]
        );
    }

    #[test]
    fn test_extension_edges_between_prebound_vars() {
        // MATCH alone yields (V, W) = (0,1), (0,3), (2,1), (2,3)
        let tree = saw_tree();
        let pairs = |except: &str| -> Vec<(WordId, WordId)> {
            let query = format!(
                r#"MATCH {{ V [upos="VERB"]; W [upos=/PROPN|ADV/]; }} EXCEPT {{ {except} }}"#
            );
            solutions(&tree, &query)
                .into_iter()
                .map(|s| (s["V"], s["W"]))
                .collect()
        };

        // Unlabeled edge: rejects only pairs where V heads W
        assert_eq!(pairs("V -> W;"), [(0, 3), (2, 1)]);
        // Precedence: rejects only pairs where W immediately precedes V
        assert_eq!(pairs("W < V;"), [(0, 1), (0, 3), (2, 3)]);
        // Labeled edge: W's deprel alone isn't enough; V must be W's head.
        // quickly is an advmod of running, so (0, 3) must survive.
        assert_eq!(pairs("V -[advmod]-> W;"), [(0, 1), (0, 3), (2, 1)]);
        // Negated edges: "V -> W or W -> V" as a clause over bound variables
        assert_eq!(pairs("V !-> W; W !-> V;"), [(0, 1), (2, 3)]);

        // OPTIONAL: an edge between pre-bound variables must hold for S to bind
        assert_eq!(
            solutions(
                &tree,
                r#"MATCH { V [upos="VERB"]; W [upos="ADV"]; }
                   OPTIONAL { S [upos="PROPN"]; V -> W; V -> S; }"#
            ),
            [sol! { "V" => 0, "W" => 3 }, sol! { "V" => 2, "W" => 3 }]
        );
    }

    #[test]
    fn test_extension_vars_distinct_from_match_words() {
        // New EXCEPT/OPTIONAL variables never bind a MATCH-bound word, even when
        // the block doesn't mention that MATCH variable
        let tree = saw_tree();
        let base = r#"MATCH { V [upos="VERB"]; S []; V -[nsubj]-> S; }"#;

        // Only one subject, so there's no *other* subject to reject on
        assert_eq!(
            solutions(&tree, &format!("{base} EXCEPT {{ X []; V -[nsubj]-> X; }}")),
            [sol! { "V" => 0, "S" => 1 }]
        );
        // X ranges over V's dependents other than S
        assert_eq!(
            solutions(&tree, &format!("{base} OPTIONAL {{ X []; V -> X; }}")),
            [sol! { "V" => 0, "S" => 1, "X" => 2 }]
        );
    }

    #[test]
    fn test_tree_matches() {
        let tree = saw_tree();
        let check = |query: &str| {
            let pattern = compile_query(query).unwrap();
            let found = tree_matches(&tree, &pattern);
            assert_eq!(
                found,
                !find_all_bindings(&tree, &pattern).is_empty(),
                "{query}"
            );
            found
        };

        assert!(check(r#"MATCH { V [upos="VERB"]; }"#));
        assert!(!check(r#"MATCH { N [upos="NOUN"]; }"#));
        assert!(check(
            r#"MATCH { V [upos="VERB"]; } EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }"#
        ));
        assert!(!check(r#"MATCH { V [upos="VERB"]; } EXCEPT { V -> _; }"#));
    }

    #[test]
    fn test_search_tree() {
        // The public entry points wrap results in `Match`es sharing the tree
        let matches = search_tree_query(saw_tree(), r#"MATCH { V [upos="VERB"]; }"#).unwrap();
        assert_eq!(matches.len(), 2);
        assert!(Arc::ptr_eq(&matches[0].tree, &matches[1].tree));
        assert!(search_tree_query(saw_tree(), "MATCH { V [foo=\"x\"]; }").is_err());
    }
}
