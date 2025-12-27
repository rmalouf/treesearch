# Current Project Status

**Last Updated**: December 2025

## Overview

This document tracks the internal development status of Treesearch. For user-facing documentation, see `docs/` folder (tutorial, API reference, query language).

## Implementation Status

### ✅ Complete

**Core Pattern Matching Engine**:
- CSP solver with DFS + forward checking
- Query language parser (Pest-based, `query.rs`)
- Pattern AST representation with constraints
- CoNLL-U file parsing with transparent gzip support
- Tree data structures with string interning (lasso with FxHash)
- Iterator-based API for trees and matches (`iterators.rs`)
- Automatic parallel file processing using rayon + channels
- Negative edge constraints (`!->`, `!-[label]->`)
- EXCEPT blocks for negative existential queries
- OPTIONAL blocks for optional variable binding
- Match struct with Arc<Tree> sharing
- Morphological features (FEATS field) and miscellaneous annotations (MISC field)
- 100 tests passing (96 unit + 4 doctests)

**Python Bindings**:
- PyO3 wrapper code in `src/python.rs`
- **Streamlined API** with both object-oriented and functional interfaces
- Full integration with Rust core
- **Object-Oriented API**:
  - `Treebank` class with `from_file()`, `from_files()`, `from_string()` class methods
  - Instance methods: `trees(ordered)`, `search(pattern, ordered)` for iteration
  - Convenience functions: `load(source)`, `from_string(text)`
- **Functional API**: `compile_query()`, `search()`, `trees()`, `search_trees()`
- Data classes: `Tree`, `Word`, `Pattern`, `Treebank`
- Iterator classes: `TreeIterator`, `MatchIterator`
- Full access to word features (FEATS) and misc annotations (MISC)
- Improved error handling with IndexError for out-of-range word IDs
- Automatic parallel processing for multi-file operations
- 46 Python tests passing

**Documentation**:
- Complete user documentation in `docs/`:
  - `index.md` - Landing page with quick start
  - `tutorial.md` - Complete walkthrough from installation to advanced usage
  - `query-language.md` - Full syntax reference
  - `api.md` - Functions and classes reference
  - `internals.md` - Architecture for contributors

### 🔄 In Progress

**Performance Benchmarks**:
- Basic benchmarks exist (`benches/coha.rs`, `benches/conllu.rs`)
- Need expansion to cover real-world query patterns

### ⏳ Not Started

**Documentation & Polish**:
- Comprehensive rustdoc for Rust APIs (user docs complete, internal docs pending)

**Future Enhancements**:
- Extended query language features (regex in constraints, disjunctions, wildcards)
- Additional relation types (ancestor, sibling, distance constraints)
- Export to CoNLL-U subcorpus
- DEPS (enhanced dependencies) support in query language

## Development Roadmap

See `ROADMAP.md` for detailed implementation plans.

### Planned Features

1. **PyPI Publishing** - Enable `pip install treesearch`
2. **Regular expressions in node constraints** - `[form=~/.*ing$/]`
3. **Disjunctions in node constraints** - `[upos="NOUN" | upos="PROPN"]`
4. **Wildcards in dependency constraints** - `X -[nsubj:*]-> Y`
5. **Export to CoNLL-U subcorpus** - Save matching trees to files
6. **DEPS field support in queries** - Query enhanced dependencies (MISC field already supported)

### Future Enhancements

- Ancestor/descendant relation types (`X <<- Y`)
- Sibling relations (`X ~ Y`)
- Distance constraints (`X <-[2..5]- Y`)
- Query result caching
- Corpus indexing for faster queries

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

- **String interning**: lasso with FxHash for memory efficiency
  - Thread-local string pools (each TreeIterator has its own pool)
  - Zero contention in parallel processing
- **File-level parallelization**: rayon + channels for automatic parallel processing
  - Bounded channels (size 8) for backpressure
  - Chunk-based processing (8 files per chunk)
  - Thread-safe tree sharing via Arc
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
├── src/              # Rust core
│   ├── lib.rs        # Module declarations and re-exports
│   ├── tree.rs       # Tree data structures
│   ├── pattern.rs    # Pattern AST representation
│   ├── query.rs      # Query language parser (Pest)
│   ├── searcher.rs   # CSP solver
│   ├── conllu.rs     # CoNLL-U file parsing
│   ├── iterators.rs  # Iterator interfaces
│   ├── bytes.rs      # Byte handling utilities
│   └── python.rs     # Python bindings (PyO3)
├── tests/            # Integration tests (96 Rust tests passing)
├── python/           # Python package (46 tests passing)
├── benches/          # Performance benchmarks
├── examples/         # Usage examples
├── docs/             # User documentation (5 files)
└── plans/            # Design documents (this file)
```

See `PROJECT_SUMMARY.md` for architectural overview, `PARSING_OPTIMIZATION_PLAN.md` for parsing performance optimizations, and `ROADMAP.md` for planned features.
