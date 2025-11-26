# Reading Trees

Guide to reading CoNLL-U files and iterating over trees.

## Basic Reading

Use `read_trees()` to read from a single file:

```python
import treesearch

for tree in treesearch.read_trees("corpus.conllu"):
    print(f"Sentence: {tree.sentence_text}")
    print(f"Words: {len(tree)}")
```

## Reading Multiple Files

Use `read_trees_glob()` for multiple files:

```python
# Parallel (default)
for tree in treesearch.read_trees_glob("data/*.conllu"):
    print(tree.sentence_text)

# Sequential
for tree in treesearch.read_trees_glob("data/*.conllu", parallel=False):
    print(tree.sentence_text)
```

## Gzip Support

Both `.conllu` and `.conllu.gz` files are automatically detected:

```python
# Works with both
for tree in treesearch.read_trees("corpus.conllu.gz"):
    print(tree.sentence_text)
```

## Accessing Tree Properties

See [Tree & Word API](../api/tree-word.md) for complete reference.

```python
for tree in treesearch.read_trees("corpus.conllu"):
    # Sentence text
    print(tree.sentence_text)

    # Metadata
    print(tree.metadata)

    # Word count
    print(len(tree))

    # Access words
    for i in range(len(tree)):
        word = tree.get_word(i)
        if word:
            print(f"{word.form}: {word.pos}")
```

## Next Steps

- [Searching Patterns](searching.md) - Search trees for patterns
- [API Reference](../api/functions.md) - Complete function documentation
