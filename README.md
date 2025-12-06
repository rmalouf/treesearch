# Treesearch

A high-performance toolkit for querying linguistic dependency parses at scale.

> **⚠️ Early Stage**: This project is under active development. The API and query language **will** change as we refine the design. Feedback and contributions are welcome!

## Overview

Treesearch enables structural pattern matching over dependency parse trees, designed for corpus linguistics research on large treebanks. Key features:

## Installation

### From Source

Requires Python 3.12+ and [Rust toolchain](https://www.rust-lang.org/tools/install).

```bash
# Clone repository
git clone https://github.com/rmalouf/treesearch
cd treesearch

# Install with uv (recommended)
uv pip install -e .

# Or with pip
pip install maturin
maturin develop
```

## Quick Start

```python
import treesearch

# Define a pattern
pattern = treesearch.parse_query("""
    MATCH {
        Verb [upos="VERB"];
        Noun [upos="NOUN"];
        Verb -[nsubj]-> Noun;
    }
""")

# Search a single file
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["Verb"])
    noun = tree.get_word(match["Noun"])
    print(f"{verb.form} has subject {noun.form}")

# Search multiple files in parallel
for tree, match in treesearch.search_files("data/*.conllu", pattern):
    verb = tree.get_word(match["Verb"])
    print(f"Found: {verb.form}")
```

## Query Language

### Node Constraints

Declare pattern variables with constraints on word properties:

```
MATCH {
    Verb [upos="VERB", lemma="be"];
    Noun [upos="NOUN"];
    Adj [upos="ADJ"];
}
```

**Available constraints:**
- `upos="VERB"` - Universal POS tag
- `xpos="VBD"` - Language-specific POS tag
- `lemma="run"` - Lemma
- `form="running"` - Surface form
- `deprel="nsubj"` - Dependency relation
- `feats.Tense="Past"` - Morphological features

### Edge Constraints

Specify structural relationships between nodes:

```
MATCH {
    Verb [upos="VERB"];
    Noun [upos="NOUN"];

    # Verb has child Noun with nsubj relation
    Verb -[nsubj]-> Noun;

    # Any child relationship
    Verb -> Noun;
}
```

### Negative Constraints

Specify edges that must NOT exist:

```
MATCH {
    V [upos="VERB"];
    Obj [];

    # V does NOT have an obj edge to Obj
    V !-[obj]-> Obj;

    # V does NOT have any child
    V !-> Obj;
}
```

### Feature Constraints

Query morphological features using dotted notation:

```
MATCH {
    # Past tense verb
    Verb [feats.Tense="Past"];
}
```

```
MATCH {
    # Plural nominative noun
    Noun [feats.Number="Plur", feats.Case="Nom"];
}
```

```
MATCH {
    # Combine with other constraints
    Be [lemma="be", upos="VERB", feats.Tense="Past"];
}
```

Feature constraints use exact string matching (case-sensitive) and return no match if the feature is not present.

## API Reference

See [API.md](API.md) for complete Python API documentation, including:
- Function reference
- Iterator interfaces
- Tree and Word objects
- Error handling
- Performance tips

## Data Format

Treesearch reads dependency trees in [CoNLL-U format](https://universaldependencies.org/format.html), the standard format for Universal Dependencies treebanks. Files can be plain text (`.conllu`) or gzip-compressed (`.conllu.gz`).

**Tip**: Use gzipped files (`.conllu.gz`) to reduce I/O time and save disk space. Decompression is automatic and transparent—no code changes needed.

## Contributing & Feedback

This project is in early development and we welcome feedback! If you:

- Find a bug or unexpected behavior
- Have suggestions for the query language
- Want to request a feature
- Have questions about usage

Please [open an issue](https://github.com/rmalouf/treesearch/issues) on GitHub.

## License

MIT

## Citation

If you use Treesearch in your research, please cite:

```bibtex
@software{treesearch,
  author = {Malouf, Robert},
  title = {Treesearch: Pattern matching for dependency treebanks},
  year = {2025},
  url = {https://github.com/rmalouf/treesearch}
}
```
