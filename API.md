# Treesearch API Reference

**Quick reference for querying dependency treebanks**

## Overview

Treesearch provides both an object-oriented and functional API for searching linguistic dependency trees using a pattern-matching query language. The typical workflow is:

1. Open a treebank with `open()` or create with `Treebank.from_*()`
2. Parse query string to `Pattern` with `parse_query()`
3. Search with `treebank.matches(pattern)` or iterate with `treebank.trees()`
4. Access matched nodes via the `Tree` and `Word` objects

## Basic Usage (Python)

### Object-Oriented API (Recommended)

```python
import treesearch as ts

# Parse your query once
query = """
    MATCH {
        Verb [upos="VERB"];
        Noun [upos="NOUN"];
        Verb -[nsubj]-> Noun;
    }
"""
pattern = ts.parse_query(query)

# Open a treebank (single file or glob pattern)
treebank = ts.open("corpus.conllu")

# Search for matches
for tree, match in treebank.matches(pattern):
    verb = tree.get_word(match["Verb"])
    noun = tree.get_word(match["Noun"])
    print(f"Match: {verb.form} ← {noun.form}")

# Multiple files with automatic parallel processing
treebank = ts.open("data/*.conllu")
for tree, match in treebank.matches(pattern):
    verb = tree.get_word(match["Verb"])
    print(f"Found: {verb.form}")

# Iterate over trees without searching
for tree in treebank.trees():
    print(f"Tree has {len(tree)} words")
```

### Functional API (Alternative)

```python
import treesearch as ts

pattern = ts.parse_query(query)

# Search a single file
for tree, match in ts.get_matches("corpus.conllu", pattern):
    verb = tree.get_word(match["Verb"])
    noun = tree.get_word(match["Noun"])
    print(f"Match: {verb.form} ← {noun.form}")

# Or iterate over trees
for tree in ts.get_trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        verb = tree.get_word(match["Verb"])
        noun = tree.get_word(match["Noun"])
        print(f"{verb.form} ← {noun.form}")
```

## Query Language

### Pattern Elements

Define nodes with constraints:

```
VariableName [constraint];
```

**Available constraints:**
- `upos="VERB"` - Universal part-of-speech tag
- `xpos="VBD"` - Language-specific POS tag
- `lemma="run"` - Lemma
- `form="running"` - Word form
- `deprel="nsubj"` - Dependency relation (to parent)
- `feats.Tense="Past"` - Morphological feature (dotted notation)

**Multiple constraints** (AND):
```
V [upos="VERB", lemma="be"];
```

**Empty constraint** (matches any node):
```
AnyNode [];
```

**Feature constraints**:
```
MATCH {
    # Past tense verb
    Verb [feats.Tense="Past"];
}

MATCH {
    # Plural nominative noun
    Noun [feats.Number="Plur", feats.Case="Nom"];
}
```

### Pattern Edges

Define relationships between nodes:

```
Parent -[deprel]-> Child;
```

**Dependency types:**
- `-[nsubj]->` - Specific dependency relation
- `->` - Any child (no relation specified)
- `!-[obj]->` - Negative constraint (does NOT have this edge)
- `!->` - Does NOT have any child

**Example patterns:**

```
MATCH {
    # VERB with nominal subject
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[nsubj]-> N;
}
```

```
MATCH {
    # Verb with xcomp (control verb)
    Main [upos="VERB"];
    Comp [upos="VERB"];
    Main -[xcomp]-> Comp;
}
```

```
MATCH {
    # Complex: VERB → NOUN → ADJ
    V [upos="VERB"];
    N [upos="NOUN"];
    A [upos="ADJ"];
    V -[obj]-> N;
    N -[amod]-> A;
}
```

## Python API Reference

### Treebank Class

#### `Treebank`

Represents a collection of dependency trees from one or more files. Supports multiple iterations and automatic parallel processing for multi-file treebanks.

**Class Methods:**

##### `Treebank.from_file(path: str) -> Treebank`

Create a treebank from a single CoNLL-U file (supports gzip).

```python
treebank = ts.Treebank.from_file("corpus.conllu")
```

##### `Treebank.from_files(paths: list[str]) -> Treebank`

Create a treebank from multiple CoNLL-U files.

```python
treebank = ts.Treebank.from_files(["file1.conllu", "file2.conllu"])
```

##### `Treebank.from_string(text: str) -> Treebank`

Create a treebank from a CoNLL-U string.

```python
conllu_text = """# text = Hello world.
1	Hello	hello	INTJ	_	_	0	root	_	_
2	world	world	NOUN	_	_	1	vocative	_	_
"""
treebank = ts.Treebank.from_string(conllu_text)
```

**Instance Methods:**

##### `trees(ordered: bool = True) -> Iterator[Tree]`

Iterate over all trees in the treebank. Can be called multiple times. Uses automatic parallel processing for multi-file treebanks.

**Parameters:**
- `ordered` (bool): If True (default), trees are returned in deterministic order. If False, trees may arrive in any order for better performance.

```python
# Ordered iteration (default)
for tree in treebank.trees():
    print(f"Tree has {len(tree)} words")
    print(f"Sentence: {tree.sentence_text}")

# Unordered for better performance
for tree in treebank.trees(ordered=False):
    print(f"Tree: {tree.sentence_text}")
```

##### `matches(pattern: Pattern, ordered: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]`

Search for pattern matches across all trees. Returns an iterator of (tree, match) tuples. Can be called multiple times. Uses automatic parallel processing for multi-file treebanks.

**Parameters:**
- `pattern` (Pattern): Compiled pattern from `parse_query()`
- `ordered` (bool): If True (default), matches are returned in deterministic order. If False, matches may arrive in any order for better performance.

```python
pattern = ts.parse_query("MATCH { Verb [upos=\"VERB\"]; }")

# Ordered iteration (default)
for tree, match in treebank.matches(pattern):
    verb = tree.get_word(match["Verb"])
    print(f"Found: {verb.form}")

# Unordered for better performance
for tree, match in treebank.matches(pattern, ordered=False):
    verb = tree.get_word(match["Verb"])
    print(f"Found: {verb.form}")
```

### Convenience Functions

#### `open(path: str) -> Treebank`

Smart function that automatically detects whether the path is a file or glob pattern and creates the appropriate Treebank.

```python
# Single file
tb = ts.open("corpus.conllu")

# Multiple files (automatically detected by * or ?)
tb = ts.open("data/*.conllu")

# Then use the treebank
for tree in tb.trees():
    print(tree.sentence_text)
```

#### `from_string(text: str) -> Treebank`

Convenience function for creating a treebank from a CoNLL-U string. Equivalent to `Treebank.from_string()`.

```python
conllu = """# text = Hello.
1	Hello	hello	INTJ	_	_	0	root	_	_
"""
tb = ts.from_string(conllu)
```

### Core Functions

#### `parse_query(query: str) -> Pattern`

Parse a query string into a Pattern object.

```python
pattern = ts.parse_query("""
    MATCH {
        Verb [upos="VERB"];
        Noun [upos="NOUN"];
        Verb -[nsubj]-> Noun;
    }
""")
```

#### `search(tree: Tree, pattern: Pattern) -> Iterator[dict[str, int]]`

Search a single tree for pattern matches. Returns an iterator of match dictionaries where keys are variable names and values are word IDs.

```python
for match in ts.search(tree, pattern):
    # match is a dictionary: {"Verb": 3, "Noun": 5}
    verb = tree.get_word(match["Verb"])
    noun = tree.get_word(match["Noun"])
    print(f"{verb.form} ← {noun.form}")
```

#### `get_trees(source: str, ordered: bool = True) -> Iterator[Tree]`

Read trees from one or more CoNLL-U files. Convenience wrapper for `open(source).trees(ordered)`.

**Parameters:**
- `source` (str): Path to a single file or glob pattern (e.g., "data/*.conllu")
- `ordered` (bool): If True (default), trees are returned in deterministic order

```python
# Single file
for tree in ts.get_trees("corpus.conllu"):
    print(f"Tree has {len(tree)} words")

# Multiple files with glob pattern
for tree in ts.get_trees("data/*.conllu"):
    print(f"Sentence: {tree.sentence_text}")

# Unordered for better performance
for tree in ts.get_trees("data/*.conllu", ordered=False):
    print(f"Tree: {tree.sentence_text}")
```

#### `get_matches(source: str, query: str | Pattern, ordered: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]`

Search one or more files for pattern matches. Convenience wrapper for `open(source).matches(pattern, ordered)`.

**Parameters:**
- `source` (str): Path to a single file or glob pattern (e.g., "data/*.conllu")
- `query` (str | Pattern): Query string or compiled Pattern
- `ordered` (bool): If True (default), matches are returned in deterministic order

```python
# Single file with query string
for tree, match in ts.get_matches("corpus.conllu", 'MATCH { V [upos="VERB"]; }'):
    verb = tree.get_word(match["V"])
    print(f"Found: {verb.form}")

# Multiple files with compiled pattern
pattern = ts.parse_query("MATCH { Verb [upos=\"VERB\"]; }")
for tree, match in ts.get_matches("data/*.conllu", pattern):
    verb = tree.get_word(match["Verb"])
    print(f"{verb.form}: {tree.sentence_text}")

# Unordered for better performance
for tree, match in ts.get_matches("data/*.conllu", pattern, ordered=False):
    verb = tree.get_word(match["Verb"])
    print(verb.form)
```

### Data Classes

#### `Tree`

Represents a dependency tree.

**Properties:**
- `sentence_text: str | None` - Reconstructed sentence text
- `metadata: dict[str, str]` - Tree metadata from CoNLL-U comments

**Methods:**
- `get_word(id: int) -> Word | None` - Get word by ID (1-indexed)
- `find_path(x: Word, y: Word) -> list[Word]` - Find dependency path between two words
- `__len__() -> int` - Number of words in tree

```python
tree = next(ts.read_trees("corpus.conllu"))
print(f"Sentence has {len(tree)} words")
print(f"Text: {tree.sentence_text}")

# Get specific word
word = tree.get_word(3)
if word:
    print(f"{word.id}: {word.form}")
```

#### `Word`

Represents a single word/node in the tree.

**Properties:**
- `id: int` - Word ID (0-based index in tree)
- `token_id: int` - Token ID from CoNLL-U (1-based)
- `form: str` - Word form
- `lemma: str` - Lemma
- `upos: str` - Universal POS tag
- `xpos: str` - Language-specific POS tag
- `deprel: str` - Dependency relation to parent
- `head: int | None` - Head word ID (0-based index, None for root)
- `feats: list[tuple[str, str]]` - Morphological features

**Methods:**
- `parent() -> Word | None` - Get parent word
- `children() -> list[Word]` - Get all children
- `children_by_deprel(deprel: str) -> list[Word]` - Get children with specific relation

```python
word = tree.get_word(5)
print(f"Form: {word.form}")
print(f"Lemma: {word.lemma}")
print(f"POS: {word.upos}")
print(f"DepRel: {word.deprel}")
print(f"Features: {word.feats}")

# Navigate tree
if word.parent():
    print(f"Parent: {word.parent().form}")
for child in word.children():
    print(f"Child: {child.form} ({child.deprel})")
```

#### `Pattern`

Represents a parsed query pattern. Created by `parse_query()`. Opaque object that can be reused across multiple searches.

```python
pattern = ts.parse_query("MATCH { Verb [upos=\"VERB\"]; }")

# Reuse pattern across multiple searches
for tree, match in ts.get_matches("data/*.conllu", pattern):
    verb = tree.get_word(match["Verb"])
    print(f"Found: {verb.form}")
```

## Complete Example

```python
import treesearch as ts

# Find all control verbs (VERB with VERB xcomp)
query = """
MATCH {
    Main [upos="VERB"];
    Comp [upos="VERB"];
    Main -[xcomp]-> Comp;
}
"""

# Parse query once
pattern = ts.parse_query(query)

# Open treebank and search for matches
treebank = ts.open("corpus.conllu")
for tree, match in treebank.matches(pattern):
    main = tree.get_word(match["Main"])
    comp = tree.get_word(match["Comp"])
    print(f"  Main = {main.form} (lemma: {main.lemma})")
    print(f"  Comp = {comp.form} (lemma: {comp.lemma})")
    print(f"  Sentence: {tree.sentence_text}")
    print()

# Or use functional API
for tree, match in ts.get_matches("corpus.conllu", pattern):
    main = tree.get_word(match["Main"])
    comp = tree.get_word(match["Comp"])
    print(f"Match: {main.form} -[xcomp]-> {comp.form}")

# Or iterate trees manually
for tree in treebank.trees():
    for match in ts.search(tree, pattern):
        main = tree.get_word(match["Main"])
        comp = tree.get_word(match["Comp"])
        print(f"{main.form} → {comp.form}")
```

## Error Handling

All operations raise Python exceptions on error:

```python
try:
    # Parse errors
    pattern = ts.parse_query("Invalid [syntax")
except Exception as e:
    print(f"Query parse error: {e}")

try:
    # File not found
    for tree in ts.get_trees("nonexistent.conllu"):
        pass
except Exception as e:
    print(f"File error: {e}")

try:
    # Malformed CoNLL-U
    for tree in ts.get_trees("bad_format.conllu"):
        pass
except Exception as e:
    print(f"Parse error: {e}")
```

## Performance Tips

- **Parse queries once**: `Pattern` objects are reusable across searches
- **Exhaustive search**: Finds ALL matches, not just first/leftmost
- **Automatic parallel processing**: Multi-file operations automatically process files in parallel for better performance
- **Memory efficient**: Iterator-based API streams results without loading entire corpus
- **Use gzipped files**: Store CoNLL-U files as `.conllu.gz` to reduce I/O time and disk usage (decompression is automatic)
- **Unordered iteration**: Use `ordered=False` for better performance when order doesn't matter

### Parallel Processing

Multi-file operations (via `open()` with glob patterns, or `get_trees()`/`get_matches()`) automatically process files in parallel using bounded channels and rayon for optimal throughput.

```python
# Automatic parallel file reading
for tree in ts.get_trees("data/*.conllu"):
    # Trees from different files processed in parallel automatically
    pass

# Automatic parallel search across files
pattern = ts.parse_query("MATCH { Verb [upos=\"VERB\"]; }")
for tree, match in ts.get_matches("data/*.conllu", pattern):
    # Searches run in parallel across files automatically
    verb = tree.get_word(match["Verb"])
    print(verb.form)

# Unordered for maximum performance
for tree, match in ts.get_matches("data/*.conllu", pattern, ordered=False):
    # Even faster when order doesn't matter
    verb = tree.get_word(match["Verb"])
    print(verb.form)
```

### Best Practices

```python
# ✅ GOOD: Parse query once, reuse
pattern = ts.parse_query(query_string)
for tree in ts.get_trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        # Process match
        pass

# ❌ BAD: Re-parsing query every iteration
for tree in ts.get_trees("corpus.conllu"):
    pattern = ts.parse_query(query_string)  # Wasteful!
    for match in ts.search(tree, pattern):
        # Process match
        pass

# ✅ GOOD: Use get_matches for simple cases
for tree, match in ts.get_matches("corpus.conllu", pattern):
    # Convenient and efficient
    verb = tree.get_word(match["Verb"])

# ✅ GOOD: Multi-file operations use automatic parallel processing
for tree, match in ts.get_matches("data/*.conllu", pattern):
    # Parallel processing happens automatically
    verb = tree.get_word(match["Verb"])

# ✅ GOOD: Use ordered=False when order doesn't matter
for tree, match in ts.get_matches("data/*.conllu", pattern, ordered=False):
    # Maximum performance
    verb = tree.get_word(match["Verb"])
```
