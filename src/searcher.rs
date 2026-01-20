//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Solve CSP to find ALL matches (exhaustive search)
//! 3. Yield matches
//!

use crate::RelationType;
use crate::bytes::Sym;
use crate::pattern::{BasePattern, Constraint, ConstraintValue, EdgeConstraint, Pattern};
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
        //        Constraint::Or(constraints) => constraints
        //            .iter()
        //            .any(|constraint| satisfies_var_constraint(tree, word, constraint)),
        Constraint::Not(inner_constraint) => {
            !satisfies_var_constraint(tree, word, inner_constraint)
        }
        Constraint::Any => true, // No filtering
        Constraint::IsChild(label) => {
            if let Some(required_label) = label {
                word.head.is_some()
                    && tree
                        .string_pool
                        .compare_bytes(word.deprel, required_label.as_bytes())
            } else {
                word.head.is_some()
            }
        }
        Constraint::HasChild(label) => {
            if let Some(required_label) = label {
                !word.children_by_deprel(tree, required_label).is_empty()
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
                && edge_constraint
                    .label
                    .as_ref()
                    .is_none_or(|expected_deprel| {
                        let actual_deprel = tree.word(to_word_id).unwrap().deprel;
                        tree.string_pool
                            .compare_bytes(actual_deprel, expected_deprel.as_bytes())
                    })
        }
        RelationType::Precedes => from_word_id < to_word_id,
        RelationType::ImmediatelyPrecedes => to_word_id == from_word_id + 1,
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
    let mut assign: Vec<Option<WordId>> = vec![None; pattern.n_vars];
    let mut assigned_words: BitFixed<u64> = BitFixed::new(num_words);

    // Pre-assign from initial_bindings and validate constraints on pre-bound variables
    for (var_name, &word_id) in initial_bindings {
        if let Some(&var_id) = pattern.var_ids.get(var_name) {
            // Check that pre-bound variable satisfies its constraints in this pattern
            let word = &tree.words[word_id];
            let constr = &pattern.var_constraints[var_id];
            if !satisfies_var_constraint(tree, word, constr) {
                return Vec::new(); // Pre-bound variable fails constraint, no solutions possible
            }
            assign[var_id] = Some(word_id);
            assigned_words.set(word_id);
        }
    }

    // Initialize domains (node consistency)
    let mut domains: Vec<BitFixed<u64>> = vec![BitFixed::new(num_words); pattern.n_vars];
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

pub fn find_all_matches(tree: Tree, pattern: &Pattern) -> Vec<Match> {
    find_matches_impl(tree, pattern, false)
}

/// Check if a tree has at least one match
pub fn tree_matches(tree: &Tree, pattern: &Pattern) -> bool {
    !find_matches_impl(tree.clone(), pattern, true).is_empty()
}

fn find_matches_impl(tree: Tree, pattern: &Pattern, first_only: bool) -> Vec<Match> {
    let tree = Arc::new(tree);
    let empty_bindings = Bindings::new();
    let base_matches =
        solve_with_bindings(&tree, &pattern.match_pattern, &empty_bindings, first_only);

    let mut results = Vec::new();
    for base_bindings in base_matches {
        let rejected = pattern
            .except_patterns
            .iter()
            .any(|except| has_any_match(&tree, except, &base_bindings));

        if rejected {
            continue;
        }

        if first_only {
            // Skip optionals for existence check - just return first valid match
            results.push(Match {
                tree: Arc::clone(&tree),
                bindings: base_bindings,
            });
            return results;
        }

        let extended_solutions =
            process_optionals(&tree, base_bindings, &pattern.optional_patterns);

        for bindings in extended_solutions {
            results.push(Match {
                tree: Arc::clone(&tree),
                bindings,
            });
        }
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
    let next_var = (0..pattern.n_vars)
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
        //let mut new_domains = domains.to_vec();
        let new_domains = domains;

        // Assign var <- word_id and update bitset
        new_assign[next_var] = Some(word_id);
        let mut new_assigned_words = assigned_words.clone();
        new_assigned_words.set(word_id);

        // AllDifferent: Remove word_id from all other unassigned variable domains
        // for domain in &mut new_domains {
        //     domain.set(word_id, false);
        // }
        // if !(0..pattern.n_vars)
        //     .all(|var_id| new_assign[var_id].is_some() || new_domains[var_id].count_ones(..) > 0)
        // {
        //     continue;
        // }

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
            new_domains,
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
    for &edge_idx in &pattern.out_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_idx];
        let target_var_id = pattern.var_ids[&edge_constraint.to];
        if new_assign[target_var_id].is_some() {
            continue;
        }
        // Remove words from domain that don't satisfy the arc constraint
        for w in new_domains[target_var_id].iter().collect::<Vec<_>>() {
            if !satisfies_arc_constraint(tree, word_id, w, edge_constraint) {
                new_domains[target_var_id].reset(w);
            }
        }
        if new_domains[target_var_id].count_ones() == 0 {
            return false;
        }
    }

    for &edge_idx in &pattern.in_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_idx];
        let source_var_id = pattern.var_ids[&edge_constraint.from];
        if new_assign[source_var_id].is_some() {
            continue;
        }
        for w in new_domains[source_var_id].iter().collect::<Vec<_>>() {
            if !satisfies_arc_constraint(tree, w, word_id, edge_constraint) {
                new_domains[source_var_id].reset(w);
            }
        }
        if new_domains[source_var_id].count_ones() == 0 {
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
    for &edge_id in &pattern.out_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_id];
        let target_var_id = pattern.var_ids[&edge_constraint.to];
        if assign[target_var_id].is_some_and(|target_word_id| {
            !satisfies_arc_constraint(tree, word_id, target_word_id, edge_constraint)
        }) {
            return false;
        }
    }
    for &edge_id in &pattern.in_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_id];
        let source_var_id = pattern.var_ids[&edge_constraint.from];
        if assign[source_var_id].is_some_and(|source_word_id| {
            !satisfies_arc_constraint(tree, source_word_id, word_id, edge_constraint)
        }) {
            return false;
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

    macro_rules! hashmap {
        ( $( $key:expr => $val:expr ),* $(,)? ) => {{
            ::std::collections::HashMap::from([
                $( ($key.to_string(), $val), )*
            ])
        }};
    }

    fn build_test_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"helped", b"help", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"us", b"we", b"PRON", b"_", Some(0), b"obj");
        tree.add_minimal_word(2, b"to", b"to", b"PART", b"_", Some(3), b"mark");
        tree.add_minimal_word(3, b"win", b"win", b"VERB", b"_", Some(0), b"xcomp");
        tree.compile_tree();
        tree
    }

    /// Helper to build a coordination tree
    /// Structure: b"and" (root) -> b"cats" (conj)
    ///                         -> b"dogs" (conj)
    fn build_coord_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"and", b"and", b"CCONJ", b"_", None, b"root");
        tree.add_minimal_word(1, b"cats", b"cat", b"NOUN", b"_", Some(0), b"conj");
        tree.add_minimal_word(2, b"dogs", b"dog", b"NOUN", b"_", Some(0), b"conj");
        tree.compile_tree();
        tree
    }

    /// Helper to build a tree with multiple verbs
    /// b"saw" (root) -> b"John" (nsubj)
    ///              -> b"running" (xcomp) -> b"quickly" (advmod)
    fn build_multi_verb_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"saw", b"see", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"John", b"John", b"PROPN", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"running", b"run", b"VERB", b"_", Some(0), b"xcomp");
        tree.add_minimal_word(3, b"quickly", b"quickly", b"ADV", b"_", Some(2), b"advmod");
        tree.compile_tree();
        tree
    }

    #[test]
    fn test_search_single_var_constraints() {
        let tree = build_test_tree();

        // Test lemma constraint - should find one match
        let matches: Vec<_> =
            search_tree_query(tree.clone(), "MATCH { V [lemma=\"help\"]; }").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 });

        // Test upos constraint - should match both verbs
        let matches: Vec<_> =
            search_tree_query(tree.clone(), "MATCH { V [upos=\"VERB\"]; }").unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 });
        assert_eq!(matches[1].bindings, hashmap! { "V" => 3 });

        // Test form constraint
        let matches: Vec<_> =
            search_tree_query(tree.clone(), "MATCH { W [form=\"to\"]; }").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "W" => 2 });

        // Test deprel constraint
        let matches: Vec<_> =
            search_tree_query(tree.clone(), "MATCH { X [deprel=\"xcomp\"]; }").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "X" => 3});
    }

    #[test]
    fn test_search_tree_query_multiple_children() {
        let tree = build_coord_tree();
        // Find word with two conj children
        let matches: Vec<_> = search_tree_query(
            tree,
            "MATCH { C [upos=\"CCONJ\"]; N1 []; N2 []; C -[conj]-> N1; C -[conj]-> N2; }",
        )
        .unwrap();
        // Should find both permutations: (and, cats, dogs) and (and, dogs, cats)
        // Because CSP solver explores all valid assignments
        assert_eq!(
            matches.len(),
            2,
            "Expected 2 matches but got {}: {:?}",
            matches.len(),
            matches
        );
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "C" => 0, "N1" => 1, "N2" => 2 }),
            "Missing match [0, 1, 2]"
        );
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "C" => 0, "N1" => 2, "N2" => 1 }),
            "Missing match [0, 2, 1]"
        );
    }

    #[test]
    fn test_search_tree_query_chain() {
        let tree = build_test_tree();
        // Find chain: helped -> win -> to (tests forward-checking efficiency)
        let matches: Vec<_> = search_tree_query(
            tree,
            "MATCH { V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; T [lemma=\"to\"]; V1 -> V2; V2 -> T; }",
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].bindings,
            hashmap! { "V1" => 0, "V2" => 3, "T" => 2 }
        );
    }

    #[test]
    fn test_search_tree_query_basic_constraints() {
        let tree = build_test_tree();

        // No matches - word doesn't exist
        let matches: Vec<_> =
            search_tree_query(tree.clone(), "MATCH { N [upos=\"NOUN\"]; }").unwrap();
        assert_eq!(matches.len(), 0);

        // Multiple constraints (AND)
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            "MATCH { V [lemma=\"help\" & upos=\"VERB\"]; }",
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 });

        // Unconstrained variable - matches all words
        let matches: Vec<_> = search_tree_query(tree.clone(), "MATCH { X []; }").unwrap();
        assert_eq!(matches.len(), 4);
    }

    #[test]
    fn test_search_tree_query_exhaustive_matching() {
        let tree = build_coord_tree();
        // Find all nouns (exhaustive search should find both)
        let matches: Vec<_> = search_tree_query(tree, "MATCH { N [upos=\"NOUN\"]; }").unwrap();
        // Should find both "cats" and "dogs"
        assert_eq!(matches.len(), 2);
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "N" => 1 })
        ); // cats
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "N" => 2 })
        ); // dogs
    }

    #[test]
    fn test_search_tree_query_complex_pattern() {
        let tree = build_multi_verb_tree();
        // Complex pattern: verb with nsubj and xcomp children
        let matches: Vec<_> = search_tree_query(
            tree,
            "MATCH { V1 [upos=\"VERB\"]; S []; V2 [upos=\"VERB\"]; V1 -[nsubj]-> S; V1 -> V2; }",
        )
        .unwrap();
        // Should match saw -> John + saw -> running
        assert!(matches.len() >= 1);
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "V1" => 0, "S" => 1, "V2" => 2 })
        );
    }

    #[test]
    fn test_search_empty_pattern() {
        let tree = build_test_tree();
        // Empty pattern has no variables, so returns one empty match
        let matches: Vec<_> = search_tree_query(tree, "MATCH { }").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! {});
    }

    #[test]
    fn test_precedence_operators() {
        // Tree: "helped" (0) "us" (1) "to" (2) "win" (3)
        let tree = build_test_tree();

        // Precedes (<<): "helped" << "win" should match (non-adjacent OK)
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            "MATCH { V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; V1 << V2; }",
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V1" => 0, "V2" => 3 });

        // Precedes: wrong order should fail
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            "MATCH { V1 [lemma=\"win\"]; V2 [lemma=\"help\"]; V1 << V2; }",
        )
        .unwrap();
        assert_eq!(matches.len(), 0);

        // Immediately precedes (<): "to" < "win" should match (adjacent)
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            "MATCH { T [lemma=\"to\"]; V [lemma=\"win\"]; T < V; }",
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "T" => 2, "V" => 3 });

        // Immediately precedes: "helped" < "win" should NOT match (not adjacent)
        let matches: Vec<_> = search_tree_query(
            tree,
            "MATCH { V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; V1 < V2; }",
        )
        .unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_mixed_dependency_and_precedence() {
        // Test combining dependency edges with precedence constraints
        // Tree: "helped" (0) "us" (1) "to" (2) "win" (3)
        //       helped -> us (obj), helped -> win (xcomp), win -> to (mark)
        let tree = build_test_tree();

        // Find: helped -[xcomp]-> win, AND helped << win (in word order)
        let matches: Vec<_> = search_tree_query(
            tree,
            "MATCH { V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; V1 -[xcomp]-> V2; V1 << V2; }",
        )
        .unwrap();

        // Should match because both constraints are satisfied
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V1" => 0, "V2" => 3 });
    }

    #[test]
    fn test_precedence_blocks_dependency_match() {
        // Negative test: precedence constraint blocks a valid dependency match
        // Tree: "helped" (0) "us" (1) "to" (2) "win" (3)
        //       helped -> win (xcomp)
        let tree = build_test_tree();

        // Without precedence, dependency edge matches
        let matches_no_precedence: Vec<_> =
            search_tree_query(tree.clone(), "MATCH { V1 []; V2 []; V1 -[xcomp]-> V2; }").unwrap();
        assert_eq!(matches_no_precedence.len(), 1);

        // But if we add a false precedence constraint (win << helped),
        // the match should fail even though the dependency exists
        let matches_with_false_precedence: Vec<_> = search_tree_query(
            tree.clone(),
            "MATCH { V1 []; V2 []; V1 -[xcomp]-> V2; V2 << V1; }",
        )
        .unwrap();

        assert_eq!(
            matches_with_false_precedence.len(),
            0,
            "Expected no matches because V2 (win=3) cannot precede V1 (helped=0)"
        );
    }

    #[test]
    fn test_precedence_with_coord_tree() {
        // Test precedence constraints on coordination tree
        // Tree: "and" (0) "cats" (1) "dogs" (2)
        let tree = build_coord_tree();

        // "and" << "cats" should match (0 precedes 1)
        let matches: Vec<_> = search_tree_query(
            tree,
            "MATCH { C [lemma=\"and\"]; N [lemma=\"cat\"]; C << N; }",
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "C" => 0, "N" => 1 });
    }

    #[test]
    fn test_precedence_chain() {
        // Test chained precedence: A << B << C
        // Tree: "helped" (0) "us" (1) "to" (2) "win" (3)
        let tree = build_test_tree();

        // "helped" << "us" << "to" should match
        let matches: Vec<_> = search_tree_query(
            tree,
            "MATCH { A [lemma=\"help\"]; B [lemma=\"we\"]; C [lemma=\"to\"]; A << B; B << C; }",
        )
        .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].bindings,
            hashmap! { "A" => 0, "B" => 1, "C" => 2 }
        );
    }

    /// Helper to build a tree with morphological features
    fn build_feature_tree() -> Tree {
        use crate::tree::Features;
        let mut tree = Tree::default();

        // Word 0: "was" - lemma=be, Tense=Past, Number=Sing
        let mut feats_was = Features::new();
        feats_was.push((
            tree.string_pool.get_or_intern(b"Tense"),
            tree.string_pool.get_or_intern(b"Past"),
        ));
        feats_was.push((
            tree.string_pool.get_or_intern(b"Number"),
            tree.string_pool.get_or_intern(b"Sing"),
        ));
        let mut misc_was = Features::new();
        misc_was.push((
            tree.string_pool.get_or_intern(b"SpaceAfter"),
            tree.string_pool.get_or_intern(b"No"),
        ));
        tree.add_word(
            0, 1, b"was", b"be", b"VERB", b"_", feats_was, None, b"root", misc_was,
        );

        // Word 1: "running" - Tense=Pres, VerbForm=Part
        let mut feats_run = Features::new();
        feats_run.push((
            tree.string_pool.get_or_intern(b"Tense"),
            tree.string_pool.get_or_intern(b"Pres"),
        ));
        feats_run.push((
            tree.string_pool.get_or_intern(b"VerbForm"),
            tree.string_pool.get_or_intern(b"Part"),
        ));
        tree.add_word(
            1,
            2,
            b"running",
            b"run",
            b"VERB",
            b"_",
            feats_run,
            Some(0),
            b"xcomp",
            Features::new(),
        );

        // Word 2: "," - no features
        tree.add_word(
            2,
            3,
            b",",
            b",",
            b"PUNCT",
            b"_",
            Features::new(),
            Some(0),
            b"punct",
            Features::new(),
        );

        tree.compile_tree();
        tree
    }

    #[test]
    fn test_feature_constraints() {
        let tree = build_feature_tree();

        // Single feature constraint
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { V [feats.Tense="Past"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 }); // "was"

        // Multiple feature constraints (AND)
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            r#"MATCH { V [feats.Tense="Past" & feats.Number="Sing"]; }"#,
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 }); // "was"

        // Feature combined with other constraints
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            r#"MATCH { V [lemma="be" & feats.Tense="Past"]; }"#,
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 });

        // Non-existent feature value
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { V [feats.Tense="Fut"]; }"#).unwrap();
        assert_eq!(matches.len(), 0); // No future tense verbs

        // Word with no features
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            r#"MATCH { P [upos="PUNCT" & feats.Tense="Past"]; }"#,
        )
        .unwrap();
        assert_eq!(matches.len(), 0); // PUNCT has no Tense feature
    }

    #[test]
    fn test_misc_constraints() {
        let tree = build_feature_tree();

        // Single misc constraint
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { V [misc.SpaceAfter="No"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 }); // "was"

        // Non-existent misc value
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { V [misc.SpaceAfter="Yes"]; }"#).unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_feature_case_sensitive() {
        let tree = build_feature_tree();

        // Correct case
        let matches =
            search_tree_query(tree.clone(), r#"MATCH { V [feats.Tense="Past"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);

        // Wrong key case
        let matches =
            search_tree_query(tree.clone(), r#"MATCH { V [feats.tense="Past"]; }"#).unwrap();
        assert_eq!(matches.len(), 0);

        // Wrong value case
        let matches =
            search_tree_query(tree.clone(), r#"MATCH { V [feats.Tense="past"]; }"#).unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_negative_constraint() {
        // Tree: "helped" (0) "us" (1) "to" (2) "win" (3)
        let tree = build_test_tree();

        // Find all words that are NOT VERBs
        let matches: Vec<_> = search_tree_query(tree, r#"MATCH { W [upos!="VERB"]; }"#).unwrap();
        assert_eq!(matches.len(), 2); // us (PRON), to (PART)
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "W" => 1 })
        );
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "W" => 2 })
        );
    }

    #[test]
    fn test_negative_feature_constraint() {
        let tree = build_feature_tree();

        // Find all verbs that are NOT past tense
        let matches: Vec<_> =
            search_tree_query(tree, r#"MATCH { V [upos="VERB" & feats.Tense!="Past"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 1 }); // "running" has Tense=Pres
    }

    #[test]
    fn test_negative_unlabeled_edge() {
        // Tree: "helped" (0) -> "us" (1, obj), "win" (3, xcomp) -> "to" (2, mark)
        let tree = build_test_tree();

        // Find pairs where V does NOT have an edge to T
        // "helped" has edges to "us" and "win", but not "to"
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; T [lemma="to"]; V !-> T; }"#,
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0, "T" => 2 }); // helped !-> to
    }

    #[test]
    fn test_negative_labeled_edge() {
        // Tree: "helped" (0) -> "us" (1, obj), "win" (3, xcomp)
        let tree = build_test_tree();

        // Find verb V and word W where V does NOT have obj edge to W
        // "helped" has obj to "us" (1), so pairs with W=1 should be excluded
        // Also, AllDifferent constraint means V != W
        let matches: Vec<_> =
            search_tree_query(tree, r#"MATCH { V [lemma="help"]; W []; V !-[obj]-> W; }"#).unwrap();

        // Should match V=0 with W=2, W=3 (not W=1 which is obj, not W=0 due to AllDifferent)
        assert_eq!(matches.len(), 2);
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "V" => 0, "W" => 2 })
        );
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "V" => 0, "W" => 3 })
        );
        assert!(
            !matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "V" => 0, "W" => 1 })
        ); // Excluded: obj edge exists
    }

    #[test]
    fn test_mixed_positive_and_negative_edges() {
        // Tree: "helped" (0) -> "us" (1, obj), "win" (3, xcomp)
        let tree = build_test_tree();

        // Find: V has xcomp to Y, but NOT obj to W
        // AllDifferent means V, Y, W must all be different
        let matches: Vec<_> = search_tree_query(
            tree,
            r#"MATCH { V []; Y []; W []; V -[xcomp]-> Y; V !-[obj]-> W; }"#,
        )
        .unwrap();

        // V=0, Y=3 (helped -[xcomp]-> win)
        // W can only be 2 (not 0=V, not 3=Y, not 1 which is obj of helped)
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].bindings,
            hashmap! { "V" => 0, "Y" => 3, "W" => 2 }
        );
    }

    #[test]
    fn test_negative_edge_with_anonymous_var() {
        // Tree: "helped" (0) -> "us" (1, obj), "win" (3, xcomp)
        let tree = build_test_tree();

        // Find words that do NOT have any incoming edges (i.e., root words)
        let matches: Vec<_> = search_tree_query(tree, r#"MATCH { W []; _ !-> W; }"#).unwrap();

        // Only word 0 (helped) has no incoming edge (it's the root)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "W" => 0 });
    }

    #[test]
    fn test_negative_labeled_edge_with_anonymous_var() {
        // Tree: "helped" (0) -> "us" (1, obj), "win" (3, xcomp) -> "to" (2, mark)
        let tree = build_test_tree();

        // Find words that are NOT anyone's obj (i.e., deprel != "obj")
        let matches: Vec<_> = search_tree_query(tree, r#"MATCH { W []; _ !-[obj]-> W; }"#).unwrap();

        // Words 0 (root), 2 (mark), 3 (xcomp) are not obj of anyone
        assert_eq!(matches.len(), 3);
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "W" => 0 })
        ); // root
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "W" => 2 })
        ); // mark
        assert!(
            matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "W" => 3 })
        ); // xcomp
        assert!(
            !matches
                .iter()
                .map(|m| m.bindings.clone())
                .collect::<Vec<Bindings>>()
                .contains(&hashmap! { "W" => 1 })
        ); // us is obj
    }

    #[test]
    fn test_negative_edge_no_deprel_constraint() {
        // Verify that negative labeled edges don't add DepRel constraint
        let _tree = build_test_tree();

        // Parse pattern with negative labeled edge
        let pattern = compile_query(r#"MATCH { V []; W []; V !-[obj]-> W; }"#).unwrap();

        // Check that W does not have a DepRel constraint
        let w_id = *pattern.match_pattern.var_ids.get("W").unwrap();
        match &pattern.match_pattern.var_constraints[w_id] {
            Constraint::Any => { /* Expected - no constraint */ }
            Constraint::And(constraints) => {
                // Should not contain DepRel constraint
                assert!(
                    !constraints
                        .iter()
                        .any(|c| matches!(c, Constraint::DepRel(_))),
                    "Negative edge should not add DepRel constraint"
                );
            }
            other => panic!("Unexpected constraint on W: {:?}", other),
        }
    }

    #[test]
    fn test_except_blocks() {
        // Tree: saw (VERB) -> John (nsubj), running (xcomp) -> quickly (advmod)
        let tree = build_multi_verb_tree();

        // Test 1: EXCEPT rejects when condition matches
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }"#,
        )
        .unwrap();
        // Should find word 0 ("saw") but not word 2 ("running" with advmod)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 });

        // Test 2: Multiple EXCEPT blocks (ANY semantics)
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }
               EXCEPT { C [upos="VERB"]; V -[xcomp]-> C; }"#,
        )
        .unwrap();
        // Both verbs rejected: saw has xcomp, running has advmod
        assert_eq!(matches.len(), 0);

        // Test 3: EXCEPT with shared MATCH variable
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; S [upos="PROPN"]; V -[nsubj]-> S; }
               EXCEPT { C [upos="VERB"]; V -[xcomp]-> C; }"#,
        )
        .unwrap();
        // saw-John pair rejected because saw has xcomp
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_optional_blocks() {
        // Tree: saw -> John (nsubj), running (xcomp) -> quickly (advmod)
        let tree = build_multi_verb_tree();

        // Test 1: OPTIONAL found - variable present in bindings
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [lemma="see"]; }
               OPTIONAL { S [upos="PROPN"]; V -[nsubj]-> S; }"#,
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0, "S" => 1 });

        // Test 2: OPTIONAL not found - variable absent from bindings
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [lemma="run"]; }
               OPTIONAL { S [upos="PROPN"]; V -[nsubj]-> S; }"#,
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 2 });
        assert!(!matches[0].bindings.contains_key("S"));

        // Test 3: Multiple OPTIONAL blocks - cross-product semantics
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [lemma="see"]; }
               OPTIONAL { S [upos="PROPN"]; V -[nsubj]-> S; }
               OPTIONAL { C [upos="VERB"]; V -[xcomp]-> C; }"#,
        )
        .unwrap();
        // Both OPTIONAL blocks match, so we get the cross-product (1 result with both)
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].bindings,
            hashmap! { "V" => 0, "S" => 1, "C" => 2 }
        );
    }

    #[test]
    fn test_combined_except_optional() {
        let tree = build_multi_verb_tree();

        // Find all verbs, exclude those with advmod, optionally capture subject
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }
               OPTIONAL { S [upos="PROPN"]; V -[nsubj]-> S; }"#,
        )
        .unwrap();
        // Should find only word 0 ("saw"), with subject
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0, "S" => 1 });
    }

    #[test]
    fn test_except_no_rejection() {
        let tree = build_multi_verb_tree();

        // EXCEPT condition that doesn't match anything - all results should pass through
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT { N [upos="NOUN"]; V -[obj]-> N; }"#,
        )
        .unwrap();
        // Both verbs should be found (no NOUNs in tree)
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|m| m.bindings == hashmap! { "V" => 0 }));
        assert!(matches.iter().any(|m| m.bindings == hashmap! { "V" => 2 }));
    }

    #[test]
    fn test_except_all_rejected() {
        let tree = build_multi_verb_tree();

        // EXCEPT condition that matches all MATCH results
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT { V [upos="VERB"]; }"#,
        )
        .unwrap();
        // All verbs should be rejected
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_except_complex_pattern() {
        // Tree: saw -> John (nsubj), running (xcomp) -> quickly (advmod)
        let tree = build_multi_verb_tree();

        // EXCEPT with multiple constraints and edges
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT {
                   C [upos="VERB"];
                   M [upos="ADV"];
                   V -[xcomp]-> C;
                   C -[advmod]-> M;
               }"#,
        )
        .unwrap();
        // Only "running" should be found (not "saw", which has xcomp to running which has advmod)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "V" => 2 });
    }

    #[test]
    fn test_optional_multiple_matches() {
        // Create a tree with multiple possible matches for OPTIONAL
        // Tree: helped -> He (nsubj), us (obj), win (xcomp)
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"helped", b"help", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"He", b"he", b"PRON", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"us", b"we", b"PRON", b"_", Some(0), b"obj");
        tree.add_minimal_word(3, b"win", b"win", b"VERB", b"_", Some(0), b"xcomp");
        tree.compile_tree();

        // Match verb, optionally match any PRON child
        let matches = search_tree_query(
            tree,
            r#"MATCH { V [lemma="help"]; }
               OPTIONAL { P [upos="PRON"]; V -> P; }"#,
        )
        .unwrap();

        // Should get 2 results: one with P=He, one with P=us
        assert_eq!(matches.len(), 2);
        assert!(
            matches
                .iter()
                .any(|m| m.bindings == hashmap! { "V" => 0, "P" => 1 })
        );
        assert!(
            matches
                .iter()
                .any(|m| m.bindings == hashmap! { "V" => 0, "P" => 2 })
        );
    }

    #[test]
    fn test_optional_cross_product_multiple_matches() {
        // Create a tree where multiple OPTIONALs each have multiple matches
        // Tree: helped -> He (nsubj), us (obj), quickly (advmod), very (advmod)
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"helped", b"help", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"He", b"he", b"PRON", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"us", b"we", b"PRON", b"_", Some(0), b"obj");
        tree.add_minimal_word(3, b"quickly", b"quickly", b"ADV", b"_", Some(0), b"advmod");
        tree.add_minimal_word(4, b"very", b"very", b"ADV", b"_", Some(0), b"advmod");
        tree.compile_tree();

        // Match verb, optionally match PRON and ADV children
        let matches = search_tree_query(
            tree,
            r#"MATCH { V [lemma="help"]; }
               OPTIONAL { P [upos="PRON"]; V -> P; }
               OPTIONAL { A [upos="ADV"]; V -> A; }"#,
        )
        .unwrap();

        // Cross-product: 2 PRONs × 2 ADVs = 4 results
        assert_eq!(matches.len(), 4);

        // Verify all combinations exist
        assert!(
            matches
                .iter()
                .any(|m| m.bindings == hashmap! { "V" => 0, "P" => 1, "A" => 3 })
        );
        assert!(
            matches
                .iter()
                .any(|m| m.bindings == hashmap! { "V" => 0, "P" => 1, "A" => 4 })
        );
        assert!(
            matches
                .iter()
                .any(|m| m.bindings == hashmap! { "V" => 0, "P" => 2, "A" => 3 })
        );
        assert!(
            matches
                .iter()
                .any(|m| m.bindings == hashmap! { "V" => 0, "P" => 2, "A" => 4 })
        );
    }

    #[test]
    fn test_optional_one_matches_one_doesnt() {
        // Test where one OPTIONAL matches and another doesn't
        let tree = build_multi_verb_tree();

        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [lemma="see"]; }
               OPTIONAL { S [upos="PROPN"]; V -[nsubj]-> S; }
               OPTIONAL { N [upos="NOUN"]; V -[obj]-> N; }"#,
        )
        .unwrap();

        // First OPTIONAL matches (John), second doesn't (no NOUN obj)
        // Should get 1 result with S but no N
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings.get("V"), Some(&0));
        assert_eq!(matches[0].bindings.get("S"), Some(&1));
        assert!(!matches[0].bindings.contains_key("N"));
    }

    #[test]
    fn test_complex_except_optional_interaction() {
        // Create a more complex tree for testing
        // Tree: helped -> He (nsubj), them (obj), win (xcomp) -> quickly (advmod)
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"helped", b"help", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"He", b"he", b"PRON", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"them", b"they", b"PRON", b"_", Some(0), b"obj");
        tree.add_minimal_word(3, b"win", b"win", b"VERB", b"_", Some(0), b"xcomp");
        tree.add_minimal_word(4, b"quickly", b"quickly", b"ADV", b"_", Some(3), b"advmod");
        tree.compile_tree();

        // Find all verbs, except those with advmod, optionally get subject and object
        let matches = search_tree_query(
            tree,
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT { A [upos="ADV"]; V -[advmod]-> A; }
               OPTIONAL { S [upos="PRON"]; V -[nsubj]-> S; }
               OPTIONAL { O [upos="PRON"]; V -[obj]-> O; }"#,
        )
        .unwrap();

        // Should find only "helped" (not "win" which has advmod)
        // With both subject and object
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].bindings,
            hashmap! { "V" => 0, "S" => 1, "O" => 2 }
        );
    }

    #[test]
    fn test_except_with_anonymous_edge() {
        // Tree: helped -> us (obj), win (xcomp)
        //       win -> to (mark)
        let tree = build_test_tree();

        // Find verbs, EXCEPT those that have a mark child
        // "win" has mark child "to", should be excluded
        // "helped" has no mark child, should be found
        let matches = search_tree_query(
            tree.clone(),
            r#"MATCH { V [upos="VERB"]; }
               EXCEPT { V -[mark]-> _; }"#,
        )
        .unwrap();

        println!(
            "Matches found: {:?}",
            matches.iter().map(|m| &m.bindings).collect::<Vec<_>>()
        );

        // Should find only "helped" (word 0), not "win" (word 3)
        assert_eq!(
            matches.len(),
            1,
            "Expected 1 match but got {:?}",
            matches.iter().map(|m| &m.bindings).collect::<Vec<_>>()
        );
        assert_eq!(matches[0].bindings, hashmap! { "V" => 0 });
    }

    #[test]
    fn test_tree_matches() {
        // Multiple matches - tree_matches returns true
        let tree = build_coord_tree();
        let pattern = compile_query("MATCH { N [upos=\"NOUN\"]; }").unwrap();
        assert_eq!(find_all_matches(tree.clone(), &pattern).len(), 2);
        assert!(tree_matches(&tree, &pattern));

        // No matches - tree_matches returns false
        let tree = build_test_tree();
        assert!(!tree_matches(&tree, &pattern));

        // With EXCEPT block - still works correctly
        let tree = build_multi_verb_tree();
        let pattern = compile_query(
            r#"MATCH { V [upos="VERB"]; } EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }"#,
        )
        .unwrap();
        assert!(tree_matches(&tree, &pattern));
    }

    /// Helper to build a tree with xpos values
    fn build_xpos_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"The", b"the", b"DET", b"DT", Some(1), b"det");
        tree.add_minimal_word(1, b"dog", b"dog", b"NOUN", b"NN", Some(2), b"nsubj");
        tree.add_minimal_word(2, b"runs", b"run", b"VERB", b"VBZ", None, b"root");
        tree.compile_tree();
        tree
    }

    #[test]
    fn test_xpos_constraint() {
        let tree = build_xpos_tree();

        // Single xpos constraint
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [xpos="DT"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "W" => 0 }); // "The"

        // xpos combined with upos
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [upos="VERB" & xpos="VBZ"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "W" => 2 }); // "runs"

        // xpos with no match
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [xpos="VBD"]; }"#).unwrap();
        assert_eq!(matches.len(), 0);

        // Negative xpos constraint
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [upos="NOUN" & xpos!="NNS"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings, hashmap! { "W" => 1 }); // "dog" has xpos=NN
    }

    #[test]
    fn test_single_word_tree() {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"word", b"word", b"NOUN", b"_", None, b"root");
        tree.compile_tree();

        // Should find the single word
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [upos="NOUN"]; }"#).unwrap();
        assert_eq!(matches.len(), 1);

        // Edge constraint should find nothing (no children)
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { P []; C []; P -> C; }"#).unwrap();
        assert_eq!(matches.len(), 0);

        // Precedence on single word - nothing precedes itself
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { A []; B []; A << B; }"#).unwrap();
        assert_eq!(matches.len(), 0); // AllDifferent means A != B, so no pairs
    }

    #[test]
    fn test_regex_constraints() {
        let tree = build_test_tree();
        // Tree has: "helped" (lemma: help), "us" (lemma: we), "to" (lemma: to), "win" (lemma: win)

        // Basic regex - match lemmas starting with "w" (anchors added automatically)
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [lemma=/w.*/]; }"#).unwrap();
        assert_eq!(matches.len(), 2); // "we" and "win"
        let word_ids: Vec<_> = matches.iter().map(|m| m.bindings["W"]).collect();
        assert!(word_ids.contains(&1)); // "us" (lemma: we)
        assert!(word_ids.contains(&3)); // "win"

        // Regex with alternation - match exactly VERB or PRON
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [upos=/VERB|PRON/]; }"#).unwrap();
        assert_eq!(matches.len(), 3); // "helped", "us", "win"
        let word_ids: Vec<_> = matches.iter().map(|m| m.bindings["W"]).collect();
        assert!(word_ids.contains(&0)); // helped (VERB)
        assert!(word_ids.contains(&1)); // us (PRON)
        assert!(word_ids.contains(&3)); // win (VERB)

        // Regex with .* - match words containing "el"
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [form=/.*el.*/]; }"#).unwrap();
        assert_eq!(matches.len(), 1); // "helped"
        assert_eq!(matches[0].bindings["W"], 0);

        // Negated regex - match lemmas NOT starting with "w"
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { W [lemma!=/w.*/]; }"#).unwrap();
        assert_eq!(matches.len(), 2); // "help" and "to"
        let word_ids: Vec<_> = matches.iter().map(|m| m.bindings["W"]).collect();
        assert!(word_ids.contains(&0)); // helped (lemma: help)
        assert!(word_ids.contains(&2)); // to

        // Mixed literal and regex
        let matches: Vec<_> =
            search_tree_query(tree.clone(), r#"MATCH { V [upos="VERB" & lemma=/h.*/]; }"#).unwrap();
        assert_eq!(matches.len(), 1); // "helped"
        assert_eq!(matches[0].bindings["V"], 0);

        // Regex in edge constraints - this tests that deprel constraints work with literals
        // (regex in edge labels would require grammar changes)
        let matches: Vec<_> = search_tree_query(
            tree.clone(),
            r#"MATCH { V []; O [lemma=/w.*/]; V -[obj]-> O; }"#,
        )
        .unwrap();
        assert_eq!(matches.len(), 1); // helped -> us (lemma: we)
        assert_eq!(matches[0].bindings["V"], 0);
        assert_eq!(matches[0].bindings["O"], 1);
    }
}
