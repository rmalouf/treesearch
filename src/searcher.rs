//! End-to-end tree search combining index lookup + VM execution
//!
//! The TreeSearcher provides the complete search pipeline:
//! 1. Parse query string into Pattern
//! 2. Compile Pattern into opcodes
//! 3. Build index from tree
//! 4. Use anchor element to get candidates from index
//! 5. Execute VM on each candidate
//! 6. Yield matches

use crate::compiler::compile_pattern;
use crate::index::TreeIndex;
use crate::parser::parse_query;
use crate::pattern::{Constraint, Pattern};
use crate::tree::{NodeId, Tree};
use crate::vm::{Match, VM};

/// Error during search
#[derive(Debug)]
pub enum SearchError {
    ParseError(crate::parser::ParseError),
}

impl From<crate::parser::ParseError> for SearchError {
    fn from(e: crate::parser::ParseError) -> Self {
        SearchError::ParseError(e)
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for SearchError {}

/// Tree searcher combining index lookup and VM execution
pub struct TreeSearcher {
    // Stateless - can be reused across searches
}

impl TreeSearcher {
    /// Create a new searcher
    pub fn new() -> Self {
        Self {}
    }

    /// Search a tree with a pre-compiled pattern
    ///
    /// Returns an iterator over all matches in the tree.
    pub fn search<'a>(
        &self,
        tree: &'a Tree,
        pattern: &Pattern,
    ) -> impl Iterator<Item = Match> + 'a {
        // Build index from tree
        let index = TreeIndex::build(tree);

        // Compile pattern to opcodes
        let (opcodes, anchor_idx, var_names) = compile_pattern(pattern.clone());

        // Get candidates from index based on anchor element
        let candidates = self.get_candidates(tree, pattern, anchor_idx, &index);

        // Execute VM on each candidate
        let vm = VM::new(opcodes, var_names);
        candidates
            .into_iter()
            .filter_map(move |node_id| vm.execute(tree, node_id))
    }

    /// Search a tree with a query string
    ///
    /// Parses the query and then searches the tree.
    pub fn search_query<'a>(
        &self,
        tree: &'a Tree,
        query: &str,
    ) -> Result<impl Iterator<Item = Match> + 'a, SearchError> {
        let pattern = parse_query(query)?;
        Ok(self.search(tree, &pattern))
    }

    /// Get candidate nodes from index based on anchor element
    fn get_candidates(
        &self,
        tree: &Tree,
        pattern: &Pattern,
        anchor_idx: usize,
        index: &TreeIndex,
    ) -> Vec<NodeId> {
        if pattern.elements.is_empty() {
            return Vec::new();
        }

        let anchor_element = &pattern.elements[anchor_idx];
        let constraint = &anchor_element.constraints;

        // Query index based on constraint type
        match Self::get_candidates_from_constraint(constraint, index) {
            Some(candidates) => candidates.to_vec(),
            None => {
                // No specific constraint - return all nodes
                (0..tree.nodes.len()).collect()
            }
        }
    }

    /// Get candidates from index based on constraint
    fn get_candidates_from_constraint<'a>(
        constraint: &Constraint,
        index: &'a TreeIndex,
    ) -> Option<&'a [NodeId]> {
        match constraint {
            Constraint::Lemma(lemma) => index.get_by_lemma(lemma),
            Constraint::POS(pos) => index.get_by_pos(pos),
            Constraint::Form(form) => index.get_by_form(form),
            Constraint::DepRel(deprel) => index.get_by_deprel(deprel),
            Constraint::And(constraints) => {
                // For And, use the most selective constraint
                // Try lemma/form first (most selective), then POS/deprel
                for c in constraints {
                    if let Some(candidates) = Self::get_candidates_from_constraint(c, index) {
                        return Some(candidates);
                    }
                }
                None
            }
            Constraint::Or(constraints) => {
                // For Or, we'd need to union all candidates
                // For now, just use the first constraint
                if let Some(c) = constraints.first() {
                    Self::get_candidates_from_constraint(c, index)
                } else {
                    None
                }
            }
            Constraint::Any => None, // No filtering
        }
    }
}

impl Default for TreeSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Node;

    /// Create a simple test tree:
    /// 0: runs (VERB, root)
    ///   ├─ 1: dog (NOUN, nsubj)
    ///   │    └─ 3: big (ADJ, amod)
    ///   └─ 2: quickly (ADV, advmod)
    fn create_test_tree() -> Tree {
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "quickly", "quickly", "ADV", "advmod"));
        tree.add_node(Node::new(3, "big", "big", "ADJ", "amod"));

        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 1);

        tree
    }

    #[test]
    fn test_search_simple_query() {
        let tree = create_test_tree();
        let searcher = TreeSearcher::new();

        // Query for VERB nodes
        let query = r#"V [pos="VERB"];"#;
        let matches: Vec<_> = searcher.search_query(&tree, query).unwrap().collect();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings[&0], 0); // "runs"
    }

    #[test]
    fn test_search_with_edge() {
        let tree = create_test_tree();
        let searcher = TreeSearcher::new();

        // Query for VERB with NOUN child
        let query = r#"
            V [pos="VERB"];
            N [pos="NOUN"];
            V -[nsubj]-> N;
        "#;
        let matches: Vec<_> = searcher.search_query(&tree, query).unwrap().collect();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings[&0], 0); // V = "runs"
        assert_eq!(matches[0].bindings[&1], 1); // N = "dog"
    }

    #[test]
    fn test_search_with_lemma() {
        let tree = create_test_tree();
        let searcher = TreeSearcher::new();

        // Query for specific lemma
        let query = r#"Dog [lemma="dog"];"#;
        let matches: Vec<_> = searcher.search_query(&tree, query).unwrap().collect();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings[&0], 1); // "dog"
    }

    #[test]
    fn test_search_no_matches() {
        let tree = create_test_tree();
        let searcher = TreeSearcher::new();

        // Query for something that doesn't exist
        let query = r#"X [pos="PRON"];"#;
        let matches: Vec<_> = searcher.search_query(&tree, query).unwrap().collect();

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_index_filtering() {
        let tree = create_test_tree();
        let searcher = TreeSearcher::new();
        let index = TreeIndex::build(&tree);

        // Get candidates for VERB
        let pattern = parse_query(r#"V [pos="VERB"];"#).unwrap();
        let candidates = searcher.get_candidates(&tree, &pattern, 0, &index);

        // Should only return the one VERB node
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], 0);
    }

    #[test]
    fn test_search_complex_pattern() {
        let tree = create_test_tree();
        let searcher = TreeSearcher::new();

        // VERB -> NOUN -> ADJ
        let query = r#"
            V [pos="VERB"];
            N [pos="NOUN"];
            A [pos="ADJ"];
            V -[nsubj]-> N;
            N -[amod]-> A;
        "#;
        let matches: Vec<_> = searcher.search_query(&tree, query).unwrap().collect();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].bindings[&0], 0); // V = "runs"
        assert_eq!(matches[0].bindings[&1], 1); // N = "dog"
        assert_eq!(matches[0].bindings[&2], 3); // A = "big"
    }
}
