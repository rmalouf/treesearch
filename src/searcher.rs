//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Solve CSP to find ALL matches (exhaustive search)
//! 3. Yield matches
//!
//! TODO: Implement CSP solver

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


// Tests will be rewritten once CSP solver is implemented
#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
