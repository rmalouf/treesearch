# Treesearch - Claude Code Guide

## Project Overview

Treesearch is a high-performance toolkit for querying linguistic dependency parses at scale, designed for corpus linguistics research on large treebanks (500M+ tokens). The project uses Rust for the performance-critical core with Python bindings via PyO3 for ease of use.

**Author**: Rob Malouf (rmalouf@sdsu.edu)
**License**: MIT
**Status**: Phase 0 - Algorithm-First Implementation

## Current Development Phase

**Phase 0: Algorithm-First Implementation** ✅ 95% COMPLETE (Nov 2025)

The project has **successfully completed** the core pattern matching VM implementation. All major Phase 0 objectives achieved, with bonus completion of the query parser (originally planned for Phase 1).

### Phase 0 Progress
- ✅ Project structure setup
- ✅ VM instruction execution (ALL instructions working)
- ✅ Wildcard search with BFS (shortest-path guarantees)
- ✅ Backtracking implementation (full support)
- ✅ Pattern compiler (anchor selection, bytecode generation)
- ✅ **BONUS: Query language parser** (Phase 1 item completed early!)
- ✅ Test fixtures (56 tests passing)
- ⏳ TreeSearcher integration (pending)
- ⏳ Performance benchmarking (pending)

**Current Status**: Ready to begin Phase 1 (CoNLL-U integration)

## Architecture

### Core Design Principles

1. **Two-phase matching strategy**: Index lookup → VM verification
2. **Deterministic match semantics**: Leftmost, shortest-path matching
3. **Efficient wildcard handling**: Avoids exponential blowup
4. **File-level parallelization**: Using rayon
5. **Error handling strategy**:
   - **User input errors** (malformed queries, invalid CoNLL-U, missing files) → `Result::Err` with clear message
   - **Internal bugs** (invalid bytecode, violated invariants, unreachable states) → `panic!` with descriptive message
   - **Never silently skip or provide fallback values** - all errors must be loud and visible

### Key Components

#### Rust Core (`src/`)
- `lib.rs` - Main library entry point with module declarations
- `tree.rs` - Minimal tree data structures for representing dependency parses (122 lines)
- `pattern.rs` - Pattern AST representation (146 lines)
- `vm.rs` - Virtual machine executor with instruction set (1,436 lines, 39 tests) ✅
- `compiler.rs` - Pattern compilation to VM bytecode (523 lines, 11 tests) ✅
- `parser.rs` - Query language parser using Pest (264 lines, 6 tests) ✅
- `query.pest` - Pest grammar for query language ✅
- `index.rs` - Inverted indices for fast candidate lookup (116 lines) ✅

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

### Current Architecture (Phase 0 Complete)

```
Query String  →  Parser (pest)  →  Pattern AST  →  Compiler  →  Bytecode  →  VM  →  Match
                    ✅                ✅              ✅           ✅         ✅      ✅
```

All core components working. Index exists but needs TreeSearcher integration.

### When Adding Features
1. Core VM and compiler are stable - avoid changes unless necessary
2. Focus on Phase 1 items: CoNLL-U parsing, TreeSearcher, Python bindings
3. Add tests following existing patterns (56 tests provide good examples)
4. Add benchmarks in `benches/` (currently empty, needs implementation)
5. Update planning docs when design decisions change

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
- ✅ Complete pattern matching VM with all instructions
- ✅ Query language parser (can parse queries like `Help [lemma="help"]; To [lemma="to"]; Help -[xcomp]-> To;`)
- ✅ Pattern compilation with anchor selection
- ✅ Backtracking for ambiguous patterns
- ✅ BFS wildcard search (descendants, ancestors, siblings)

### What's Still Needed
- ⏳ TreeSearcher integration (combine index + compiler + VM)
- ⏳ CoNLL-U parser (currently using minimal test fixtures)
- ⏳ Python bindings (Phase 1)
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
   cargo check              # Compiles without warnings
   cargo test               # 56 tests pass
   cargo run --example query_example  # See working demo
   ```

2. **Understanding the codebase**: Start with these files in order:
   - `README.md` - User-level overview
   - `plans/PROJECT_SUMMARY.md` - Design rationale and current status
   - `plans/PHASE_0_PROGRESS.md` - Detailed progress notes
   - `examples/query_example.rs` - Working example of query → execution
   - `src/vm.rs` - Core VM implementation (well-tested)
   - `src/compiler.rs` - Pattern compilation
   - `src/parser.rs` - Query language parsing

3. **Making changes**:
   - Phase 0 core is stable - avoid changing VM/compiler unless necessary
   - Focus on Phase 1: CoNLL-U parsing, TreeSearcher, Python bindings
   - Follow existing test patterns (56 tests provide good examples)
   - Run `cargo test` and `cargo check` before committing
   - Update planning docs if design changes

4. **Running examples**:
   ```bash
   cargo run --example query_example
   ```

## References

- **Repository**: https://github.com/rmalouf/treesearch
- **Related work**: This project builds on lessons from existing treebank query tools but prioritizes performance for very large corpora.
