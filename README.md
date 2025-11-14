# Treesearch

A high-performance toolkit for querying linguistic dependency parses at scale.

## Project Status

**Current Phase**: Core Implementation Complete

The core pattern matching engine is implemented using constraint satisfaction (CSP) with exhaustive search. Python bindings and benchmarking remain to be completed.

## Overview

Treesearch is designed for corpus linguistics research on large treebanks (500M+ tokens). It provides:

- Fast structural pattern matching over dependency trees
- Rust core for performance with Python bindings for ease of use
- Exhaustive match semantics (finds ALL valid matches)
- CSP-based solver with forward checking for efficiency

## Architecture

- **Core implementation**: Rust
- **Python bindings**: PyO3 + maturin (in progress)
- **Pattern matching**: Constraint satisfaction with DFS + forward checking
- **Parallelization**: rayon for file-level parallelism (planned)

## Current Status

✅ **Implemented:**
- Query language parser (Pest-based)
- Pattern AST representation
- CoNLL-U file parsing with transparent gzip support
- Tree data structures with string interning
- CSP solver with exhaustive search
- 38 tests passing

⏳ **In Progress:**
- Python bindings (PyO3)
- Performance benchmarks
- Documentation

## Project Structure

```
treesearch/
├── src/
│   ├── tree.rs      # Tree data structures
│   ├── pattern.rs   # Pattern AST representation
│   ├── parser.rs    # Query language parser (Pest)
│   ├── searcher.rs  # CSP solver
│   ├── conllu.rs    # CoNLL-U file parsing
│   └── python.rs    # Python bindings (WIP)
├── tests/           # Integration tests
├── benches/         # Performance benchmarks
├── examples/        # Usage examples
└── plans/           # Design documents
```

## Development Setup

### Requirements

- Rust (latest stable)
- Python 3.12+ (for bindings)
- maturin (for building Python package)

### Building

```bash
# Check Rust code
cargo check

# Run tests
cargo test

# Run benchmarks
cargo bench

# Build Python package (when ready)
maturin develop
```

## Query Language Example

```
# Declare pattern variables with constraints
Help [lemma="help"];
To [lemma="to"];
Verb [pos="VERB"];

# Specify structural relationships
Help -> To;           # Help has child To
To -[mark]-> Verb;    # To has child Verb with deprel=mark
```

## Next Steps

1. Complete Python bindings
2. Add comprehensive benchmarks
3. Optimize for large corpora (rayon parallelization)
4. Extend query features (negation, regex, precedence operators)

See `CLAUDE.md` and `plans/` for detailed design documentation.

## License

MIT
