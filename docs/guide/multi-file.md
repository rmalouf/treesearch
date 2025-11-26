# Multi-File Processing

Guide to processing multiple CoNLL-U files efficiently.

## Glob Patterns

Use glob patterns to match multiple files:

```python
import treesearch

# All .conllu files
for tree in treesearch.read_trees_glob("data/*.conllu"):
    print(tree.sentence_text)

# Recursive pattern
for tree in treesearch.read_trees_glob("data/**/*.conllu"):
    print(tree.sentence_text)

# Multiple patterns: use list comprehension
patterns = ["data/*.conllu", "corpus/*.conllu.gz"]
for pattern in patterns:
    for tree in treesearch.read_trees_glob(pattern):
        print(tree.sentence_text)
```

## Parallel Processing

By default, files are processed in parallel:

```python
# Parallel (default) - faster
for tree, match in treesearch.search_files("*.conllu", pattern):
    process(tree, match)

# Sequential - preserves file order
for tree, match in treesearch.search_files("*.conllu", pattern, parallel=False):
    process(tree, match)
```

## Performance Tips

1. **Use parallel mode** for large corpora
2. **Parse queries once** before searching
3. **Use search_files()** instead of read + search

```python
# Best: direct search with parallelization
pattern = treesearch.parse_query(query)
for tree, match in treesearch.search_files("*.conllu", pattern):
    process(tree, match)
```

## Next Steps

- [Searching Patterns](searching.md) - Search strategies
- [API Reference](../api/functions.md) - Complete documentation
