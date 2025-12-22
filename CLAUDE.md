# Treesearch - Claude Code Guide

## Project Overview

Treesearch is a high-performance toolkit for querying linguistic dependency parses at scale, designed for corpus linguistics research on large treebanks (500M+ tokens). The project uses Rust for the performance-critical core with Python bindings via PyO3 for ease of use.

**Author**: Rob Malouf (rmalouf@sdsu.edu)
**License**: MIT
**Status**: Core complete (95 Rust tests + 40 Python tests passing), Python bindings fully working

---

## Quick Reference

### Python API

```python
import treesearch as ts

# Load treebank
tb = ts.load("corpus.conllu")           # Single file or glob pattern
tb = ts.from_string(conllu_text)        # From string

# Search for patterns
pattern = ts.compile_query('MATCH { V [upos="VERB"]; }')
for tree, match in tb.search(pattern):
    verb = tree.word(match["V"])
    print(verb.form)

# Convenience functions
for tree in ts.trees("*.conllu"):       # Read trees
    print(tree.sentence_text)

for tree, match in ts.search("*.conllu", pattern):  # Search files
    process(match)
```

### Key Classes

- **Tree**: Dependency tree with `word(id)`, `sentence_text`, `metadata`
- **Word**: Tree node with `form`, `lemma`, `upos`, `deprel`, `parent()`, `children()`
- **Pattern**: Compiled query (from `compile_query()`)
- **Treebank**: Collection of trees with `trees()`, `search()` methods

### Error Handling

- `tree.word(id)` raises `IndexError` if out of range
- File/parsing errors raise Python exceptions during iteration
- Invalid queries raise `ValueError` from `compile_query()`

---

## Architecture

### Core Components

**Rust Core** (`src/`):
- `tree.rs` - Tree data structures with string interning (lasso + FxHash)
- `pattern.rs` - Pattern AST representation with constraints
- `query.rs` - Query language parser (Pest-based)
- `searcher.rs` - CSP solver (DFS + forward checking + MRV heuristic)
- `conllu.rs` - CoNLL-U parsing with transparent gzip support
- `iterators.rs` - Channel-based parallel iteration (rayon)
- `python.rs` - Python bindings (PyO3)

**Python Package** (`python/treesearch/`):
- Wrapper functions: `load()`, `trees()`, `search()`, `compile_query()`
- Classes: `Tree`, `Word`, `Pattern`, `Treebank`
- Iterators: `TreeIterator`, `MatchIterator`

### Design Principles

1. **Constraint satisfaction**: Pattern matching as CSP solving with exhaustive search
2. **All solutions**: Find ALL possible matches, no pruning
3. **Automatic parallelism**: File-level parallelism with rayon + channels
4. **Iterator-based**: Memory-efficient streaming of trees and matches
5. **Pythonic API**: Clean, simple interface with proper error handling

### Error Handling Strategy

- **User errors** (bad queries, missing files, invalid CoNLL-U) → Python exceptions
- **Internal bugs** (violated invariants) → `panic!` with descriptive message
- **Never silently skip** - all errors must be visible

---

## Development

### Building

```bash
cargo check                # Compile Rust
cargo test                 # Run Rust tests (95)
maturin develop            # Build Python bindings
pytest tests/              # Run Python tests (40)
```

### Dependencies

**Rust**:
- `pyo3` (0.27) - Python bindings
- `pest` (2.7) - Parser generator
- `lasso` (0.7) - String interning
- `flate2` (1.0) - Gzip support
- `rayon` / `crossbeam-channel` - Parallel processing

**Python**:
- Python 3.12+
- Built with maturin

### Code Style

- Rust: Standard rustfmt
- Python: Ruff (line-length=100, target=py312)
- Prefer functional interfaces over OO where appropriate
- Documentation: rustdoc for public APIs

---

## Documentation

**User-Facing** (no implementation details):
- `README.md` - Installation, quick start, query language
- `API.md` - Python API reference with examples
- `docs/` - Comprehensive user documentation

**Developer** (internal details):
- `CLAUDE.md` - This file (quick reference)
- `plans/STATUS.md` - Implementation status and roadmap
- `plans/PROJECT_SUMMARY.md` - Architecture overview
- `plans/PARSING_OPTIMIZATION_PLAN.md` - Performance notes

---

## Common Tasks

### Adding Python API features

1. Add Rust implementation in `src/python.rs`
2. Export in `python/treesearch/__init__.py`
3. Add type hints in `python/treesearch/treesearch.pyi`
4. Add tests in `tests/test_python_bindings.py`
5. Document in `API.md`

### Running tests

```bash
# All Rust tests
cargo test

# Specific Rust test
cargo test test_name

# All Python tests
pytest tests/test_python_bindings.py

# Specific Python test
pytest tests/test_python_bindings.py::TestClass::test_method

# With output
pytest -v -s
```

### Profiling

```bash
# Time profiler (macOS)
cargo instruments -t time --example latwp_par --release

# List available templates
cargo instruments -l
```

---

## Implementation Notes

### Recent API Changes (Dec 2025)

- **Error handling**: `tree.word(id)` now raises `IndexError` instead of returning `None`
- **Repr formats**:
  - `Tree`: `<Tree len=6 words='He helped us ...'>`
  - `Word`: `<Word id=1 form='helped' lemma='help' upos='VERB' deprel='root'>`
- **Property rename**: `Word.pos` → `Word.upos` for clarity
- **Function renames**: `parse_query()` → `compile_query()`

See `plans/STATUS.md` for detailed implementation history and roadmap.

---

## Academic Context

This tool is for corpus linguistics research on large treebanks. Researchers need to:
- Find specific syntactic patterns (e.g., passive constructions, raising verbs)
- Count construction frequencies
- Extract examples for analysis
- Handle 500M+ token corpora efficiently

The CSP-based approach ensures exhaustive search - finding ALL matches, not just the first or "best" one.
