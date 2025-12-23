# Treesearch

High-performance dependency treebank querying for corpus linguistics research.

## What is Treesearch?

Treesearch lets you find syntactic patterns in dependency-parsed corpora using a simple query language. Designed for very large treebanks (500M+ tokens), it combines:

- **Fast pattern matching** using constraint satisfaction with exhaustive search
- **Simple query language** for specifying structural patterns
- **Parallel processing** for searching multiple files efficiently
- **Python API** for easy integration with research workflows

## Quick Example

```python
import treesearch

# Find help-to-infinitive constructions
query = """
MATCH {
    Help [lemma="help", upos="VERB"];
    To [lemma="to"];
    XComp [upos="VERB"];
    Help -[xcomp]-> XComp;
    XComp -[mark]-> To;
    Help << XComp;
}
"""

pattern = treesearch.compile_query(query)
for tree, match in treesearch.search("corpus/*.conllu", pattern):
    help_word = tree[match["Help"]]
    xcomp_word = tree[match["XComp"]]
    print(f"{help_word.form} ... to {xcomp.form}: {tree.sentence_text}")
```

## Key Features

### Powerful Query Language
- Node constraints: lemma, POS tags, word forms (with negation support)
- Dependency edges: specify parent-child relationships (positive and negative)
- Negative edge constraints: require absence of relationships
- Precedence operators: linear word order constraints
- See [Query Language](guide/query-language.md) for complete reference

### Built for Scale
- Designed for corpora with 500M+ tokens
- Parallel file processing using Rust's rayon
- Iterator-based API for memory efficiency
- Transparent gzip support

## Get Started

- **[Installation](getting-started/installation.md)** - Install from source or pip
- **[Quick Start](getting-started/quickstart.md)** - 5-minute tutorial
- **[Query Language](guide/query-language.md)** - Learn the query syntax
- **[API Reference](api/functions.md)** - Complete API documentation

## Example Workflows

- **[Finding Constructions](workflows/constructions.md)** - Locate specific syntactic patterns
- **[Frequency Analysis](workflows/frequency.md)** - Count construction occurrences
- **[Extracting Examples](workflows/examples.md)** - Get representative sentences

## Architecture

Treesearch uses a constraint satisfaction approach to pattern matching:

- **Core**: Rust for performance-critical code
- **Bindings**: PyO3 for Python integration
- **Solver**: CSP with DFS and forward checking
- **Parallelization**: File-level parallel processing

See [Architecture](advanced/architecture.md) for implementation details.

## License

MIT License - see repository for details.
