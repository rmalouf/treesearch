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
- **Regular expressions** for flexible pattern matching with automatic anchoring
- **Exhaustive search** finds all matches using CSP solving
- **Automatic parallelism** for multi-file processing
- **Memory efficient** streaming with string interning
- **Transparent gzip** support

## Quick Examples

```python
# Exact match
ts.search("*.conllu", 'MATCH { V [lemma="run"]; }')

# Regex: words ending in -ing
ts.search("*.conllu", 'MATCH { V [form=/.*ing/]; }')

# Regex: VERB or AUX
ts.search("*.conllu", 'MATCH { V [upos=/VERB|AUX/]; }')

# Complex: progressive construction
query = """
MATCH {
    Aux [lemma=/be.*/];      # be, is, was, etc.
    V [form=/.*ing/];        # -ing form
    Aux -[aux]-> V;
}
"""
```

## Documentation

- **[Tutorial](tutorial.md)** - Complete walkthrough from installation to advanced usage
- **[Query Language](query-language.md)** - Full syntax reference
- **[API Reference](api.md)** - Functions and classes
- **[Internals](internals.md)** - Architecture for contributors

## License

MIT
