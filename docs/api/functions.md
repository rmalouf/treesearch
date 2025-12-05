# Functions API Reference

Complete reference for all treesearch functions.

## Core Functions

### parse_query()

Parse a query string into a compiled pattern.

```python
treesearch.parse_query(query: str) -> Pattern
```

**Parameters:**

- `query` (str): Query string with variable declarations and constraints

**Returns:**

- `Pattern`: Compiled pattern object

**Raises:**

- `ValueError`: If query syntax is invalid

**Example:**

```python
pattern = treesearch.parse_query("""
MATCH {
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[obj]-> N;
}
""")
```

**Notes:**

- Parse patterns once and reuse for better performance
- Patterns are thread-safe and can be shared
- See [Query Language](../guide/query-language.md) for syntax

---

### search()

Search a single tree for pattern matches.

```python
treesearch.search(tree: Tree, pattern: Pattern) -> Iterator[dict[str, int]]
```

**Parameters:**

- `tree` (Tree): Tree to search
- `pattern` (Pattern): Compiled pattern from `parse_query()`

**Returns:**

- Iterator of match dictionaries. Each dict maps variable names to word IDs.

**Example:**

```python
for tree in treesearch.read_trees("corpus.conllu"):
    for match in treesearch.search(tree, pattern):
        verb = tree.get_word(match["V"])
        noun = tree.get_word(match["N"])
        print(f"{verb.form} -> {noun.form}")
```

**Notes:**

- Returns ALL matches (exhaustive search)
- Matches are dictionaries: `{"V": 3, "N": 7}`
- Word IDs are 0-based indices

---

### read_trees()

Read trees from a CoNLL-U file.

```python
treesearch.read_trees(path: str) -> Iterator[Tree]
```

**Parameters:**

- `path` (str): Path to CoNLL-U file (supports `.conllu` and `.conllu.gz`)

**Returns:**

- Iterator yielding Tree objects

**Raises:**

- `ValueError`: If file cannot be opened

**Example:**

```python
for tree in treesearch.read_trees("corpus.conllu"):
    print(f"Sentence: {tree.sentence_text}")
    print(f"Words: {len(tree)}")
```

**Notes:**

- Automatically detects and decompresses gzip files
- Streams trees without loading entire file into memory
- Errors in CoNLL-U format are reported with line numbers

---

### search_file()

Search a single CoNLL-U file for pattern matches.

```python
treesearch.search_file(path: str, pattern: Pattern) -> Iterator[tuple[Tree, dict[str, int]]]
```

**Parameters:**

- `path` (str): Path to CoNLL-U file
- `pattern` (Pattern): Compiled pattern

**Returns:**

- Iterator yielding (tree, match) tuples

**Raises:**

- `ValueError`: If file cannot be opened

**Example:**

```python
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"Found: {verb.form} in '{tree.sentence_text}'")
```

**Notes:**

- More efficient than manually calling `read_trees()` + `search()`
- Automatically handles gzip compression
- Streams results without loading entire file

---

### read_trees_glob()

Read trees from multiple CoNLL-U files.

```python
treesearch.read_trees_glob(
    glob_pattern: str
) -> Iterator[Tree]
```

**Parameters:**

- `glob_pattern` (str): Glob pattern to match files (e.g., `"data/*.conllu"`)

**Returns:**

- Iterator yielding Tree objects from all matching files

**Raises:**

- `ValueError`: If glob pattern is invalid

**Example:**

```python
# Automatic parallel processing
for tree in treesearch.read_trees_glob("corpus/*.conllu"):
    print(tree.sentence_text)
```

**Notes:**

- Automatically uses parallel processing for better performance
- Order of results is not deterministic due to parallel processing
- Failed files print warnings but don't stop iteration

---

### search_files()

Search multiple CoNLL-U files for pattern matches.

```python
treesearch.search_files(
    glob_pattern: str,
    pattern: Pattern
) -> Iterator[tuple[Tree, dict[str, int]]]
```

**Parameters:**

- `glob_pattern` (str): Glob pattern to match files
- `pattern` (Pattern): Compiled pattern

**Returns:**

- Iterator yielding (tree, match) tuples

**Raises:**

- `ValueError`: If glob pattern is invalid

**Example:**

```python
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

# Automatic parallel search
for tree, match in treesearch.search_files("data/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

**Notes:**

- **Most efficient** way to search large corpora
- Automatically uses parallel processing for better performance
- Handles both `.conllu` and `.conllu.gz` files
- Failed files print warnings but don't stop iteration

---

## Function Comparison

| Function | Input | Returns | Best For |
|----------|-------|---------|----------|
| `parse_query()` | Query string | Pattern | Compiling queries |
| `search()` | Single tree | Matches | In-memory trees |
| `read_trees()` | File path | Trees | Reading corpus |
| `search_file()` | File + pattern | (Tree, match) tuples | Single file search |
| `read_trees_glob()` | Glob pattern | Trees | Multi-file reading |
| `search_files()` | Glob + pattern | (Tree, match) tuples | **Large corpus search** |

## Performance Tips

### Parse Once, Search Many

```python
# Good: Parse once
pattern = treesearch.parse_query(query)
for tree, match in treesearch.search_files("*.conllu", pattern):
    process(match)

# Bad: Re-parsing every iteration
for tree, match in treesearch.search_files("*.conllu", treesearch.parse_query(query)):
    process(match)
```

### Use search_files() for Large Corpora

```python
# Best: Direct search with automatic parallelization
for tree, match in treesearch.search_files("data/*.conllu", pattern):
    process(tree, match)

# Slower: Manual iteration
for tree in treesearch.read_trees_glob("data/*.conllu"):
    for match in treesearch.search(tree, pattern):
        process(tree, match)
```

**Note:** Multi-file operations use automatic parallel processing. Results may not be in deterministic file order due to parallelization.

## Error Handling

```python
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

## Next Steps

- [Tree & Word API](tree-word.md) - Working with results
- [Pattern API](pattern.md) - Pattern object reference
- [Working with Results](../guide/results.md) - Practical examples
