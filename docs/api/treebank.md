# Treebank class

A collection of dependency trees from files or strings.

## Overview

The Treebank class represents a collection of dependency trees. Treebanks are created from CoNLL-U files, glob patterns, or strings, and provide methods for iterating over trees and searching for pattern matches.

Treebanks use automatic parallel processing when working with multiple files for better performance.

## Instance methods

### trees()

Iterate over all trees in the treebank.

```python
treebank.trees(ordered: bool = True) -> Iterator[Tree]
```

**Parameters:**

- `ordered` (bool) - If True (default), return trees in corpus order. If False, trees may arrive in any order for better performance.

**Returns:**

- Iterator over Tree objects

**Example:**

```python
tb = treesearch.load("data/*.conllu")

# Deterministic order (default)
for tree in tb.trees():
    print(tree.sentence_text)

# Faster, non-deterministic order
for tree in tb.trees(ordered=False):
    print(tree.sentence_text)
```

**See also:** search()

---

### search()

Search for pattern matches across all trees.

```python
treebank.search(pattern: Pattern, ordered: bool = True) -> Iterator[Match]
```

**Parameters:**

- `pattern` (Pattern) - Compiled pattern from parse_query()
- `ordered` (bool) - If True (default), return matches in deterministic order. If False, matches may arrive in any order for better performance.

**Returns:**

- Iterator yielding (tree, match) tuples where match is a dictionary mapping variable names to word IDs

**Example:**

```python
tb = treesearch.load("data/*.conllu")
pattern = treesearch.compile_query("""
    MATCH {
        V [upos="VERB"];
        Subj [upos="NOUN"];
        V <-[nsubj]- Subj;
    }
""")

# Deterministic order (default)
for tree, match in tb.matches(pattern):
    verb = tree[match["V"]]
    subj = tree[match["Subj"]]
    print(f"{subj.form} {verb.form}: {tree.sentence_text}")

# Faster, non-deterministic order
for tree, match in tb.search(pattern, ordered=False):
    process(tree, match)
```

**See also:** trees()

## See also

- [Tree](tree-word.md#tree) - Tree class for individual dependency trees
- [Pattern](pattern.md) - Pattern class for compiled queries
- [search()](functions.md#search) - Functional interface for single file
