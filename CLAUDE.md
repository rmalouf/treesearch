# Treesearch - Claude Code Guide

## Project Overview

Treesearch is a high-performance toolkit for querying linguistic dependency parses at scale, designed for corpus linguistics research on large treebanks (500M+ tokens). The project uses Rust for the performance-critical core with Python bindings via PyO3 for ease of use.

**Author**: Rob Malouf (rmalouf@sdsu.edu)
**License**: MIT
**Status**: Rewriting matching algorithm as constraint satisfaction problem

## Current Development Phase

**VM Rewrite** (Nov 2025)

The project is undergoing a fundamental redesign. The previous VM-based approach has been removed in favor of treating pattern matching as a constraint satisfaction problem.

### Current Status
- ✅ Query language parser (can parse queries)
- ✅ Pattern AST representation
- ✅ CoNLL-U parsing and tree structures
- ✅ Inverted indices for candidate lookup
- 🔄 **Rewriting matcher as CSP solver** (in progress)

## Architecture

### Core Design Principles

1. **Constraint satisfaction approach**: Pattern matching as CSP solving
2. **Index-based candidate lookup**: Inverted indices for fast initial filtering
3. **File-level parallelization**: Using rayon
4. **Error handling strategy**:
   - **User input errors** (malformed queries, invalid CoNLL-U, missing files) → `Result::Err` with clear message
   - **Internal bugs** (violated invariants, unreachable states) → `panic!` with descriptive message
   - **Never silently skip or provide fallback values** - all errors must be loud and visible

### Key Components

#### Rust Core (`src/`)
- `lib.rs` - Main library entry point with module declarations
- `tree.rs` - Tree data structures for representing dependency parses
- `pattern.rs` - Pattern AST representation
- `parser.rs` - Query language parser using Pest
- `query.pest` - Pest grammar for query language
- `index.rs` - Inverted indices for fast candidate lookup
- `conllu.rs` - CoNLL-U file parsing
- `searcher.rs` - Main search coordination (to be rewritten as CSP solver)

#### Python Bindings (`python/`)
- PyO3-based bindings (Phase 1, not yet implemented)
- Will provide ergonomic Python API over Rust core

#### Planning Documents (`plans/`)
- `PROJECT_SUMMARY.md` - Overall project roadmap and design
- `PHASE_1_PLAN.md` - Next phase planning

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

# Build Python package (Phase 1+)
maturin develop
```

### Dependencies

**Rust (Cargo.toml)**:
- `pyo3` (0.27) - Python bindings
- `rayon` (1.11) - Data parallelism
- `criterion` (0.7, dev) - Benchmarking

**Python (pyproject.toml)**:
- Requires Python 3.12+
- Uses maturin for building

## Key Design Decisions

### 1. Constraint Satisfaction Approach
Pattern matching is treated as a constraint satisfaction problem (CSP). Each pattern element represents a variable that must be bound to a tree node, subject to constraints from node attributes and edge relationships.

### 2. Index-Based Candidate Generation
Inverted indices quickly generate candidate nodes for each pattern variable, providing the initial domains for the CSP solver.

### 3. Performance Focus
Designed to handle very large corpora (500M+ tokens) with file-level parallelization using rayon.

## Working with This Codebase

### Current Architecture (In Progress)

```
Query String  →  Parser  →  Pattern AST  →  CSP Solver  →  Matches
                   ✅          ✅              🔄             🔄
```

Index exists for candidate lookup. CSP solver needs to be implemented.

### When Adding Features
1. CSP solver is the current focus - this is the core matching algorithm
2. Add tests as you implement
3. Add benchmarks in `benches/` to measure performance
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
- ✅ Pattern AST representation
- ✅ CoNLL-U file parsing
- ✅ Tree data structures
- ✅ Inverted indices for candidate lookup

### What's Being Rewritten
- 🔄 Pattern matching algorithm (moving from VM to CSP approach)

### What's Still Needed
- ⏳ CSP solver implementation
- ⏳ Python bindings
- ⏳ Performance benchmarks

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
   - `src/index.rs` - Inverted indices
   - `src/searcher.rs` - Search coordination (being rewritten)

3. **Making changes**:
   - Current focus: Rewriting pattern matching as CSP solver
   - Run `cargo test` and `cargo check` before committing
   - Update planning docs when design changes

## References

- **Repository**: https://github.com/rmalouf/treesearch
- **Related work**: This project builds on lessons from existing treebank query tools but prioritizes performance for very large corpora.
