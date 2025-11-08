//! Treesearch: High-performance dependency tree pattern matching
//!
//! A toolkit for querying linguistic dependency parses at scale.
//! Core implementation in Rust with Python bindings.

// Core modules (algorithm-first approach)
pub mod tree;      // Tree data structures with full CoNLL-U support
pub mod pattern;   // Pattern AST and compilation
pub mod vm;        // Virtual machine executor and instruction set
pub mod index;     // Inverted indices for candidate lookup
pub mod compiler;  // Pattern compilation to VM bytecode
pub mod parser;    // Query language parser
pub mod conllu;    // CoNLL-U file parsing
pub mod searcher;  // End-to-end search (index + compiler + VM)

// Python bindings (will be implemented in Phase 1)
#[cfg(feature = "python")]
pub mod python;

// Re-exports for convenience
pub use tree::{Node, Tree, Features, TokenId};
pub use pattern::{Pattern, PatternElement};
pub use vm::{VM, Instruction, Match};
pub use parser::parse_query;
pub use conllu::CoNLLUReader;
pub use searcher::TreeSearcher;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        // Placeholder test - will add real tests as we implement modules
        assert!(true);
    }
}
