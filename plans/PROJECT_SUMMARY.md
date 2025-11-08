# Treesearch - Project Summary

## Overview

High-performance toolkit for querying linguistic dependency parses at scale. Rust core with Python bindings for corpus linguistics research.

**Primary Use Case**: Structural pattern matching over large treebanks (500M+ tokens, 1000s of files).

## Core Architecture

**1. CoNLL-U Parsing** - Read and parse Universal Dependencies files
**2. Pattern Matching VM** - Execute structural queries with backtracking
**3. Index + Compile + Execute** - Two-phase strategy for efficiency
**4. Python Bindings** (PyO3) - Pythonic API for research workflows

### Query Language

```
# Node declarations with constraints
Help [lemma="help"];
To [lemma="to"];
Verb [];

# Edge declarations (structural relations)
Help -[xcomp]-> To;
To -[mark]-> Verb;
```

### Matching Algorithm

1. **Index lookup**: Find candidate anchor nodes using inverted indices
2. **VM verification**: Execute compiled bytecode to verify structural constraints
3. **Match semantics**: Leftmost, shortest-path (deterministic results)
4. **Bounded search**: BFS with depth limits prevents exponential blowup

## Current Status (Nov 2025)

### Completed ✅

**Phase 0: Core Matching** (95% complete)
- ✅ VM with all instructions (vm.rs: 1,436 lines, 39 tests)
- ✅ BFS wildcard search with shortest-path guarantees
- ✅ Full backtracking support
- ✅ Pattern compiler with anchor selection (523 lines, 11 tests)
- ✅ Query language parser using Pest (264 lines, 6 tests)
- ✅ Inverted indices for fast lookup (116 lines)
- ✅ 71 tests passing

**Phase 1: Integration** (~90% complete)
- ✅ Full CoNLL-U parser with error handling (conllu.rs: 444 lines)
- ✅ Complete tree representation (all UD fields)
- ✅ TreeSearcher end-to-end pipeline (searcher.rs: 169 lines)
- ✅ Leftmost semantics using token positions
- ✅ End-to-end examples

### Remaining Work ⏳

**Phase 1 Polish**:
- Python bindings (PyO3)
- Performance benchmarks
- Comprehensive rustdoc

**Phase 2 (Future)**:
- Multi-file processing with rayon
- Extended query features (regex, negation)
- Performance optimization

## Technology Stack

- **Language**: Rust 2021 edition
- **Python**: PyO3 + maturin
- **Parser**: Pest 2.7
- **Parallelization**: Rayon 1.11

## Key Design Principles

1. **Performance**: Rust core for 500M+ token corpora
2. **Deterministic**: Leftmost, shortest-path semantics
3. **Error handling**: User errors → Result::Err, bugs → panic with context
4. **No pathological cases**: Bounded search prevents exponential blowup
5. **Python-friendly**: Ergonomic bindings for research workflows

## Example Workflow

```python
from treesearch import TreeSearcher, CoNLLUReader

# Load treebank
reader = CoNLLUReader.from_file("corpus.conllu")
trees = list(reader)

# Execute query
searcher = TreeSearcher()
query = """
    Verb [pos="VERB"];
    Subj [pos="NOUN"];
    Verb -[nsubj]-> Subj;
"""

for tree in trees:
    for match in searcher.search(tree, query):
        # Custom analysis on matched structures
        verb = match['Verb']
        subj = match['Subj']
        print(f"{verb.form} ← {subj.form}")
```

## References

- Design details: `plans/pattern_matching_vm_design.md`
- CoNLL-U format: https://universaldependencies.org/format.html
- Development guide: `CLAUDE.md`
