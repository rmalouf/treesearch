#![allow(clippy::large_enum_variant, clippy::result_large_err)]

//! Treesearch: High-performance dependency tree pattern matching
//!
//! A toolkit for querying linguistic dependency parses at scale.
//! Core implementation in Rust with Python bindings.

// Core modules
mod conllu; // CoNLL-U file parsing
mod iterators; // Iterator interfaces for trees and matches
mod pattern; // Pattern AST
mod query; // Query language parser
mod searcher;
mod tree; // Tree data structures with full CoNLL-U support
mod python;
mod bytes;

// Re-exports for convenience
pub use conllu::TreeIterator;
pub use iterators::Treebank;
pub use pattern::{Constraint, EdgeConstraint, Pattern, PatternVar, RelationType, VarId};
pub use query::parse_query;
pub use searcher::{Match, search, search_query};
pub use tree::{Features, TokenId, Tree, Word, WordId};
