# Treesearch - Claude Code Guide

## Project Overview

Treesearch is a high-performance toolkit for querying linguistic dependency parses at scale, designed for corpus linguistics research on large treebanks (500M+ tokens). The project uses Rust for the performance-critical core with Python bindings via PyO3 for ease of use.

**Author**: Rob Malouf (rmalouf@sdsu.edu)
**License**: MIT
**Status**: Core implementation complete, Python bindings in progress

## Current Development Phase

**Core Complete, Python Bindings WIP** (Nov 2025)

The core pattern matching engine is fully implemented using constraint satisfaction. Python bindings are partially implemented but not yet functional.

### Current Status
- ✅ Query language parser (Pest-based)
- ✅ Pattern AST representation with constraints
- ✅ CoNLL-U parsing with transparent gzip support
- ✅ Tree data structures with string interning (lasso + FxHash)
- ✅ CSP solver with DFS + forward checking
- ✅ 38 tests passing (2372 lines of code)
- 🔄 **Python bindings** (partially implemented)
- ⏳ **Performance benchmarks** (not started)

## Architecture

### Core Design Principles

1. **Constraint satisfaction approach**: Pattern matching as CSP solving with exhaustive search
2. **All solutions**: Find ALL possible matches, no filtering or pruning based on leftmost/shortest/etc.
3. **File-level parallelization**: Using rayon (planned, not yet implemented)
4. **Error handling strategy**:
   - **User input errors** (malformed queries, invalid CoNLL-U, missing files) → `Result::Err` with clear message
   - **Internal bugs** (violated invariants, unreachable states) → `panic!` with descriptive message
   - **Never silently skip or provide fallback values** - all errors must be loud and visible

### Key Components

#### Rust Core (`src/`)
- `lib.rs` - Main library entry point with module declarations
- `tree.rs` - Tree data structures for representing dependency parses
  - String interning using lasso with FxHash
  - Parent/child relationships
  - Full CoNLL-U field support
- `pattern.rs` - Pattern AST representation
  - Variable constraints (lemma, pos, form, deprel)
  - Edge constraints (child, precedes, follows)
  - Constraint combinators (And, Or)
- `parser.rs` - Query language parser using Pest
- `query.pest` - Pest grammar for query language
- `conllu.rs` - CoNLL-U file parsing
  - Transparent gzip detection via magic bytes
  - Iterator-based API for memory efficiency
  - Full error reporting with line numbers
- `searcher.rs` - CSP solver for pattern matching (IMPLEMENTED)
  - DFS with forward checking
  - MRV (Minimum Remaining Values) variable ordering
  - AllDifferent constraint
  - Arc consistency checking
- `python.rs` - Python bindings via PyO3 (WIP)

#### Python Bindings (`python/`)
- PyO3-based bindings (partial implementation in `src/python.rs`)
- Will provide ergonomic Python API over Rust core

#### Planning Documents (`plans/`)
- `PROJECT_SUMMARY.md` - Overall project roadmap and design
- `PHASE_1_PLAN.md` - Phase 1 planning (may be outdated)

## Directory Structure

```
treesearch/
├── src/              # Rust core implementation
├── python/           # Python package structure (Phase 1+)
├── tests/            # Integration tests
├── benches/          # Performance benchmarks (Criterion)
├── examples/         # Usage examples
├── plans/            # Design documents and roadmaps
├── Cargo.toml        # Rust dependencies and config
├── pyproject.toml    # Python packaging config (maturin)
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

# Build Python package (not yet functional)
maturin develop
```

### Dependencies

**Rust (Cargo.toml)**:
- `pyo3` (0.27) - Python bindings
- `rayon` (1.11) - Data parallelism
- `pest` (2.7) - Parser generator
- `lasso` (0.7) - String interning
- `flate2` (1.0) - Gzip support
- `criterion` (0.7, dev) - Benchmarking

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
- File-level parallelization using rayon (planned)
- Transparent gzip support

## Working with This Codebase

### Current Architecture

```
Query String  →  Parser  →  Pattern AST  →  CSP Solver  →  All Matches
                   ✅          ✅              ✅              ✅
```

All core components are implemented and working.

### When Adding Features
1. **Python bindings** are the current priority
2. **Benchmarks** should be added to establish performance baselines
3. Add tests as you implement
4. Update planning docs when design decisions change

### Code Style
- Rust: Standard rustfmt style
- Python: Ruff with line-length=100, target py312
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
- ✅ Query language parser (can parse queries like `Help [lemma="help"]; To [lemma="to"]; Help -[xcomp]-> To;`)
- ✅ Pattern AST representation with constraints
- ✅ CoNLL-U file parsing with gzip detection
- ✅ Tree data structures with string interning
- ✅ CSP solver with exhaustive search (DFS + forward checking)
- ✅ 38 tests passing

### What Needs Work
- 🔄 Python bindings (started but not functional)
- ⏳ Performance benchmarks
- ⏳ Multi-file processing with rayon
- ⏳ Extended query features (negation, regex, more relation types)

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
   - `plans/PROJECT_SUMMARY.md` - Design rationale and current status
   - `src/parser.rs` - Query language parsing
   - `src/pattern.rs` - Pattern AST
   - `src/tree.rs` - Tree data structures
   - `src/conllu.rs` - CoNLL-U parsing
   - `src/searcher.rs` - CSP solver (IMPLEMENTED)

3. **Making changes**:
   - Current focus: Python bindings and benchmarks
   - Run `cargo test` and `cargo check` before committing
   - Update planning docs when design changes

## References

- **Repository**: https://github.com/rmalouf/treesearch
- **CoNLL-U format**: https://universaldependencies.org/format.html
- **Related work**: This project builds on lessons from existing treebank query tools but prioritizes performance for very large corpora.
