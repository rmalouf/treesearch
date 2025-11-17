//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Solve CSP to find ALL matches (exhaustive search)
//! 3. Yield matches
//!
//! TODO: Implement CSP solver

use crate::RelationType;
use crate::pattern::{Constraint, Pattern};
use crate::query::parse_query;
use crate::tree::Word;
use crate::tree::{Tree, WordId};

/// Error during search
#[derive(Debug)]
pub enum SearchError {
    QueryError(crate::query::QueryError),
}

impl From<crate::query::QueryError> for SearchError {
    fn from(e: crate::query::QueryError) -> Self {
        SearchError::QueryError(e)
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::QueryError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for SearchError {}

/// A match is a binding from pattern variable IDs (VarId) to tree word IDs (WordId)
pub type Match = Vec<WordId>;

/// Check if a tree word satisfies a pattern variable's constraint
fn satisfies_var_constraint(tree: &Tree, word: &Word, constraint: &Constraint) -> bool {
    match constraint {
        Constraint::Lemma(lemma) => *tree.string_pool.resolve(word.lemma) == *lemma.as_bytes(),
        Constraint::POS(pos) => *tree.string_pool.resolve(word.upos) == *pos.as_bytes(),
        Constraint::Form(form) => *tree.string_pool.resolve(word.form) == *form.as_bytes(),
        Constraint::DepRel(deprel) => *tree.string_pool.resolve(word.deprel) == *deprel.as_bytes(),
        Constraint::And(constraints) => constraints
            .iter()
            .all(|constraint| satisfies_var_constraint(tree, word, constraint)),
        Constraint::Or(constraints) => constraints
            .iter()
            .any(|constraint| satisfies_var_constraint(tree, word, constraint)),
        Constraint::Any => true, // No filtering
    }
}

fn satisfies_arc_constraint(
    tree: &Tree,
    from_word_id: WordId,
    to_word_id: WordId,
    relation: &RelationType,
) -> bool {
    match relation {
        RelationType::Child => tree.check_rel(from_word_id, to_word_id),
        RelationType::Precedes => from_word_id < to_word_id,
        RelationType::Follows => to_word_id < from_word_id,
        _ => panic!("Unsupported relation"),
    }
}

/// Enumerate all matches
pub fn enumerate(tree: &Tree, pattern: &Pattern) -> Vec<Match> {
    // Initial candidate domains (node consistency)
    let mut domains: Vec<Vec<WordId>> = vec![Vec::new(); pattern.n_vars];
    for (var_id, var) in pattern.vars.iter().enumerate() {
        for (word_id, word) in tree.words.iter().enumerate() {
            if satisfies_var_constraint(tree, word, &var.constraints) {
                domains[var_id].push(word_id);
            }
        }
        if domains[var_id].is_empty() {
            return Vec::new(); // no solution possible
        }
    }

    // DFS with forward-checking
    let assign: Vec<Option<WordId>> = vec![None; pattern.n_vars];
    dfs(tree, pattern, &assign, &domains)
}

fn dfs(
    tree: &Tree,
    pattern: &Pattern,
    assign: &[Option<WordId>],
    domains: &[Vec<WordId>],
) -> Vec<Match> {
    // No more variables to assign!
    if assign.iter().all(|word_id| word_id.is_some()) {
        let solution = assign.iter().copied().flatten().collect();
        return vec![solution];
    }

    // Select an unassigned variable with Minimum Remaining Values (MRV)
    let next_var = (0..pattern.n_vars)
        .filter(|&var_id| assign[var_id].is_none())
        .min_by_key(|&var_id| domains[var_id].len())
        .expect("No MRV var found");

    let mut solutions: Vec<Match> = Vec::new();

    // Try each candidate word for this variable
    'candidates: for &word_id in &domains[next_var] {
        // AllDifferent: Check if word_id is already assigned to another variable
        if assign.contains(&Some(word_id)) {
            continue;
        };

        // Early prune: Check arc consistency with already-assigned neighbors
        if !check_arc_consistency(tree, pattern, assign, next_var, word_id) {
            continue;
        }

        let mut new_assign = assign.to_vec();
        let mut new_domains = domains.to_vec();

        // Assign var <- word_id and update domains
        new_assign[next_var] = Some(word_id);

        // AllDifferent: Remove word_id from all other unassigned variable domains
        for var_id in 0..pattern.n_vars {
            if var_id != next_var && new_assign[var_id].is_none() {
                new_domains[var_id].retain(|&w| w != word_id);
                if new_domains[var_id].is_empty() {
                    continue 'candidates;
                }
            }
        }

        // Forward-check: Propagate along edge constraints touching next_var
        if !forward_check(
            tree,
            pattern,
            next_var,
            word_id,
            &mut new_assign,
            &mut new_domains,
        ) {
            continue;
        }

        // Recurse
        solutions.extend(dfs(tree, pattern, &new_assign, &new_domains));
    }
    solutions
}

fn forward_check(
    tree: &Tree,
    pattern: &Pattern,
    next_var: usize,
    word_id: WordId,
    new_assign: &mut [Option<WordId>],
    new_domains: &mut [Vec<WordId>],
) -> bool {
    // Forward-check: Propagate along edge constraints touching next_var
    for &edge_idx in &pattern.out_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_idx];
        let target_var_id = pattern.var_names[&edge_constraint.to];
        if new_assign[target_var_id].is_some() {
            continue;
        }
        new_domains[target_var_id]
            .retain(|&w| satisfies_arc_constraint(tree, word_id, w, &edge_constraint.relation));
        if new_domains[target_var_id].is_empty() {
            return false;
        }
    }

    for &edge_idx in &pattern.in_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_idx];
        let source_var_id = pattern.var_names[&edge_constraint.from];
        if new_assign[source_var_id].is_some() {
            continue;
        }
        new_domains[source_var_id]
            .retain(|&w| satisfies_arc_constraint(tree, w, word_id, &edge_constraint.relation));
        if new_domains[source_var_id].is_empty() {
            return false;
        }
    }
    true
}

fn check_arc_consistency(
    tree: &Tree,
    pattern: &Pattern,
    assign: &[Option<WordId>],
    next_var: usize,
    word_id: WordId,
) -> bool {
    // Check arc consistency with already-assigned neighbors (early prune)
    for &edge_id in &pattern.out_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_id];
        let target_var_id = pattern.var_names[&edge_constraint.to];
        if assign[target_var_id].is_some_and(|target_word_id| {
            !satisfies_arc_constraint(tree, word_id, target_word_id, &edge_constraint.relation)
        }) {
            return false;
        }
    }
    for &edge_id in &pattern.in_edges[next_var] {
        let edge_constraint = &pattern.edge_constraints[edge_id];
        let source_var_id = pattern.var_names[&edge_constraint.from];
        if assign[source_var_id].is_some_and(|source_word_id| {
            !satisfies_arc_constraint(tree, source_word_id, word_id, &edge_constraint.relation)
        }) {
            return false;
        }
    }
    true
}

/// Search a tree with a pre-compiled pattern
///
/// Returns an iterator over all matches in the tree.
pub fn search(tree: &Tree, pattern: &Pattern) -> impl Iterator<Item = Match> {
    // Placeholder - will be reimplemented as CSP solver
    enumerate(tree, pattern).into_iter()
}

/// Search a tree with a query string
///
/// Parses the query and then searches the tree.
pub fn search_query<'a>(
    tree: &'a Tree,
    query: &str,
) -> Result<impl Iterator<Item = Match> + 'a, SearchError> {
    let pattern = parse_query(query)?;
    //Ok(search(tree, pattern))
    Ok(enumerate(tree, &pattern).into_iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"helped", b"help", b"VERB", None, b"root");
        tree.add_minimal_word(1, b"us", b"we", b"PRON", Some(0), b"obj");
        tree.add_minimal_word(2, b"to", b"to", b"PART", Some(3), b"mark");
        tree.add_minimal_word(3, b"win", b"win", b"VERB", Some(0), b"xcomp");
        tree.compile_tree();
        tree
    }

    /// Helper to build a coordination tree
    /// Structure: b"and" (root) -> b"cats" (conj)
    ///                         -> b"dogs" (conj)
    fn build_coord_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"and", b"and", b"CCONJ", None, b"root");
        tree.add_minimal_word(1, b"cats", b"cat", b"NOUN", Some(0), b"conj");
        tree.add_minimal_word(2, b"dogs", b"dog", b"NOUN", Some(0), b"conj");
        tree.compile_tree();
        tree
    }

    /// Helper to build a tree with multiple verbs
    /// b"saw" (root) -> b"John" (nsubj)
    ///              -> b"running" (xcomp) -> b"quickly" (advmod)
    fn build_multi_verb_tree() -> Tree {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"saw", b"see", b"VERB", None, b"root");
        tree.add_minimal_word(1, b"John", b"John", b"PROPN", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"running", b"run", b"VERB", Some(0), b"xcomp");
        tree.add_minimal_word(3, b"quickly", b"quickly", b"ADV", Some(2), b"advmod");
        tree.compile_tree();
        tree
    }

    #[test]
    fn test_search_query_single_var_lemma() {
        let tree = build_test_tree();
        let matches: Vec<_> = search_query(&tree, "V [lemma=\"help\"];")
            .unwrap()
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec![0]); // word 0 = "helped"
    }

    #[test]
    fn test_search_query_single_var_pos() {
        let tree = build_test_tree();
        let matches: Vec<_> = search_query(&tree, "V [pos=\"VERB\"];").unwrap().collect();
        // Should match both verbs: "helped" and "win"
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&vec![0])); // "helped"
        assert!(matches.contains(&vec![3])); // "win"
    }

    #[test]
    fn test_search_query_single_var_form() {
        let tree = build_test_tree();
        let matches: Vec<_> = search_query(&tree, "W [form=\"to\"];").unwrap().collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec![2]); // word 2 = "to"
    }

    #[test]
    fn test_search_query_single_var_deprel() {
        let tree = build_test_tree();
        let matches: Vec<_> = search_query(&tree, "X [deprel=\"xcomp\"];")
            .unwrap()
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec![3]); // word 3 = "win"
    }

    #[test]
    fn test_search_query_child_relation() {
        let tree = build_test_tree();
        // Find verb with obj child
        let matches: Vec<_> = search_query(&tree, "V [pos=\"VERB\"]; O []; V -[obj]-> O;")
            .unwrap()
            .collect();
        // Should match "helped -> us" (but also potentially "win -> us" if that edge existed, which it doesn't)
        // Only one match because only "helped" has an "obj" child
        assert!(matches.len() >= 1);
        assert!(matches.contains(&vec![0, 1])); // helped -> us
    }

    #[test]
    fn test_search_query_multiple_children() {
        let tree = build_coord_tree();
        // Find word with two conj children
        let matches: Vec<_> = search_query(
            &tree,
            "C [pos=\"CCONJ\"]; N1 []; N2 []; C -[conj]-> N1; C -[conj]-> N2;",
        )
        .unwrap()
        .collect();
        // Should find both permutations: (and, cats, dogs) and (and, dogs, cats)
        // Because CSP solver explores all valid assignments
        assert_eq!(
            matches.len(),
            2,
            "Expected 2 matches but got {}: {:?}",
            matches.len(),
            matches
        );
        assert!(matches.contains(&vec![0, 1, 2]), "Missing match [0, 1, 2]");
        assert!(matches.contains(&vec![0, 2, 1]), "Missing match [0, 2, 1]");
    }

    #[test]
    fn test_search_query_chain() {
        let tree = build_test_tree();
        // Find chain: helped -> win -> to
        let matches: Vec<_> = search_query(
            &tree,
            "V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; T [lemma=\"to\"]; V1 -> V2; V2 -> T;",
        )
        .unwrap()
        .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec![0, 3, 2]); // helped -> win -> to
    }

    #[test]
    fn test_search_query_no_matches() {
        let tree = build_test_tree();
        // Search for something that doesn't exist
        let matches: Vec<_> = search_query(&tree, "N [pos=\"NOUN\"];").unwrap().collect();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_search_query_constraint_and() {
        let tree = build_test_tree();
        // Find word with both lemma and pos constraints
        let matches: Vec<_> = search_query(&tree, "V [lemma=\"help\", pos=\"VERB\"];")
            .unwrap()
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec![0]); // "helped"
    }

    #[test]
    fn test_search_query_unconstrained_var() {
        let tree = build_test_tree();
        // Find any word
        let matches: Vec<_> = search_query(&tree, "X [];").unwrap().collect();
        assert_eq!(matches.len(), 4); // All 4 words in tree
    }

    #[test]
    fn test_search_query_parse_error() {
        let tree = build_test_tree();
        // Invalid query syntax
        let result = search_query(&tree, "V [invalid syntax");
        assert!(result.is_err());
        match result {
            Err(SearchError::QueryError(_)) => {} // Expected
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_search_query_exhaustive_matching() {
        let tree = build_coord_tree();
        // Find all nouns (exhaustive search should find both)
        let matches: Vec<_> = search_query(&tree, "N [pos=\"NOUN\"];").unwrap().collect();
        // Should find both "cats" and "dogs"
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&vec![1])); // cats
        assert!(matches.contains(&vec![2])); // dogs
    }

    #[test]
    fn test_search_query_complex_pattern() {
        let tree = build_multi_verb_tree();
        // Complex pattern: verb with nsubj and xcomp children
        let matches: Vec<_> = search_query(
            &tree,
            "V1 [pos=\"VERB\"]; S []; V2 [pos=\"VERB\"]; V1 -[nsubj]-> S; V1 -> V2;",
        )
        .unwrap()
        .collect();
        // Should match saw -> John + saw -> running
        // But there are 2 verbs, so we might get other combinations too
        // Let's verify we get at least the expected match
        assert!(matches.len() >= 1);
        assert!(matches.contains(&vec![0, 1, 2])); // saw -> John, saw -> running
    }

    #[test]
    fn test_search_query_forward_checking() {
        let tree = build_test_tree();
        // Pattern that should be pruned efficiently by forward-checking
        // Looking for "helped" with a child "win" that has a child "to"
        let matches: Vec<_> = search_query(
            &tree,
            "V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; T [lemma=\"to\"]; V1 -> V2; V2 -> T;",
        )
        .unwrap()
        .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec![0, 3, 2]);
    }

    #[test]
    fn test_search_empty_pattern() {
        let tree = build_test_tree();
        // Empty pattern has no variables, so returns one empty match
        let matches: Vec<_> = search_query(&tree, "").unwrap().collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec![]); // Empty assignment
    }
}
