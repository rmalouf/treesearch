# Standalone functions

Functional interface for treesearch operations.

## Overview

Treesearch provides standalone functions for pattern compilation, searching, and reading treebanks. These functions offer a functional alternative to the object-oriented Treebank API.

Functions are organized into three categories:

- **Pattern compilation** - parse_query()
- **Searching** - search(), search_file(), search_files()
- **Reading trees** - read_trees(), read_trees_glob()

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

for tree in treesearch.read_trees("corpus.conllu"):
    matches = treesearch.search(tree, pattern)
    for match in matches:
        verb = tree.get_word(match["V"])
        print(f"Found: {verb.form}")
```

**Notes:**

- Returns ALL matches (exhaustive search)
- Returns list, not iterator (all matches computed immediately)
- Word IDs are 0-based indices

**See also:** search_file(), search_files()

---

### search_file()

Search a single CoNLL-U file for pattern matches.

```python
treesearch.search_file(
    path: str,
    pattern: Pattern,
    ordered: bool = True
) -> Iterator[tuple[Tree, dict[str, int]]]
```

**Parameters:**

- `path` (str) - Path to CoNLL-U file
- `pattern` (Pattern) - Compiled pattern from parse_query()
- `ordered` (bool) - If True (default), return matches in file order. Has no effect for single files.

**Returns:**

- Iterator yielding (tree, match) tuples

**Raises:**

- `ValueError` - If file cannot be opened or parsed

**Example:**

```python
import treesearch

pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Subj [];
        V <-[nsubj]- Subj;
    }
""")

for tree, match in treesearch.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    subj = tree.get_word(match["Subj"])
    print(f"{subj.form} {verb.form}: {tree.sentence_text}")
```

**Notes:**

- Automatically handles gzip-compressed files (`.conllu.gz`)
- More efficient than manually calling read_trees() + search()
- Streams results without loading entire file into memory

**See also:** search_files(), read_trees()

---

### search_files()

Search multiple CoNLL-U files for pattern matches.

```python
treesearch.search_files(
    glob_pattern: str,
    pattern: Pattern,
    ordered: bool = True
) -> Iterator[tuple[Tree, dict[str, int]]]
```

**Parameters:**

- `glob_pattern` (str) - Glob pattern to match files (e.g., `"data/*.conllu"`)
- `pattern` (Pattern) - Compiled pattern from parse_query()
- `ordered` (bool) - If True (default), return matches in deterministic file order. If False, matches may arrive in any order for better performance.

**Returns:**

- Iterator yielding (tree, match) tuples

**Raises:**

- `ValueError` - If glob pattern is invalid

**Example:**

```python
import treesearch

# Find passive constructions across corpus
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V <-[aux:pass]- Aux;
    }
""")

# Deterministic order (default)
for tree, match in treesearch.search_files("data/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"Passive: {tree.sentence_text}")

# Faster, non-deterministic order
for tree, match in treesearch.search_files("data/*.conllu", pattern, ordered=False):
    process(tree, match)
```

**Notes:**

- Most efficient way to search large corpora
- Automatically uses parallel processing
- Handles both `.conllu` and `.conllu.gz` files
- With `ordered=True`, files are processed in sorted order
- With `ordered=False`, files are processed in parallel with non-deterministic order for better performance
- Failed files print warnings but don't stop iteration

**See also:** search_file(), read_trees_glob()

---

## Reading functions

### read_trees()

Read trees from a CoNLL-U file.

```python
treesearch.read_trees(
    path: str,
    ordered: bool = True
) -> Iterator[Tree]
```

**Parameters:**

- `path` (str) - Path to CoNLL-U file (supports `.conllu` and `.conllu.gz`)
- `ordered` (bool) - If True (default), return trees in file order. Has no effect for single files.

**Returns:**

- Iterator yielding Tree objects

**Raises:**

- `ValueError` - If file cannot be opened or parsed

**Example:**

```python
import treesearch

for tree in treesearch.read_trees("corpus.conllu"):
    print(f"Sentence: {tree.sentence_text}")
    print(f"Words: {len(tree)}")
```

**Notes:**

- Automatically detects and decompresses gzip files
- Streams trees without loading entire file into memory
- Errors in CoNLL-U format are reported with line numbers

**See also:** read_trees_glob(), search_file()

---

### read_trees_glob()

Read trees from multiple CoNLL-U files.

```python
treesearch.read_trees_glob(
    glob_pattern: str,
    ordered: bool = True
) -> Iterator[Tree]
```

**Parameters:**

- `glob_pattern` (str) - Glob pattern to match files (e.g., `"data/*.conllu"`)
- `ordered` (bool) - If True (default), return trees in deterministic file order. If False, trees may arrive in any order for better performance.

**Returns:**

- Iterator yielding Tree objects from all matching files

**Raises:**

- `ValueError` - If glob pattern is invalid

**Example:**

```python
import treesearch

# Read from multiple files (deterministic order)
for tree in treesearch.read_trees_glob("corpus/*.conllu"):
    print(tree.sentence_text)

# Faster, non-deterministic order
for tree in treesearch.read_trees_glob("corpus/*.conllu", ordered=False):
    process(tree)
```

**Notes:**

- Automatically uses parallel processing
- With `ordered=True`, files are processed in sorted order
- With `ordered=False`, uses parallel processing with non-deterministic order for better performance
- Failed files print warnings but don't stop iteration
- Handles both `.conllu` and `.conllu.gz` files

**See also:** read_trees(), search_files()

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
for tree, match in treesearch.search_files("data/*.conllu", pattern):
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

for tree, match in treesearch.search_file("corpus.conllu", pattern):
    main = tree.get_word(match["Main"])
    inf = tree.get_word(match["Inf"])
    print(f"{main.form} ... to {inf.form}: {tree.sentence_text}")
```

### Reading and analyzing trees

```python
import treesearch

# Count POS tags across corpus
pos_counts = {}

for tree in treesearch.read_trees_glob("data/*.conllu"):
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
for tree, match in treesearch.search_files("data/*.conllu", pattern, ordered=True):
    count1 += 1

# Non-deterministic, faster parallel processing
count2 = 0
for tree, match in treesearch.search_files("data/*.conllu", pattern, ordered=False):
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
    for tree, match in treesearch.search_file("missing.conllu", pattern):
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
    for tree, match in treesearch.search_file(file_path, pattern):
        process(match)
```

---

## Function comparison

| Function | Input | Returns | Best For |
|----------|-------|---------|----------|
| `parse_query()` | Query string | Pattern | Compiling queries |
| `search()` | Single tree + pattern | List of matches | In-memory trees |
| `search_file()` | File path + pattern | (Tree, match) iterator | Single file search |
| `search_files()` | Glob pattern + pattern | (Tree, match) iterator | **Large corpus search** |
| `read_trees()` | File path | Tree iterator | Reading single file |
| `read_trees_glob()` | Glob pattern | Tree iterator | Reading multiple files |

---

## Performance tips

### Compile patterns once

```python
# Good: Parse once, reuse many times
pattern = treesearch.parse_query(query)
for tree, match in treesearch.search_files("*.conllu", pattern):
    process(match)

# Bad: Re-parsing every iteration
for tree, match in treesearch.search_files("*.conllu", treesearch.parse_query(query)):
    process(match)
```

### Use search_files() for large corpora

```python
# Best: Direct search with automatic parallelization
for tree, match in treesearch.search_files("data/*.conllu", pattern):
    process(tree, match)

# Slower: Manual iteration
for tree in treesearch.read_trees_glob("data/*.conllu"):
    for match in treesearch.search(tree, pattern):
        process(tree, match)
```

### Use ordered=False for faster processing

```python
# When result order doesn't matter
results = []
for tree, match in treesearch.search_files("data/*.conllu", pattern, ordered=False):
    results.append((tree, match))
```

---

## See also

- [Treebank](treebank.md) - Object-oriented API
- [Pattern](pattern.md) - Pattern class
- [Tree & Word](tree-word.md) - Tree and Word classes
- [Query language](../guide/query-language.md) - Query syntax reference
