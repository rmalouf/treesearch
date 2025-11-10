//! Treesearch: High-performance dependency tree pattern matching
//!
//! A toolkit for querying linguistic dependency parses at scale.
//! Core implementation in Rust with Python bindings.

// Core modules (algorithm-first approach)
pub mod compiler; // Pattern compilation to VM opcodes
pub mod conllu; // CoNLL-U file parsing
pub mod index; // Inverted indices for candidate lookup
pub mod parser; // Query language parser
pub mod pattern; // Pattern AST and compilation
pub mod searcher;
pub mod tree; // Tree data structures with full CoNLL-U support
pub mod vm; // Virtual machine executor and instruction set // End-to-end search (index + compiler + VM)

// Python bindings
#[cfg(feature = "pyo3")]
pub mod python;

// Re-exports for convenience
pub use conllu::CoNLLUReader;
pub use parser::parse_query;
pub use pattern::{Pattern, PatternElement};
pub use searcher::{SearchError, search, search_query};
pub use tree::{Features, Node, TokenId, Tree};
pub use vm::{Instruction, Match, VM};

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        // Placeholder test - will add real tests as we implement modules
        assert!(true);
    }
}
