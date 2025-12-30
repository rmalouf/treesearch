# Treesearch

[![PyPI](https://img.shields.io/pypi/v/treesearch-ud)](https://pypi.org/project/treesearch-ud/)

Pattern matching for dependency treebanks.

> **⚠️ Early Stage**: This project is under active development. The API and query language **will** change as we refine the design.

## Overview

Treesearch finds syntactic patterns in dependency-parsed corpora. It reads treebanks in CoNLL-U format and returns all sentences matching a specified structural pattern. Designed for corpus linguistics research on large treebanks with automatic parallel processing for multi-file operations.

## Installation

### From PyPI

Requires Python 3.12+.

```bash
pip install treesearch-ud

# Optional: Install with visualization support (displaCy)
pip install treesearch-ud[viz]
```

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

## Quick Example

Find passive constructions in an English treebank:

```python
import treesearch

# Parse a pattern for passive voice
pattern = treesearch.compile_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V -[aux:pass]-> Aux;
    }
""")

# Search a single file
for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

Search multiple files with automatic parallel processing:

```python
# Glob pattern for multiple files
for tree, match in treesearch.search("data/*.conllu", pattern):
    verb = tree.word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")

# Or use the object-oriented API
treebank = treesearch.load("data/*.conllu")
for tree, match in treebank.search(pattern):
    verb = tree.word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

## Pattern Language

Patterns specify structural constraints on dependency trees:

```
MATCH {
    Verb [upos="VERB" & lemma="help"];
    Obj [upos="NOUN"];
    Verb -[obj]-> Obj;
}
```

**Node constraints**: `upos`, `xpos`, `lemma`, `form`, `deprel`, `feats.*` (morphological features), `misc.*` (miscellaneous features)

**Edge constraints**: `->` (child), `-[label]->` (labeled edge), `!->` (negative), `!-[label]->` (negative labeled edge)

**Precedence**: `<` (immediately precedes), `<<` (precedes)

**EXCEPT blocks**: Reject matches where a condition is true (negative existential)

**OPTIONAL blocks**: Extend matches with additional bindings if possible

## Data Format

Reads treebanks in [CoNLL-U format](https://universaldependencies.org/format.html). Supports plain text (`.conllu`) and gzip-compressed files (`.conllu.gz`) with automatic decompression.

## Documentation

- [API.md](API.md) - Complete Python API reference
- [GitHub repository](https://github.com/rmalouf/treesearch) - Source code and issue tracker

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
