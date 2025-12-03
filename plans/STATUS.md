# Current Project Status

**Last Updated**: December 2025

## Overview

This document tracks the internal development status of Treesearch. For user-facing documentation, see README.md and API.md.

## Implementation Status

### ✅ Complete

**Core Pattern Matching Engine**:
- CSP solver with DFS + forward checking
- Query language parser (Pest-based, `query.rs`)
- Pattern AST representation with constraints
- CoNLL-U file parsing with transparent gzip support
- Tree data structures with string interning (rustc-hash FxHash + hashbrown)
- Iterator-based API for trees and matches (`iterators.rs`)
- Parallel file processing using rayon
- Negative edge constraints (`!->`, `!-[label]->`)
- 89 tests passing, 4378 lines of code

**Python Bindings**:
- PyO3 wrapper code in `src/python.rs`
- Functional API (refactored from OO in commit 137499c)
- Full test suite passing (pytest)
- Functions: `parse_query`, `search`, `read_trees`, `search_file`, `read_trees_glob`, `search_files`
- Data classes: `Tree`, `Word`, `Pattern`

### 🔄 In Progress

**Performance Benchmarks**:
- Basic benchmarks exist (`benches/coha.rs`, `benches/conllu.rs`)
- Need expansion to cover real-world query patterns

### ⏳ Not Started

**Documentation & Polish**:
- Comprehensive rustdoc for public APIs
- API documentation needs update to reflect functional API changes

**Future Enhancements**:
- Extended query language features (negation, regex, more operators)
- Additional relation types (ancestor, sibling, etc.)
- Performance optimization (after benchmarking establishes baseline)

## Development Roadmap

### Next Priorities

1. **Benchmarks** - Expand coverage beyond basic benchmarks to establish performance baseline
2. **Documentation** - Add comprehensive rustdoc comments for public APIs
3. **Extended query features** - Add regex support, additional relation types (ancestor, descendant)
4. **Performance optimization** - Based on benchmark results
5. **PyPI Publishing** - Package and publish to PyPI for easy installation

### Future Enhancements

- Regex patterns in constraints
- Ancestor/descendant relation types
- Optional/alternative patterns (OR constraints)
- Query result caching
- Incremental parsing for very large files

## Architecture & Implementation Details

### Core Algorithm

The pattern matching engine uses **constraint satisfaction programming (CSP)**:

- **Variables**: Pattern nodes to be matched against tree words
- **Domains**: Tree words that satisfy each variable's node constraints
- **Constraints**: Edge relationships (child, precedes, follows, negative edges)
- **Solver**: Depth-first search (DFS) with forward checking
- **Heuristics**: MRV (Minimum Remaining Values) for variable ordering
- **Global constraints**: AllDifferent (no two variables bind to same word)
- **Search strategy**: Exhaustive (finds ALL valid solutions)

### Key Implementation Details

- **String interning**: lasso + FxHash for memory efficiency
- **File-level parallelization**: rayon for parallel processing across files
- **Transparent compression**: Automatic gzip detection and decompression
- **Iterator-based design**: Memory-efficient streaming without loading entire corpus
- **Error handling**: User errors → Result::Err, internal bugs → panic with context

### Technology Stack

- **Core**: Rust 2024 edition
- **Parser**: Pest 2.8 (PEG parser generator)
- **Hashing**: rustc-hash 2.1 (FxHash) + hashbrown 0.16
- **Compression**: flate2 1.1 (gzip with zlib-rs)
- **Allocator**: mimalloc 0.1
- **Parallelization**: rayon 1.11
- **Python bindings**: PyO3 0.27 + maturin
- **Benchmarking**: divan 0.1

### Project Structure

```
treesearch/
├── src/              # Rust core (4378 lines)
│   ├── lib.rs        # Module declarations and re-exports
│   ├── tree.rs       # Tree data structures
│   ├── pattern.rs    # Pattern AST representation
│   ├── query.rs      # Query language parser (Pest)
│   ├── searcher.rs   # CSP solver
│   ├── conllu.rs     # CoNLL-U file parsing
│   ├── iterators.rs  # Iterator interfaces
│   ├── bytes.rs      # Byte handling utilities
│   └── python.rs     # Python bindings (PyO3)
├── tests/            # Integration tests (89 passing)
├── benches/          # Performance benchmarks
├── examples/         # Usage examples
└── plans/            # Design documents (this file)
```

See `PROJECT_SUMMARY.md` for architectural overview and `PARSING_OPTIMIZATION_PLAN.md` for parsing performance optimizations.
