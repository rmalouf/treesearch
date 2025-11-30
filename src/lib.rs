#![allow(clippy::large_enum_variant, clippy::result_large_err)]

//! Treesearch: High-performance dependency tree pattern matching
//!
//! A toolkit for querying linguistic dependency parses at scale.
//! Core implementation in Rust with Python bindings.

// Core modules
pub mod conllu; // CoNLL-U file parsing
pub mod iterators; // Iterator interfaces for trees and matches
pub mod pattern; // Pattern AST
pub mod query; // Query language parser
pub mod searcher;
pub mod tree; // Tree data structures with full CoNLL-U support

// Python bindings
mod bytes;
#[cfg(feature = "pyo3")]
pub mod python;

// Re-exports for convenience
pub use conllu::TreeIterator;
pub use iterators::{MatchSet, TreeSet};
pub use pattern::{Constraint, EdgeConstraint, Pattern, PatternVar, RelationType, VarId};
pub use query::parse_query;
pub use searcher::{Match, search, search_query};
pub use tree::{Features, TokenId, Tree, Word, WordId};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test - will add real tests as we implement modules
        assert!(true);
    }
}
