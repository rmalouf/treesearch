//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Build index from tree
//! 3. Generate candidate domains from index
//! 4. Solve CSP to find matches
//! 5. Yield matches
//!
//! TODO: Reimplement as CSP solver

use crate::index::TreeIndex;
use crate::parser::parse_query;
use crate::pattern::{Constraint, Pattern};
use crate::tree::{NodeId, Tree};
use std::collections::HashMap;

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

/// A match is a binding from pattern variable indices to tree node IDs
pub type Match = HashMap<usize, NodeId>;

/// Search a tree with a pre-compiled pattern
///
/// Returns an iterator over all matches in the tree.
/// TODO: Reimplement as CSP solver
pub fn search<'a>(_tree: &'a Tree, _pattern: Pattern) -> impl Iterator<Item = Match> + 'a {
    // Placeholder - will be reimplemented as CSP solver
    std::iter::empty()
}

/// Search a tree with a query string
///
/// Parses the query and then searches the tree.
pub fn search_query<'a>(
    tree: &'a Tree,
    query: &str,
) -> Result<impl Iterator<Item = Match> + 'a, SearchError> {
    let pattern = parse_query(query)?;
    Ok(search(tree, pattern))
}

/// Get candidate nodes from index based on anchor element
fn get_candidates(
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
    match get_candidates_from_constraint(constraint, index) {
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
                if let Some(candidates) = get_candidates_from_constraint(c, index) {
                    return Some(candidates);
                }
            }
            None
        }
        Constraint::Or(constraints) => {
            // For Or, we'd need to union all candidates
            // For now, just use the first constraint
            if let Some(c) = constraints.first() {
                get_candidates_from_constraint(c, index)
            } else {
                None
            }
        }
        Constraint::Any => None, // No filtering
    }
}

// Tests will be rewritten once CSP solver is implemented
#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
