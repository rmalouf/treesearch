# Treesearch - Project Summary

## Overview

High-performance toolkit for querying linguistic dependency parses at scale. Rust core with Python bindings for corpus linguistics research.

**Primary Use Case**: Structural pattern matching over large treebanks (500M+ tokens, 1000s of files).

## Core Architecture

**1. CoNLL-U Parsing** - Read and parse Universal Dependencies files
**2. Pattern Matching CSP** - Execute structural queries with constraint satisfaction
**3. Exhaustive Search** - Find ALL valid matches, no pruning
**4. Python Bindings** (PyO3) - Pythonic API for research workflows

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

## Current Status (Nov 2025)

### Completed ✅

**Core Implementation** (100% complete)
- ✅ CSP solver with DFS + forward checking (searcher.rs: 472 lines, 18 tests)
- ✅ Query language parser using Pest (parser.rs: 264 lines, 6 tests)
- ✅ Pattern AST with constraints (pattern.rs: 200+ lines)
- ✅ CoNLL-U parser with gzip support (conllu.rs: 446 lines, 14 tests)
- ✅ Tree data structures with string interning (tree.rs: 400+ lines)
- ✅ 38 tests passing (2372 lines of code)

### In Progress 🔄

**Python Bindings**:
- 🔄 PyO3 bindings partially implemented (python.rs exists)
- ⏳ Not yet functional or tested

### Remaining Work ⏳

**Polish & Performance**:
- ⏳ Complete Python bindings (PyO3)
- ⏳ Performance benchmarks (Criterion)
- ⏳ Multi-file processing with rayon
- ⏳ Comprehensive rustdoc

**Future Enhancements**:
- ⏳ Extended query features (negation, regex)
- ⏳ More relation types (ancestor, sibling, etc.)
- ⏳ Performance optimization based on benchmarks

## Technology Stack

- **Language**: Rust 2021 edition
- **Python**: PyO3 + maturin
- **Parser**: Pest 2.7
- **String interning**: lasso with FxHash
- **Compression**: flate2 (gzip)
- **Parallelization**: Rayon 1.11 (planned)
- **Benchmarking**: Criterion 0.7

## Key Design Principles

1. **Performance**: Rust core for 500M+ token corpora
2. **Exhaustive**: Find ALL matches, no pruning (leftmost/shortest)
3. **Error handling**: User errors → Result::Err, bugs → panic with context
4. **Efficient search**: CSP with forward checking prevents exponential blowup
5. **Python-friendly**: Ergonomic bindings for research workflows

## Example Workflow (Planned)

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

- CoNLL-U format: https://universaldependencies.org/format.html
- Development guide: `CLAUDE.md`
- Repository: https://github.com/rmalouf/treesearch
