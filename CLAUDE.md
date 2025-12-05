# Treesearch - Claude Code Guide

## Project Overview

Treesearch is a high-performance toolkit for querying linguistic dependency parses at scale, designed for corpus linguistics research on large treebanks (500M+ tokens). The project uses Rust for the performance-critical core with Python bindings via PyO3 for ease of use.

**Author**: Rob Malouf (rmalouf@sdsu.edu)
**License**: MIT
**Status**: Core complete (87 tests passing), Python bindings fully working

---

## Current Development Phase

**Core Complete, Python Bindings Working, Channel-Based Iteration** (Dec 2025)

The core pattern matching engine is fully implemented using constraint satisfaction with automatic parallel file processing. Python bindings provide a functional API and are fully working.

### Recent Changes (Dec 2025)
- **Completed Python bindings migration** to new iteration API
  - Removed `MatchSet` abstraction for simpler API
  - Automatic parallel processing via channel-based iteration
  - Match struct now contains `Arc<Tree>` and bindings
  - Removed explicit `parallel` parameter (now automatic)
  - All 87 tests passing
- **Channel-based parallel iteration** using rayon + channels
  - File-level parallelism with bounded channels for backpressure
  - Chunk-based processing (8 files per chunk)
  - Simplified iteration: `tree_iter()` and `match_iter()` methods
  - Removed ~200 lines of complex iterator types

### Previous Changes (Nov 2025)
- Refactored `iterators.rs` to use composition instead of duplication
- Updated Python bindings to use functional API
- Added negative edge constraints and morphological features

### Current Status
- ✅ Query language parser (Pest-based, `query.rs`)
- ✅ Pattern AST representation with constraints
- ✅ Negative edge constraints (`!->`, `!-[label]->`)
- ✅ CoNLL-U parsing with transparent gzip support
- ✅ Tree data structures with string interning (lasso + FxHash)
- ✅ CSP solver with DFS + forward checking
- ✅ Iterator-based API for trees and matches (`iterators.rs`)
- ✅ Automatic parallel file processing (rayon + channels)
- ✅ 87 tests passing
- ✅ **Python bindings** (functional API, fully working)
- ✅ **Performance benchmarks** (basic benchmarks exist)

## Architecture

### Core Design Principles

1. **Constraint satisfaction approach**: Pattern matching as CSP solving with exhaustive search
2. **All solutions**: Find ALL possible matches, no filtering or pruning based on leftmost/shortest/etc.
3. **Automatic parallel processing**: Channel-based file-level parallelism with rayon (implemented)
4. **Functional over OO**: Prefer functional APIs over object-oriented ones. Use objects/structs for data storage, not for organizing namespaces.
   - Recent refactor (commit 137499c): Python bindings changed from OO to functional API
5. **Iterator-based design**: Use iterators for memory efficiency and composability
6. **Error handling strategy**:
   - **User input errors** (malformed queries, invalid CoNLL-U, missing files) → `Result::Err` with clear message
   - **Internal bugs** (violated invariants, unreachable states) → `panic!` with descriptive message
   - **Never silently skip or provide fallback values** - all errors must be loud and visible

### Key Components

#### Rust Core (`src/`)
- `lib.rs` - Main library entry point with module declarations and re-exports
- `tree.rs` - Tree data structures for representing dependency parses
  - String interning using lasso with FxHash
  - Parent/child relationships
  - Full CoNLL-U field support
- `pattern.rs` - Pattern AST representation
  - Variable constraints (lemma, pos, form, deprel)
  - Edge constraints (child, precedes, follows)
  - Constraint combinators (And, Or)
- `query.rs` - Query language parser using Pest (previously `parser.rs`)
  - Pest grammar for query language
  - Error handling with thiserror
- `conllu.rs` - CoNLL-U file parsing
  - Transparent gzip detection via magic bytes
  - Iterator-based API for memory efficiency
  - Full error reporting with line numbers
- `searcher.rs` - CSP solver for pattern matching
  - DFS with forward checking
  - MRV (Minimum Remaining Values) variable ordering
  - AllDifferent constraint
  - Arc consistency checking
  - Match struct contains `Arc<Tree>` and bindings
- `iterators.rs` - Iterator interfaces for trees and matches
  - `Treebank` - Collection of trees from files/strings
  - `tree_iter()` method for iterating over trees
  - `match_iter()` method for searching across trees
  - Channel-based parallel processing (rayon + bounded channels)
- `bytes.rs` - Byte handling utilities
- `python.rs` - Python bindings via PyO3 (functional API, fully working)

#### Python Bindings (`python/`)
- PyO3-based bindings in `src/python.rs` (functional API)
- Package structure in `python/treesearch/`
- Functional API design:
  - `parse_query(query: str) -> Pattern` - Parse query strings
  - `search(tree, pattern) -> list[dict]` - Search single tree
  - `read_trees(path) -> Iterator[Tree]` - Read from CoNLL-U file
  - `search_file(path, pattern) -> Iterator[tuple[Tree, dict]]` - Search single file
  - `read_trees_glob(pattern) -> Iterator[Tree]` - Read multiple files (automatic parallelism)
  - `search_files(pattern, pattern) -> Iterator[tuple[Tree, dict]]` - Search multiple files (automatic parallelism)
- Data classes: `Tree`, `Word`, `Pattern` (for data storage, not namespace organization)
- All functions working, automatic parallel processing for multi-file operations

#### Planning Documents (`plans/`)
- `PROJECT_SUMMARY.md` - Overall project roadmap and design (may be outdated)
- `STATUS.md` - Project status tracking
- `PARSING_OPTIMIZATION_PLAN.md` - Parsing optimization notes
- `PHASE_1_PLAN.md` - Phase 1 planning (outdated)

#### Examples and Documentation
- `examples/` - Rust examples (`latwp.rs`, `latwp_par.rs`) and Python examples
- `benches/` - Performance benchmarks (`coha.rs`, `conllu.rs`)
- `README.md` - User-facing documentation (installation, quick start, query language)
- `API.md` - User-facing API reference (Python API only, no internal details)

## Directory Structure

```
treesearch/
├── src/              # Rust core implementation (9 modules, 3094 lines)
│   ├── lib.rs        # Module declarations and re-exports
│   ├── tree.rs       # Tree data structures
│   ├── pattern.rs    # Pattern AST
│   ├── query.rs      # Query parser (formerly parser.rs)
│   ├── searcher.rs   # CSP solver
│   ├── conllu.rs     # CoNLL-U parsing
│   ├── iterators.rs  # Iterator interfaces
│   ├── bytes.rs      # Byte utilities
│   └── python.rs     # Python bindings (compilation broken)
├── python/           # Python package structure
│   └── treesearch/   # Package directory
├── tests/            # Integration tests
├── benches/          # Performance benchmarks (coha.rs, conllu.rs)
├── examples/         # Rust and Python usage examples
├── plans/            # Design documents (may be outdated)
├── Cargo.toml        # Rust dependencies and config
├── pyproject.toml    # Python packaging config (maturin)
├── CLAUDE.md         # This file - comprehensive project guide
├── API.md            # API reference (may be outdated)
└── README.md         # User-facing documentation
```

## Development Workflow

### Building and Testing

```bash
# Check Rust code compiles
cargo check

# Run tests
cargo test

# Build in release mode
cargo build --release

# Run benchmarks
cargo bench

# Build Python package
maturin develop
```

### Dependencies

**Rust (Cargo.toml)**:
- `pyo3` (0.27) - Python bindings
- `pariter` (0.5) - Parallel iterator processing
- `pest` (2.7) - Parser generator
- `lasso` (0.7) - String interning
- `flate2` (1.0) - Gzip support
- `divan` (0.1, dev) - Benchmarking

**Python (pyproject.toml)**:
- Requires Python 3.12+
- Uses maturin for building

## Key Design Decisions

### 1. Constraint Satisfaction Approach
Pattern matching is treated as a constraint satisfaction problem (CSP). Each pattern element represents a variable that must be bound to a tree node, subject to constraints from node attributes and edge relationships.

The solver uses:
- **DFS** with backtracking for search
- **Forward checking** to prune domains early
- **MRV heuristic** for variable ordering
- **AllDifferent** global constraint (no two variables bind to same word)
- **Arc consistency** checking before assignment

### 2. Exhaustive Search
The CSP solver finds ALL possible matches. No pruning strategies like "leftmost" or "shortest path" - we want every valid solution.

### 3. Performance Focus
Designed to handle very large corpora (500M+ tokens) with:
- String interning to reduce memory overhead
- Efficient tree representations
- File-level parallelization using pariter (implemented)
- Transparent gzip support
- Iterator-based APIs to avoid loading entire corpus into memory
- Lazy evaluation throughout (no forced collections)

## Working with This Codebase

### Current Architecture

```
Query String  →  Parser  →  Pattern AST  →  CSP Solver  →  Iterators  →  Matches
                   ✅          ✅              ✅            ✅           ✅
                                                             ↓
                                                        Parallel
                                                        Processing ✅
```

All core Rust components are implemented and working. Python bindings need compilation fixes.

### Documentation Organization

**User-Facing Documentation** (no internal details):
- `README.md` - Installation, quick start, query language overview
- `API.md` - Python API reference with examples

**Developer Documentation** (internal details):
- `CLAUDE.md` - This file - comprehensive development guide
- `plans/STATUS.md` - Implementation status, architecture details, roadmap
- `plans/PROJECT_SUMMARY.md` - Architectural overview
- `plans/PARSING_OPTIMIZATION_PLAN.md` - Performance optimization details

**Important**: Keep README.md and API.md focused on user needs. Move implementation details, test counts, algorithm specifics, and development roadmaps to planning docs.

### When Adding Features
1. Add tests as you implement
2. Update user-facing docs (README.md/API.md) if user-visible changes
3. Update developer docs (CLAUDE.md/STATUS.md) for internal changes
4. Expand benchmarks to cover new features

### Code Style
- Rust: Standard rustfmt style
- Python: Ruff with line-length=100, target py312
- API Design: Prefer functional interfaces over object-oriented. Avoid gratuitous objects - use them for data storage, not namespace organization.
- Documentation: Inline rustdoc comments for public APIs

### Testing Strategy
- Unit tests in source files (`#[cfg(test)]` modules)
- Integration tests in `tests/`
- Benchmarks in `benches/`
- Focus on correctness first, then optimize

## Important Context for AI Assistants

### What This Project Does
Searches for structural patterns in dependency parse trees (linguistic data). Think of it as a specialized query engine for tree-structured linguistic annotations.

**What's Working Now**:
- ✅ Query language parser (supports positive/negative edges, node constraints, precedence)
- ✅ Pattern AST representation with constraints
- ✅ Negative edge constraints (`X !-> Y`, `X !-[label]-> Y`)
- ✅ CoNLL-U file parsing with gzip detection
- ✅ Tree data structures with string interning
- ✅ CSP solver with exhaustive search (DFS + forward checking)
- ✅ Iterator-based APIs for single and multi-file processing
- ✅ Automatic parallel file processing (rayon + channels)
- ✅ Python bindings fully working
- ✅ 87 tests passing

### What Needs Work
- ⏳ **Extended query features** (regex patterns, descendant/ancestor relations)
- ⏳ **Additional benchmarks** for performance profiling

### Performance Goals
- Handle 500M+ token corpora
- Sub-second queries on typical patterns
- Memory-efficient indexing

### Academic Context
This is for corpus linguistics research, where researchers need to find specific syntactic patterns across massive treebanks.

## Getting Started with Development

1. **First time setup**:
   ```bash
   cargo check              # Check compilation
   cargo test               # Run tests
   ```

2. **Understanding the codebase**: Start with these files in order:
   - `README.md` - User-level overview
   - `CLAUDE.md` - This file - comprehensive project guide
   - `src/lib.rs` - Module organization and public API
   - `src/query.rs` - Query language parsing (formerly `parser.rs`)
   - `src/pattern.rs` - Pattern AST
   - `src/tree.rs` - Tree data structures
   - `src/conllu.rs` - CoNLL-U parsing
   - `src/searcher.rs` - CSP solver
   - `src/iterators.rs` - Iterator interfaces
   - `src/python.rs` - Python bindings (currently broken)

3. **Making changes**:
   - **Immediate priority**: Fix Python bindings compilation error
   - Secondary focus: Expand benchmarks and documentation
   - Run `cargo test` and `cargo check` before committing
   - Keep CLAUDE.md up to date with major changes
   - Planning docs in `plans/` may be outdated

## References

- **Repository**: https://github.com/rmalouf/treesearch
- **CoNLL-U format**: https://universaldependencies.org/format.html
- **Related work**: This project builds on lessons from existing treebank query tools but prioritizes performance for very large corpora.
- to run in profiler: cargo instruments -t time --example latwp_par --release
- to list profiler templates: cargo instruments -l