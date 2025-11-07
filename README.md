# Treesearch

A high-performance toolkit for querying linguistic dependency parses at scale.

## Project Status

**Current Phase**: Algorithm-First Implementation (Phase 0)

We're implementing the pattern matching virtual machine first, before building out the full CoNLL-U parser and query language. This approach ensures that all other components are optimized for the matching workflow.

## Overview

Treesearch is designed for corpus linguistics research on large treebanks (500M+ tokens). It provides:

- Fast structural pattern matching over dependency trees
- Rust core for performance with Python bindings for ease of use
- Deterministic match semantics (leftmost, shortest-path)
- Efficient handling of wildcard patterns without exponential blowup

## Architecture

- **Core implementation**: Rust
- **Python bindings**: PyO3 + maturin
- **Pattern matching**: Two-phase strategy (index lookup → VM verification)
- **Parallelization**: rayon for file-level parallelism

## Project Structure

```
treesearch/
├── src/
│   ├── tree.rs      # Minimal tree data structures
│   ├── pattern.rs   # Pattern AST representation
│   ├── vm.rs        # Virtual machine executor
│   └── index.rs     # Inverted indices
├── tests/           # Integration tests
├── benches/         # Performance benchmarks
├── examples/        # Usage examples
├── python/          # Python package (Phase 1)
└── plans/           # Design documents
```

## Development Setup

### Requirements

- Rust (latest stable)
- Python 3.12+
- maturin

### Building

```bash
# Check Rust code
cargo check

# Run tests
cargo test

# Build Python package (when ready)
maturin develop
```

## Next Steps (Phase 0)

1. ✅ Set up project structure
2. Implement VM instruction execution
3. Add wildcard search with BFS
4. Implement backtracking
5. Create test fixtures
6. Optimize and benchmark

See `plans/PROJECT_SUMMARY.md` for detailed roadmap.

## License

MIT
