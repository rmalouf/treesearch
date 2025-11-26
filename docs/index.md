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
    Help [lemma="help"];
    To [lemma="to"];
    V [upos="VERB"];
    Help -[xcomp]-> To;
    To < V;
"""

pattern = treesearch.parse_query(query)
for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    help_word = tree.get_word(match["Help"])
    verb = tree.get_word(match["V"])
    print(f"{help_word.form} ... to {verb.form}: {tree.sentence_text}")
```

## Key Features

### Exhaustive Search
Finds **all** valid matches in your corpus - no arbitrary filtering or pruning.

### Powerful Query Language
- Node constraints: lemma, POS tags, word forms
- Dependency edges: specify parent-child relationships
- Precedence operators: linear word order constraints
- See [Query Language](guide/query-language.md) for complete reference

### Built for Scale
- Designed for corpora with 500M+ tokens
- Parallel file processing using Rust's rayon
- Iterator-based API for memory efficiency
- Transparent gzip support

### Corpus Linguistics Focus
Target audience is researchers working with dependency treebanks. Examples and workflows focus on common corpus linguistics tasks like finding syntactic constructions and extracting examples.

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
