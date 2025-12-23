# Multi-File Processing

Guide to processing multiple CoNLL-U files efficiently.

## Glob Patterns

Use glob patterns to match multiple files:

```python
import treesearch

# All .conllu files
for tree in treesearch.trees("data/*.conllu"):
    print(tree.sentence_text)

# Recursive pattern
for tree in treesearch.trees("data/**/*.conllu"):
    print(tree.sentence_text)

# Multiple patterns: use list comprehension
patterns = ["data/*.conllu", "corpus/*.conllu.gz"]
for pattern in patterns:
    for tree in treesearch.trees(pattern):
        print(tree.sentence_text)
```

## Automatic Parallel Processing

Multi-file operations automatically process files in parallel for better performance:

```python
# Automatic parallel processing
for tree, match in treesearch.search("*.conllu", pattern):
    process(tree, match)
```

**Note:** Results may not be in deterministic file order due to parallel processing.

## Performance Tips

1. **Automatic parallelization** - Multi-file operations use all available cores
2. **Parse queries once** before searching
3. **Use get_matches()** instead of read + search

```python
# Best: direct search with automatic parallelization
pattern = treesearch.parse_query(query)
for tree, match in treesearch.search("*.conllu", pattern):
    process(tree, match)
```

## Next Steps

- [Searching Patterns](searching.md) - Search strategies
- [API Reference](../api/functions.md) - Complete documentation
