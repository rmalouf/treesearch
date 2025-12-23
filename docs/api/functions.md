# Standalone functions

Functional interface for treesearch operations.

## Overview

Treesearch provides standalone functions for pattern compilation, searching, and reading treebanks. These functions offer a functional alternative to the object-oriented Treebank API.

Functions are organized into three categories:

- **Pattern compilation** - parse_query()
- **Searching** - search(), get_matches()
- **Reading trees** - get_trees()

## Pattern compilation

### parse_query()

Compile a query string into a Pattern object.

```python
treesearch.parse_query(query: str) -> Pattern
```

**Parameters:**

- `query` (str) - Query string in treesearch query language

**Returns:**

- Pattern object for use with search functions

**Raises:**

- `ValueError` - If query syntax is invalid

**Example:**

```python
import treesearch

pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        N [upos="NOUN"];
        V -[obj]-> N;
    }
""")
```

**Notes:**

- Patterns are reusable and thread-safe
- Parse once and reuse for better performance
- Patterns cannot be modified after creation

**See also:** [Pattern](pattern.md), [Query language guide](../guide/query-language.md)

---

## Searching functions

### search()

Search a single tree for pattern matches.

```python
treesearch.search(tree: Tree, pattern: Pattern) -> list[dict[str, int]]
```

**Parameters:**

- `tree` (Tree) - Tree to search
- `pattern` (Pattern) - Compiled pattern from parse_query()

**Returns:**

- List of match dictionaries. Each dictionary maps variable names to word IDs (0-based).

**Example:**

```python
import treesearch

pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

for tree in treesearch.trees("corpus.conllu"):
    for match in treesearch.search(tree, pattern):
        verb = tree.get_word(match["V"])
        print(f"Found: {verb.form}")
```

**Notes:**

- Returns ALL matches (exhaustive search)
- Returns iterator of matches
- Word IDs are 0-based indices

**See also:** get_matches()

---

### get_matches()

Search one or more CoNLL-U files for pattern matches.

```python
treesearch.search(
    source: str,
query: str | Pattern,
ordered: bool = True
) -> Iterator[tuple[Tree, dict[str, int]]]
```

**Parameters:**

- `source` (str) - Path to a single file or glob pattern (e.g., `"data/*.conllu"`)
- `query` (str | Pattern) - Query string or compiled Pattern from parse_query()
- `ordered` (bool) - If True (default), return matches in deterministic order. If False, matches may arrive in any order for better performance.

**Returns:**

- Iterator yielding (tree, match) tuples

**Raises:**

- `ValueError` - If file cannot be opened, parsed, or glob pattern is invalid

**Example:**

```python
import treesearch

# Single file with query string
for tree, match in treesearch.search("corpus.conllu", 'MATCH { V [upos="VERB"]; }'):
    verb = tree.get_word(match["V"])
    print(f"Found: {verb.form}")

# Multiple files with compiled pattern
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Subj [];
        V <-[nsubj]- Subj;
    }
""")

for tree, match in treesearch.search("data/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    subj = tree.get_word(match["Subj"])
    print(f"{subj.form} {verb.form}: {tree.sentence_text}")

# Faster, non-deterministic order
for tree, match in treesearch.search("data/*.conllu", pattern, ordered=False):
    process(tree, match)
```

**Notes:**

- Automatically handles gzip-compressed files (`.conllu.gz`)
- Auto-detects single file vs glob pattern
- Automatically uses parallel processing for multiple files
- With `ordered=True`, files are processed in sorted order
- With `ordered=False`, uses parallel processing with non-deterministic order for better performance
- Streams results without loading entire corpus into memory
- Can accept query string directly (auto-compiles pattern)

**See also:** get_trees(), search()

---

## Reading functions

### get_trees()

Read trees from one or more CoNLL-U files.

```python
treesearch.trees(
    source: str,
ordered: bool = True
) -> Iterator[Tree]
```

**Parameters:**

- `source` (str) - Path to a single file or glob pattern (e.g., `"data/*.conllu"`)
- `ordered` (bool) - If True (default), return trees in deterministic order. If False, trees may arrive in any order for better performance.

**Returns:**

- Iterator yielding Tree objects

**Raises:**

- `ValueError` - If file cannot be opened, parsed, or glob pattern is invalid

**Example:**

```python
import treesearch

# Single file
for tree in treesearch.trees("corpus.conllu"):
    print(f"Sentence: {tree.sentence_text}")
    print(f"Words: {len(tree)}")

# Multiple files (deterministic order)
for tree in treesearch.trees("corpus/*.conllu"):
    print(tree.sentence_text)

# Faster, non-deterministic order
for tree in treesearch.trees("corpus/*.conllu", ordered=False):
    process(tree)
```

**Notes:**

- Automatically detects and decompresses gzip files
- Auto-detects single file vs glob pattern
- Automatically uses parallel processing for multiple files
- With `ordered=True`, files are processed in sorted order
- With `ordered=False`, uses parallel processing with non-deterministic order for better performance
- Streams trees without loading entire corpus into memory
- Errors in CoNLL-U format are reported with line numbers

**See also:** get_matches()

---

## Examples

### Basic search workflow

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

# Search files
for tree, match in treesearch.search("data/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    noun = tree.get_word(match["N"])
    print(f"{verb.form} -> {noun.form}")
```

### Finding control constructions

```python
import treesearch

# Find help-to-infinitive constructions
pattern = treesearch.parse_query("""
    MATCH {
        Main [lemma="help"];
        Inf [upos="VERB"];
        Main -[xcomp]-> Inf;
    }
""")

for tree, match in treesearch.search("corpus.conllu", pattern):
    main = tree.get_word(match["Main"])
    inf = tree.get_word(match["Inf"])
    print(f"{main.form} ... to {inf.form}: {tree.sentence_text}")
```

### Reading and analyzing trees

```python
import treesearch

# Count POS tags across corpus
pos_counts = {}

for tree in treesearch.trees("data/*.conllu"):
    for word_id in range(len(tree)):
        word = tree.get_word(word_id)
        if word:
            pos_counts[word.pos] = pos_counts.get(word.pos, 0) + 1

for pos, count in sorted(pos_counts.items()):
    print(f"{pos}: {count}")
```

### Performance tuning with ordered parameter

```python
import treesearch

pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

# Deterministic, reproducible (slower for multi-file)
count1 = 0
for tree, match in treesearch.search("data/*.conllu", pattern, ordered=True):
    count1 += 1

# Non-deterministic, faster parallel processing
count2 = 0
for tree, match in treesearch.search("data/*.conllu", pattern, ordered=False):
    count2 += 1

# Both counts are the same, but ordered=False is faster
assert count1 == count2
```

### Error handling

```python
import treesearch

try:
    pattern = treesearch.parse_query("V [invalid syntax")
except ValueError as e:
    print(f"Query error: {e}")

try:
    for tree, match in treesearch.search("missing.conllu", pattern):
        pass
except ValueError as e:
    print(f"File error: {e}")
```

### Pattern reuse

```python
import treesearch

# Compile once
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

# Reuse across multiple files
files = ["file1.conllu", "file2.conllu", "file3.conllu"]
for file_path in files:
    for tree, match in treesearch.search(file_path, pattern):
        process(match)
```

### Using query strings directly

```python
import treesearch

# get_matches() accepts query strings directly
for tree, match in treesearch.search("corpus.conllu", 'MATCH { V [upos="VERB"]; }'):
    verb = tree.get_word(match["V"])
    print(verb.form)

# But compiling once is more efficient for repeated use
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')
for file in ["file1.conllu", "file2.conllu"]:
    for tree, match in treesearch.search(file, pattern):
        process(match)
```

---

## Function comparison

| Function | Input | Returns | Best For |
|----------|-------|---------|----------|
| `parse_query()` | Query string | Pattern | Compiling queries |
| `search()` | Single tree + pattern | Match iterator | In-memory trees |
| `get_matches()` | File/glob + query/pattern | (Tree, match) iterator | **Searching files** |
| `get_trees()` | File/glob | Tree iterator | Reading files |

---

## Performance tips

### Compile patterns once

```python
# Good: Parse once, reuse many times
pattern = treesearch.parse_query(query)
for tree, match in treesearch.search("*.conllu", pattern):
    process(match)

# Acceptable: Query string auto-compiled (less efficient for repeated use)
for tree, match in treesearch.search("corpus.conllu", 'MATCH { V [upos="VERB"]; }'):
    process(match)

# Bad: Re-parsing explicitly every file
for file in files:
    pattern = treesearch.parse_query(query)  # Wasteful!
    for tree, match in treesearch.search(file, pattern):
        process(match)
```

### Use get_matches() for large corpora

```python
# Best: Direct search with automatic parallelization
for tree, match in treesearch.search("data/*.conllu", pattern):
    process(tree, match)

# Slower: Manual iteration
for tree in treesearch.trees("data/*.conllu"):
    for match in treesearch.search(tree, pattern):
        process(tree, match)
```

### Use ordered=False for faster processing

```python
# When result order doesn't matter
results = []
for tree, match in treesearch.search("data/*.conllu", pattern, ordered=False):
    results.append((tree, match))
```

---

## See also

- [Treebank](treebank.md) - Object-oriented API
- [Pattern](pattern.md) - Pattern class
- [Tree & Word](tree-word.md) - Tree and Word classes
- [Query language](../guide/query-language.md) - Query syntax reference
