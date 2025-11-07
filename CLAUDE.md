# Treesearch - Claude Code Guide

## Project Overview

Treesearch is a high-performance toolkit for querying linguistic dependency parses at scale, designed for corpus linguistics research on large treebanks (500M+ tokens). The project uses Rust for the performance-critical core with Python bindings via PyO3 for ease of use.

**Author**: Rob Malouf (rmalouf@sdsu.edu)
**License**: MIT
**Status**: Phase 0 - Algorithm-First Implementation

## Current Development Phase

**Phase 0: Algorithm-First Implementation**

The project is in its early stages, implementing the pattern matching virtual machine BEFORE building the full CoNLL-U parser and query language. This ensures all components are optimized for the matching workflow from the start.

### Phase 0 Progress
- ✅ Project structure setup
- 🚧 VM instruction execution (in progress)
- ⏳ Wildcard search with BFS
- ⏳ Backtracking implementation
- ⏳ Test fixtures
- ⏳ Optimization and benchmarking

## Architecture

### Core Design Principles

1. **Two-phase matching strategy**: Index lookup → VM verification
2. **Deterministic match semantics**: Leftmost, shortest-path matching
3. **Efficient wildcard handling**: Avoids exponential blowup
4. **File-level parallelization**: Using rayon

### Key Components

#### Rust Core (`src/`)
- `lib.rs` - Main library entry point with module declarations
- `tree.rs` - Minimal tree data structures for representing dependency parses
- `pattern.rs` - Pattern AST representation and compilation
- `vm.rs` - Virtual machine executor with instruction set
- `index.rs` - Inverted indices for fast candidate lookup

#### Python Bindings (`python/`)
- PyO3-based bindings (Phase 1, not yet implemented)
- Will provide ergonomic Python API over Rust core

#### Planning Documents (`plans/`)
- `PROJECT_SUMMARY.md` - Overall project roadmap and design
- `PHASE_0_IMPLEMENTATION_PLAN.md` - Current phase details
- `pattern_matching_vm_design.md` - VM architecture and instruction set
- `SETUP_NOTES.md` - Setup and configuration notes

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

### 1. Algorithm-First Approach
The project deliberately implements the matching VM before parsing/query language to ensure optimal performance from the start.

### 2. Virtual Machine Architecture
Pattern matching uses a stack-based VM with specialized instructions for tree navigation, avoiding regex-style backtracking where possible.

### 3. Two-Phase Matching
- **Phase 1**: Inverted indices quickly find candidate nodes
- **Phase 2**: VM verifies complete pattern match

### 4. Deterministic Matching
Always returns leftmost, shortest-path matches to ensure reproducible results.

## Working with This Codebase

### When Adding Features
1. Start with Rust core implementation (`src/`)
2. Add tests in `tests/`
3. Add benchmarks in `benches/` if performance-critical
4. Update relevant planning docs in `plans/`
5. Python bindings come later (Phase 1+)

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

### What It Doesn't Do Yet
- No query language parser yet (Phase 0 focuses on matching)
- No CoNLL-U parser yet (using minimal test fixtures)
- No Python bindings yet (Phase 1+)

### Performance Goals
- Handle 500M+ token corpora
- Sub-second queries on typical patterns
- Memory-efficient indexing

### Academic Context
This is for corpus linguistics research, where researchers need to find specific syntactic patterns across massive treebanks.

## Getting Started with Development

1. **First time setup**:
   ```bash
   cargo check
   cargo test
   ```

2. **Understanding the codebase**: Start with these files in order:
   - `README.md` - User-level overview
   - `plans/PROJECT_SUMMARY.md` - Design rationale
   - `plans/PHASE_0_IMPLEMENTATION_PLAN.md` - Current work
   - `src/lib.rs` - Code structure
   - `plans/pattern_matching_vm_design.md` - Core algorithm

3. **Making changes**:
   - Read relevant planning docs first
   - Implement in Rust core
   - Add tests
   - Run `cargo test` and `cargo check`
   - Update planning docs if design changes

## References

- **Repository**: https://github.com/rmalouf/treesearch
- **Related work**: This project builds on lessons from existing treebank query tools but prioritizes performance for very large corpora.
