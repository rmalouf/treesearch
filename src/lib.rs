#![allow(clippy::large_enum_variant, clippy::result_large_err)]

//! Treesearch: High-performance dependency tree pattern matching
//!
//! A toolkit for querying linguistic dependency parses at scale.
//! Core implementation in Rust with Python bindings.

// Core modules
pub mod bytes;
pub mod conllu; // CoNLL-U file parsing
pub mod iterators; // Iterator interfaces for trees and matches
pub mod pattern; // Pattern AST
pub mod python;
pub mod query; // Query language parser
pub mod searcher;
pub mod tree; // Tree data structures with full CoNLL-U support

// Re-exports for convenience
pub use conllu::TreeIterator;
pub use iterators::{Treebank, TreebankError};
pub use pattern::{Constraint, EdgeConstraint, Pattern, PatternVar, RelationType, VarId};
pub use query::compile_query;
pub use searcher::{Match, search_tree, search_tree_query, tree_matches};
pub use tree::{Features, TokenId, Tree, Word, WordId};
