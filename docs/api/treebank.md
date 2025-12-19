# Treebank class

A collection of dependency trees from files or strings.

## Overview

The Treebank class represents a collection of dependency trees that can be iterated multiple times. Treebanks are created from CoNLL-U files, glob patterns, or strings, and provide methods for iterating over trees and searching for pattern matches.

Treebanks use automatic parallel processing when working with multiple files for better performance.

## Class methods

### from_string()

Create a treebank from a CoNLL-U string.

```python
Treebank.from_string(text: str) -> Treebank
```

**Parameters:**

- `text` (str) - CoNLL-U formatted text

**Returns:**

- Treebank instance

**Example:**

```python
conllu_data = """
# sent_id = 1
# text = She runs.
1   She    she    PRON   ...
2   runs   run    VERB   ...

"""

tb = treesearch.Treebank.from_string(conllu_data)
for tree in tb.trees():
    print(tree.sentence_text)
```

**See also:** from_file(), from_glob()

---

### from_file()

Create a treebank from a CoNLL-U file.

```python
Treebank.from_file(path: str) -> Treebank
```

**Parameters:**

- `path` (str) - Path to CoNLL-U file

**Returns:**

- Treebank instance

**Example:**

```python
tb = treesearch.Treebank.from_file("corpus.conllu")
for tree in tb.trees():
    print(f"{tree.sentence_text} ({len(tree)} words)")
```

**Notes:**

- Automatically detects and handles gzip-compressed files (`.conllu.gz`)
- File is not loaded into memory until iteration begins
- Can iterate multiple times

**See also:** from_glob(), from_string()

---

### from_glob()

Create a treebank from multiple files matching a glob pattern.

```python
Treebank.from_glob(pattern: str) -> Treebank
```

**Parameters:**

- `pattern` (str) - Glob pattern to match files (e.g., `"data/*.conllu"`)

**Returns:**

- Treebank instance

**Raises:**

- `ValueError` - If glob pattern is invalid

**Example:**

```python
# Single directory
tb = treesearch.Treebank.from_glob("corpus/*.conllu")

# Multiple directories
tb = treesearch.Treebank.from_glob("data/**/*.conllu")

# Compressed files
tb = treesearch.Treebank.from_glob("corpus/*.conllu.gz")
```

**Notes:**

- Files are processed in sorted order by default
- Supports both `.conllu` and `.conllu.gz` files
- Uses automatic parallel processing
- Can iterate multiple times

**See also:** from_file(), from_string()

---

## Instance methods

### trees()

Iterate over all trees in the treebank.

```python
treebank.trees(ordered: bool = True) -> Iterator[Tree]
```

**Parameters:**

- `ordered` (bool) - If True (default), return trees in deterministic order. If False, trees may arrive in any order for better performance.

**Returns:**

- Iterator over Tree objects

**Example:**

```python
tb = treesearch.Treebank.from_glob("data/*.conllu")

# Deterministic order (default)
for tree in tb.trees():
    print(tree.sentence_text)

# Faster, non-deterministic order
for tree in tb.trees(ordered=False):
    print(tree.sentence_text)
```

**Notes:**

- Can be called multiple times on the same treebank
- For single-file treebanks, `ordered` has no effect
- For multi-file treebanks, `ordered=True` processes files sequentially
- For multi-file treebanks, `ordered=False` uses parallel processing with non-deterministic order

**See also:** matches()

---

### matches()

Search for pattern matches across all trees.

```python
treebank.matches(pattern: Pattern, ordered: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]
```

**Parameters:**

- `pattern` (Pattern) - Compiled pattern from parse_query()
- `ordered` (bool) - If True (default), return matches in deterministic order. If False, matches may arrive in any order for better performance.

**Returns:**

- Iterator yielding (tree, match) tuples where match is a dictionary mapping variable names to word IDs

**Example:**

```python
tb = treesearch.Treebank.from_glob("data/*.conllu")
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Subj [upos="NOUN"];
        V <-[nsubj]- Subj;
    }
""")

# Deterministic order (default)
for tree, match in tb.matches(pattern):
    verb = tree.get_word(match["V"])
    subj = tree.get_word(match["Subj"])
    print(f"{subj.form} {verb.form}: {tree.sentence_text}")

# Faster, non-deterministic order
for tree, match in tb.matches(pattern, ordered=False):
    process(tree, match)
```

**Notes:**

- Can be called multiple times on the same treebank
- Returns ALL matches (exhaustive search)
- For multi-file treebanks, uses automatic parallel processing
- For multi-file treebanks, `ordered=False` provides better performance

**See also:** trees()

---

## Examples

### Multiple iterations

```python
tb = treesearch.Treebank.from_file("corpus.conllu")

# First iteration
count = sum(1 for tree in tb.trees())
print(f"Total trees: {count}")

# Second iteration (reuses same treebank)
for tree in tb.trees():
    analyze(tree)
```

### Performance tuning

```python
tb = treesearch.Treebank.from_glob("large-corpus/*.conllu")
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

# Deterministic order (slower, reproducible)
results = list(tb.matches(pattern, ordered=True))

# Non-deterministic order (faster, non-reproducible)
results = list(tb.matches(pattern, ordered=False))
```

### Different input sources

```python
# From string
conllu = """# text = She runs.\n1\tShe\t...\n2\truns\t...\n"""
tb1 = treesearch.Treebank.from_string(conllu)

# From single file
tb2 = treesearch.Treebank.from_file("corpus.conllu")

# From multiple files
tb3 = treesearch.Treebank.from_glob("data/*.conllu")

# All support same methods
for tb in [tb1, tb2, tb3]:
    for tree in tb.trees():
        print(tree.sentence_text)
```

### Combining with pattern search

```python
tb = treesearch.Treebank.from_glob("data/*.conllu")

# Find passive constructions
passive_pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V <-[aux:pass]- Aux;
    }
""")

for tree, match in tb.matches(passive_pattern):
    verb = tree.get_word(match["V"])
    print(f"Passive: {tree.sentence_text}")
```

## See also

- [Tree](tree-word.md#tree) - Tree class for individual dependency trees
- [Pattern](pattern.md) - Pattern class for compiled queries
- [search_file()](functions.md#search_file) - Functional interface for single file
- [search_files()](functions.md#search_files) - Functional interface for multiple files
