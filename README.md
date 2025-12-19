# Treesearch

Pattern matching for dependency treebanks.

> **⚠️ Early Stage**: This project is under active development. The API and query language **will** change as we refine the design.

## Overview

Treesearch finds syntactic patterns in dependency-parsed corpora. It reads treebanks in CoNLL-U format and returns all sentences matching a specified structural pattern. Designed for corpus linguistics research on large treebanks with automatic parallel processing for multi-file operations.

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

## Quick Example

Find passive constructions in an English treebank:

```python
import treesearch

# Parse a pattern for passive voice
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V <-[aux:pass]- Aux;
    }
""")

# Search a single file
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

Search multiple files with automatic parallel processing:

```python
# Glob pattern for multiple files
treebank = treesearch.open("data/*.conllu")
for tree, match in treebank.matches(pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

## Pattern Language

Patterns specify structural constraints on dependency trees:

```
MATCH {
    Verb [upos="VERB", lemma="help"];
    Obj [upos="NOUN"];
    Verb -[obj]-> Obj;
}
```

**Node constraints**: `upos`, `xpos`, `lemma`, `form`, `deprel`, `feats.*` (morphological features)

**Edge constraints**: `->` (child), `<-` (parent), `-[label]->` (labeled edge), `!->` (negative)

**Precedence**: `<` (immediately precedes), `<<` (precedes)

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
