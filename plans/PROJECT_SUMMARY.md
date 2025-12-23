# Treesearch - Project Summary

## Overview

High-performance toolkit for querying linguistic dependency parses at scale. Rust core with Python bindings for corpus linguistics research.

**Primary Use Case**: Structural pattern matching over large treebanks (500M+ tokens, 1000s of files).

## Core Architecture

**1. CoNLL-U Parsing** - Read and parse Universal Dependencies files with transparent gzip support
**2. Pattern Matching CSP** - Execute structural queries with constraint satisfaction
**3. Exhaustive Search** - Find ALL valid matches, no pruning
**4. Parallel Processing** - File-level parallelization using rayon
**5. Python Bindings** (PyO3) - Functional API for research workflows

### Query Language

```
MATCH {
    # Node declarations with constraints
    Help [lemma="help"];
    To [lemma="to"];
    Verb [upos="VERB"];

    # Edge declarations (structural relations)
    Help -> To;           # Help has child To
    To -[mark]-> Verb;    # To has child Verb with deprel=mark
}
```

### Matching Algorithm

**Constraint Satisfaction Problem (CSP)**:
- Variables: Pattern nodes to be matched
- Domains: Tree words satisfying node constraints
- Constraints: Edge relationships (child, precedes, follows)
- Solver: DFS with forward checking and MRV heuristic
- Global constraint: AllDifferent (no two variables bind to same word)
- Result: ALL valid solutions (exhaustive)

## Current Status (December 2025)

### Completed ✅

**Core Implementation** (100% complete)
- ✅ CSP solver with DFS + forward checking (searcher.rs)
- ✅ Query language parser using Pest (query.rs)
- ✅ Pattern AST with constraints (pattern.rs)
- ✅ CoNLL-U parser with transparent gzip support (conllu.rs)
- ✅ Tree data structures with string interning using lasso with FxHash (tree.rs)
- ✅ Iterator-based APIs for trees and matches (iterators.rs)
- ✅ Channel-based parallel file processing with rayon
- ✅ Negative edge constraints (`!->`, `!-[label]->`)
- ✅ 95 Rust tests passing (4669 lines of code)

**Python Bindings** (100% complete)
- ✅ PyO3 bindings with streamlined OO + functional API (python.rs)
- ✅ Full test suite passing (40 Python tests)
- ✅ **Object-Oriented API**:
  - `Treebank` class with `from_file()`, `from_files()`, `from_string()` class methods
  - Instance methods: `trees(ordered)`, `matches(pattern, ordered)` for iteration
  - Convenience functions: `open(source)`, `from_string(text)`
- ✅ **Functional API**: `parse_query()`, `search()`, `get_trees()`, `get_matches()`
- ✅ Data classes: `Tree`, `Word`, `Pattern`, `Treebank`
- ✅ Iterator classes: `TreeIterator`, `MatchIterator`

### In Progress 🔄

**Performance Benchmarks**:
- 🔄 Basic benchmarks exist (`benches/coha.rs`, `benches/conllu.rs`)
- 🔄 Need expansion to cover real-world query patterns

### Remaining Work ⏳

**Documentation & Polish**:
- ⏳ Comprehensive rustdoc for public APIs
- ⏳ Update API documentation to reflect functional API

**Future Enhancements**:
- ⏳ Extended query features (negation, regex, more operators)
- ⏳ Additional relation types (ancestor, sibling, etc.)
- ⏳ Performance optimization based on benchmark results

## Technology Stack

- **Language**: Rust 2024 edition
- **Python**: PyO3 0.27 + maturin
- **Parser**: Pest 2.8
- **Hashing**: rustc-hash 2.1 (FxHash) + hashbrown 0.16
- **Compression**: flate2 1.1 (gzip with zlib-rs)
- **Allocator**: mimalloc 0.1
- **Parallelization**: Rayon 1.11
- **Benchmarking**: divan 0.1

## Key Design Principles

1. **Performance**: Rust core for 500M+ token corpora
2. **Exhaustive**: Find ALL matches, no pruning (leftmost/shortest)
3. **Error handling**: User errors → Result::Err, bugs → panic with context
4. **Efficient search**: CSP with forward checking prevents exponential blowup
5. **Python-friendly**: Ergonomic bindings for research workflows

## Example Workflow

```python
import treesearch as ts

# Parse query once
query_str = """
MATCH {
    Verb [upos="VERB"];
    Subj [upos="NOUN"];
    Verb -[nsubj]-> Subj;
}
"""
pattern = ts.parse_query(query_str)

# Object-oriented API: Create treebank and iterate
treebank = ts.Treebank.from_file("corpus.conllu")
for tree, match in treebank.matches(pattern):
  verb = tree.get_word(match["Verb"])
  subj = tree.get_word(match["Subj"])
  print(f"Found match: {verb.form} ← {subj.form}")

# Functional API: Search files directly with automatic parallelization
for tree, match in ts.search("data/*.conllu", pattern):
  verb = tree.get_word(match["Verb"])
  print(f"{verb.form} in: {tree.sentence_text}")

# Work with individual trees
for tree in ts.trees("corpus.conllu"):
  for match in ts.search(tree, pattern):
    verb = tree.get_word(match["Verb"])
    subj = tree.get_word(match["Subj"])
    print(f"{verb.form} ← {subj.form}")
```

## References

- CoNLL-U format: https://universaldependencies.org/format.html
- Development guide: `CLAUDE.md`
- Repository: https://github.com/rmalouf/treesearch
