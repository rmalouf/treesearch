# Treesearch

A high-performance toolkit for querying linguistic dependency parses at scale.

## Project Status

**Current Phase**: Core Complete, Python Bindings Ready

The core pattern matching engine is implemented using constraint satisfaction (CSP) with exhaustive search. Python bindings are now functional with parallel processing support. Benchmarking remains to be completed.

## Overview

Treesearch is designed for corpus linguistics research on large treebanks (500M+ tokens). It provides:

- Fast structural pattern matching over dependency trees
- Rust core for performance with Python bindings for ease of use
- Exhaustive match semantics (finds ALL valid matches)
- CSP-based solver with forward checking for efficiency

## Architecture

- **Core implementation**: Rust
- **Python bindings**: PyO3 + maturin
- **Pattern matching**: Constraint satisfaction with DFS + forward checking
- **Parallelization**: rayon for file-level parallelism

## Current Status

✅ **Implemented:**
- Query language parser (Pest-based)
- Pattern AST representation
- CoNLL-U file parsing with transparent gzip support
- Tree data structures with string interning
- CSP solver with exhaustive search
- Python bindings with parallel iterators
- 38 tests passing

⏳ **In Progress:**
- Performance benchmarks
- Extended documentation

## Project Structure

```
treesearch/
├── src/
│   ├── tree.rs       # Tree data structures
│   ├── pattern.rs    # Pattern AST representation
│   ├── query.rs      # Query language parser (Pest)
│   ├── searcher.rs   # CSP solver
│   ├── conllu.rs     # CoNLL-U file parsing
│   ├── iterators.rs  # Iterator interfaces
│   └── python.rs     # Python bindings
├── tests/            # Integration tests
├── benches/          # Performance benchmarks
├── examples/         # Usage examples
└── plans/            # Design documents
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

# Build Python package
maturin develop
```

## Query Language Example

```
# Declare pattern variables with constraints
Help [lemma="help"];
To [lemma="to"];
Verb [upos="VERB"];

# Specify structural relationships
Help -> To;            # Help has child To
To -[mark]-> Verb;     # To has child Verb with deprel=mark
```

### Feature Constraints

Query morphological features using dotted notation:

```
# Find past tense verbs
Verb [feats.Tense="Past"];

# Find plural nominative nouns
Noun [feats.Number="Plur", feats.Case="Nom"];

# Combine with other constraints
Be [lemma="be", upos="VERB", feats.Tense="Past"];
```

Feature constraints use exact string matching (case-sensitive) and return no match if the feature is not present.

## Python Usage

```python
import treesearch

# Parse a query into a compiled pattern
pattern = treesearch.parse_query("""
    Verb [upos="VERB"];
    Noun [upos="NOUN"];
    Verb -[nsubj]-> Noun;
""")

# Search a single file
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    # match is a dictionary: {"Verb": 3, "Noun": 5}
    verb = tree.get_word(match["Verb"])
    noun = tree.get_word(match["Noun"])
    print(f"{verb.form} has subject {noun.form}")

# Search multiple files in parallel
for tree, match in treesearch.search_files("data/*.conllu", pattern, parallel=True):
    # Process matches from all files with automatic parallelization
    verb = tree.get_word(match["Verb"])
    print(f"Found: {verb.form} in {tree.sentence_text}")
```

See `API.md` for complete API reference and examples.

## Next Steps

1. Add comprehensive benchmarks
2. Further optimize for large corpora
3. Extend query features (negation, regex, precedence operators)
4. Publish to PyPI

See `CLAUDE.md` and `plans/` for detailed design documentation.

## License

MIT
