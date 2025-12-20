# API reference

Complete Python API documentation for treesearch.

## Overview

### Treebank operations

Creating and iterating over collections of dependency trees:

- [Treebank](treebank.md) - Collection of trees from files or strings

### Pattern compilation

Parsing query strings into reusable pattern objects:

- [parse_query()](functions.md#parse_query) - Compile query string to Pattern

### Searching

Finding pattern matches in trees:

- [search()](functions.md#search) - Search single tree
- [search_file()](functions.md#search_file) - Search single file
- [search_files()](functions.md#search_files) - Search multiple files

### Tree and Word access

Navigating dependency structures and accessing word properties:

- [Tree](tree-word.md#tree) - Dependency tree representation
- [Word](tree-word.md#word) - Individual word with properties and relations

### Utility functions

Reading trees without searching:

- [read_trees()](functions.md#read_trees) - Read single file
- [read_trees_glob()](functions.md#read_trees_glob) - Read multiple files

## Quick reference

### Basic workflow

```python
import treesearch

# Compile pattern once
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        N [upos="NOUN"];
        V -[obj]-> N;
    }
""")

# Search multiple files
for tree, match in treesearch.search_files("data/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    noun = tree.get_word(match["N"])
    print(f"{verb.form} -> {noun.form}")
```

### Object-oriented workflow

```python
import treesearch

# Create treebank
tb = treesearch.Treebank.from_glob("data/*.conllu")

# Iterate over trees
for tree in tb.trees():
    print(tree.sentence_text)

# Search for patterns
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')
for tree, match in tb.matches(pattern):
    verb = tree.get_word(match["V"])
    print(verb.form)
```

## Common patterns

### Single file operations

```python
# Read trees
for tree in treesearch.read_trees("corpus.conllu"):
    print(tree.sentence_text)

# Search file
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    process(match)
```

### Multi-file operations

```python
# Read from multiple files (automatic parallel processing)
for tree in treesearch.read_trees_glob("data/*.conllu"):
    analyze(tree)

# Search multiple files (automatic parallel processing)
for tree, match in treesearch.search_files("data/*.conllu", pattern):
    process(match)
```

### Working with matches

```python
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    # Match is dict mapping variable names to word IDs
    verb_id = match["V"]
    noun_id = match["N"]

    # Get Word objects
    verb = tree.get_word(verb_id)
    noun = tree.get_word(noun_id)

    # Access properties
    print(f"{verb.form} ({verb.pos}) -> {noun.form} ({noun.pos})")
```

## API sections

- [Treebank](treebank.md) - Treebank class and methods
- [Tree & Word](tree-word.md) - Tree and Word classes
- [Pattern](pattern.md) - Pattern class
- [Functions](functions.md) - Standalone functions
