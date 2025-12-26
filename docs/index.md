# Treesearch

High-performance dependency treebank querying for corpus linguistics.

## Quick Start

```bash
pip install treesearch
```

```python
import treesearch as ts

# Find all passive constructions
query = """
MATCH {
    V [upos="VERB"];
    V -[aux:pass]-> _;
    V -[nsubj:pass]-> Subj;
}
"""

for tree, match in ts.search("corpus/*.conllu", query):
    verb = tree.word(match["V"])
    subj = tree.word(match["Subj"])
    print(f"{subj.form} was {verb.form}: {tree.sentence_text}")
```

## Features

- **Query language** for structural patterns (nodes, edges, precedence, negation)
- **Exhaustive search** finds all matches using CSP solving
- **Automatic parallelism** for multi-file processing
- **Memory efficient** streaming with string interning
- **Transparent gzip** support

## Documentation

- **[Tutorial](tutorial.md)** - Complete walkthrough from installation to advanced usage
- **[Query Language](query-language.md)** - Full syntax reference
- **[API Reference](api.md)** - Functions and classes
- **[Internals](internals.md)** - Architecture for contributors

## License

MIT
