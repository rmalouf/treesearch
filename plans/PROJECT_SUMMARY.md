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
# Node declarations with constraints
Help [lemma="help"];
To [lemma="to"];
Verb [pos="VERB"];

# Edge declarations (structural relations)
Help -> To;           # Help has child To
To -[mark]-> Verb;    # To has child Verb with deprel=mark
```

### Matching Algorithm

**Constraint Satisfaction Problem (CSP)**:
- Variables: Pattern nodes to be matched
- Domains: Tree words satisfying node constraints
- Constraints: Edge relationships (child, precedes, follows)
- Solver: DFS with forward checking and MRV heuristic
- Global constraint: AllDifferent (no two variables bind to same word)
- Result: ALL valid solutions (exhaustive)

## Current Status (November 2025)

### Completed ✅

**Core Implementation** (100% complete)
- ✅ CSP solver with DFS + forward checking (searcher.rs)
- ✅ Query language parser using Pest (query.rs, formerly parser.rs)
- ✅ Pattern AST with constraints (pattern.rs)
- ✅ CoNLL-U parser with transparent gzip support (conllu.rs)
- ✅ Tree data structures with string interning using rustc-hash FxHash + hashbrown (tree.rs)
- ✅ Iterator-based APIs for trees and matches (iterators.rs)
- ✅ Parallel file processing with rayon
- ✅ 50 tests passing (3094 lines of code)

**Python Bindings** (100% complete)
- ✅ PyO3 bindings with functional API (python.rs)
- ✅ Full test suite passing (pytest)
- ✅ Functions: `parse_query`, `search`, `read_trees`, `search_file`, `read_trees_glob`, `search_files`
- ✅ Data classes: `Tree`, `Word`, `Pattern`

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
    Verb [pos="VERB"];
    Subj [pos="NOUN"];
    Verb -[nsubj]-> Subj;
"""
pattern = ts.parse_query(query_str)

# Search single file
for match in ts.search_file("corpus.conllu", pattern):
    verb_idx, subj_idx = match
    print(f"Found match: verb={verb_idx}, subject={subj_idx}")

# Or search multiple files in parallel
for match in ts.search_files("data/*.conllu", pattern, parallel=True):
    # Process matches from all files
    pass

# Or work with individual trees
for tree in ts.read_trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        verb = tree.words[match[0]]
        subj = tree.words[match[1]]
        print(f"{verb.form} ← {subj.form}")
```

## References

- CoNLL-U format: https://universaldependencies.org/format.html
- Development guide: `CLAUDE.md`
- Repository: https://github.com/rmalouf/treesearch
