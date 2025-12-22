# Searching Patterns

Guide to searching trees for pattern matches.

## Basic Search

### Search a Single Tree

```python
import treesearch

pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

for tree in treesearch.get_trees("corpus.conllu"):
    for match in treesearch.search(tree, pattern):
        verb = tree.get_word(match["V"])
        print(verb.form)
```

### Search a Single File

More efficient than reading and searching separately:

```python
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

for tree, match in treesearch.get_matches("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

### Search Multiple Files

Best for large corpora:

```python
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

for tree, match in treesearch.get_matches("data/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

## Understanding Matches

Matches are dictionaries mapping variable names to word IDs:

```python
match = {"V": 3, "N": 7}
```

Use these IDs with `tree.get_word()`:

```python
verb = tree.get_word(match["V"])
noun = tree.get_word(match["N"])
```

## Exhaustive Search

Treesearch finds **all** valid matches:

```python
query = """
MATCH {
    X [];
    Y [];
    X -> Y;
}
"""

# This will find ALL parent-child pairs in every tree
pattern = treesearch.parse_query(query)
for tree, match in treesearch.get_matches("corpus.conllu", pattern):
    # Many matches per tree
    pass
```

## Performance

### Parse Once

```python
# Good
pattern = treesearch.parse_query(query)
for tree, match in treesearch.get_matches("*.conllu", pattern):
    pass

# Bad: re-parsing
for tree, match in treesearch.get_matches("*.conllu", treesearch.parse_query(query)):
    pass
```

### Use Parallel Processing

```python
# Fast: parallel (default)
for tree, match in treesearch.get_matches("*.conllu", pattern):
    pass

# Slower: sequential
for tree, match in treesearch.get_matches("*.conllu", pattern, parallel=False):
    pass
```

## Next Steps

- [Working with Results](results.md) - Navigate trees and extract data
- [Query Language](query-language.md) - Complete query syntax
- [API Reference](../api/functions.md) - Complete function documentation
