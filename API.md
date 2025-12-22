# Treesearch API Reference

**Quick reference for querying dependency treebanks**

## Overview

Treesearch provides both an object-oriented and functional API for searching linguistic dependency trees using a pattern-matching query language. The typical workflow is:

1. Load a treebank with `load()` or create with `Treebank.from_*()`
2. Compile query string to `Pattern` with `compile_query()`
3. Search with `treebank.search(pattern)` or iterate with `treebank.trees()`
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
pattern = ts.compile_query(query)

# Open a treebank (single file or glob pattern)
treebank = ts.load("corpus.conllu")

# Search for matches
for tree, match in treebank.search(pattern):
    verb = tree.word(match["Verb"])
    noun = tree.word(match["Noun"])
    print(f"Match: {verb.form} ← {noun.form}")

# Multiple files with automatic parallel processing
treebank = ts.load("data/*.conllu")
for tree, match in treebank.search(pattern):
    verb = tree.word(match["Verb"])
    print(f"Found: {verb.form}")

# Iterate over trees without searching
for tree in treebank.trees():
    print(f"Tree has {len(tree)} words")
```

### Functional API (Alternative)

```python
import treesearch as ts

pattern = ts.compile_query(query)

# Search a single file
for tree, match in ts.search("corpus.conllu", pattern):
    verb = tree.word(match["Verb"])
    noun = tree.word(match["Noun"])
    print(f"Match: {verb.form} ← {noun.form}")

# Or iterate over trees
for tree in ts.trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        verb = tree.word(match["Verb"])
        noun = tree.word(match["Noun"])
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

##### `search(pattern: Pattern, ordered: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]`

Search for pattern matches across all trees. Returns an iterator of (tree, match) tuples. Can be called multiple times. Uses automatic parallel processing for multi-file treebanks.

**Parameters:**
- `pattern` (Pattern): Compiled pattern from `compile_query()`
- `ordered` (bool): If True (default), matches are returned in deterministic order. If False, matches may arrive in any order for better performance.

```python
pattern = ts.compile_query("MATCH { Verb [upos=\"VERB\"]; }")

# Ordered iteration (default)
for tree, match in treebank.search(pattern):
    verb = tree.word(match["Verb"])
    print(f"Found: {verb.form}")

# Unordered for better performance
for tree, match in treebank.search(pattern, ordered=False):
    verb = tree.word(match["Verb"])
    print(f"Found: {verb.form}")
```

### Convenience Functions

#### `load(path: str) -> Treebank`

Smart function that automatically detects whether the path is a file or glob pattern and creates the appropriate Treebank.

```python
# Single file
tb = ts.load("corpus.conllu")

# Multiple files (automatically detected by * or ?)
tb = ts.load("data/*.conllu")

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

#### `compile_query(query: str) -> Pattern`

Parse a query string into a Pattern object.

```python
pattern = ts.compile_query("""
    MATCH {
        Verb [upos="VERB"];
        Noun [upos="NOUN"];
        Verb -[nsubj]-> Noun;
    }
""")
```

#### `trees(source: str, ordered: bool = True) -> Iterator[Tree]`

Read trees from one or more CoNLL-U files. Convenience wrapper for `load(source).trees(ordered)`.

**Parameters:**
- `source` (str): Path to a single file or glob pattern (e.g., "data/*.conllu")
- `ordered` (bool): If True (default), trees are returned in deterministic order

```python
# Single file
for tree in ts.trees("corpus.conllu"):
    print(f"Tree has {len(tree)} words")

# Multiple files with glob pattern
for tree in ts.trees("data/*.conllu"):
    print(f"Sentence: {tree.sentence_text}")

# Unordered for better performance
for tree in ts.trees("data/*.conllu", ordered=False):
    print(f"Tree: {tree.sentence_text}")
```

#### `search(source: str, query: str | Pattern, ordered: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]`

Search one or more files for pattern matches. Convenience wrapper for `load(source).search(pattern, ordered)`.

**Parameters:**
- `source` (str): Path to a single file or glob pattern (e.g., "data/*.conllu")
- `query` (str | Pattern): Query string or compiled Pattern
- `ordered` (bool): If True (default), matches are returned in deterministic order

```python
# Single file with query string
for tree, match in ts.search("corpus.conllu", 'MATCH { V [upos="VERB"]; }'):
    verb = tree.word(match["V"])
    print(f"Found: {verb.form}")

# Multiple files with compiled pattern
pattern = ts.compile_query("MATCH { Verb [upos=\"VERB\"]; }")
for tree, match in ts.search("data/*.conllu", pattern):
    verb = tree.word(match["Verb"])
    print(f"{verb.form}: {tree.sentence_text}")

# Unordered for better performance
for tree, match in ts.search("data/*.conllu", pattern, ordered=False):
    verb = tree.word(match["Verb"])
    print(verb.form)
```

#### `search_trees(trees: Tree | Iterable[Tree], query: str | Pattern) -> Iterator[tuple[Tree, dict[str, int]]]`

Search one or more Tree objects for pattern matches.

**Parameters:**
- `trees` (Tree | Iterable[Tree]): Single tree or list of trees to search
- `query` (str | Pattern): Query string or compiled Pattern

```python
# Search a single tree
tree = next(ts.trees("corpus.conllu"))
for tree, match in ts.search_trees(tree, pattern):
    verb = tree.word(match["V"])
    print(f"Found: {verb.form}")

# Search a list of trees
trees = list(ts.trees("corpus.conllu"))
for tree, match in ts.search_trees(trees, pattern):
    verb = tree.word(match["V"])
    print(f"Found: {verb.form}")
```

### Data Classes

#### `Tree`

Represents a dependency tree.

**Properties:**
- `sentence_text: str | None` - Reconstructed sentence text
- `metadata: dict[str, str]` - Tree metadata from CoNLL-U comments

**Methods:**
- `word(id: int) -> Word` - Get word by ID (0-indexed). Raises `IndexError` if out of range.
- `__getitem__(id: int) -> Word` - Alternative syntax: `tree[id]`. Raises `IndexError` if out of range.
- `__len__() -> int` - Number of words in tree

**String representation:**
```python
repr(tree)  # <Tree len=6 words='He helped us ...'>
```

**Examples:**
```python
tree = next(ts.trees("corpus.conllu"))
print(f"Sentence has {len(tree)} words")
print(f"Text: {tree.sentence_text}")
print(repr(tree))  # <Tree len=6 words='He helped us ...'>

# Get specific word (0-indexed)
word = tree.word(3)
print(f"{word.id}: {word.form}")

# Or use indexing syntax
word = tree[3]
print(f"{word.id}: {word.form}")

# Raises IndexError if out of range
try:
    word = tree.word(999)
except IndexError as e:
    print(f"Error: {e}")  # Error: word index out of range: 999
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

**String representation:**
```python
repr(word)  # <Word id=1 form='helped' lemma='help' upos='VERB' deprel='root'>
```

**Examples:**
```python
word = tree.word(5)
print(f"Form: {word.form}")
print(f"Lemma: {word.lemma}")
print(f"POS: {word.upos}")
print(repr(word))  # <Word id=5 form='...' lemma='...' upos='...' deprel='...'>
print(f"DepRel: {word.deprel}")
print(f"Features: {word.feats}")

# Navigate tree
if word.parent():
    print(f"Parent: {word.parent().form}")
for child in word.children():
    print(f"Child: {child.form} ({child.deprel})")
```

#### `Pattern`

Represents a parsed query pattern. Created by `compile_query()`. Opaque object that can be reused across multiple searches.

```python
pattern = ts.compile_query("MATCH { Verb [upos=\"VERB\"]; }")

# Reuse pattern across multiple searches
for tree, match in ts.search("data/*.conllu", pattern):
    verb = tree.word(match["Verb"])
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
pattern = ts.compile_query(query)

# Open treebank and search for matches
treebank = ts.load("corpus.conllu")
for tree, match in treebank.search(pattern):
    main = tree.word(match["Main"])
    comp = tree.word(match["Comp"])
    print(f"  Main = {main.form} (lemma: {main.lemma})")
    print(f"  Comp = {comp.form} (lemma: {comp.lemma})")
    print(f"  Sentence: {tree.sentence_text}")
    print()

# Or use functional API
for tree, match in ts.search("corpus.conllu", pattern):
    main = tree.word(match["Main"])
    comp = tree.word(match["Comp"])
    print(f"Match: {main.form} -[xcomp]-> {comp.form}")

# Or iterate trees manually
for tree in treebank.trees():
    for match in ts.search(tree, pattern):
        main = tree.word(match["Main"])
        comp = tree.word(match["Comp"])
        print(f"{main.form} → {comp.form}")
```

## Error Handling

All operations raise Python exceptions on error:

```python
try:
    # Parse errors
    pattern = ts.compile_query("Invalid [syntax")
except Exception as e:
    print(f"Query parse error: {e}")

try:
    # File not found
    for tree in ts.trees("nonexistent.conllu"):
        pass
except Exception as e:
    print(f"File error: {e}")

try:
    # Malformed CoNLL-U
    for tree in ts.trees("bad_format.conllu"):
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

Multi-file operations (via `load()` with glob patterns, or `get_trees()`/`get_matches()`) automatically process files in parallel using bounded channels and rayon for optimal throughput.

```python
# Automatic parallel file reading
for tree in ts.trees("data/*.conllu"):
    # Trees from different files processed in parallel automatically
    pass

# Automatic parallel search across files
pattern = ts.compile_query("MATCH { Verb [upos=\"VERB\"]; }")
for tree, match in ts.search("data/*.conllu", pattern):
    # Searches run in parallel across files automatically
    verb = tree.word(match["Verb"])
    print(verb.form)

# Unordered for maximum performance
for tree, match in ts.search("data/*.conllu", pattern, ordered=False):
    # Even faster when order doesn't matter
    verb = tree.word(match["Verb"])
    print(verb.form)
```

### Best Practices

```python
# ✅ GOOD: Parse query once, reuse
pattern = ts.compile_query(query_string)
for tree in ts.trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        # Process match
        pass

# ❌ BAD: Re-parsing query every iteration
for tree in ts.trees("corpus.conllu"):
    pattern = ts.compile_query(query_string)  # Wasteful!
    for match in ts.search(tree, pattern):
        # Process match
        pass

# ✅ GOOD: Use get_matches for simple cases
for tree, match in ts.search("corpus.conllu", pattern):
    # Convenient and efficient
    verb = tree.word(match["Verb"])

# ✅ GOOD: Multi-file operations use automatic parallel processing
for tree, match in ts.search("data/*.conllu", pattern):
    # Parallel processing happens automatically
    verb = tree.word(match["Verb"])

# ✅ GOOD: Use ordered=False when order doesn't matter
for tree, match in ts.search("data/*.conllu", pattern, ordered=False):
    # Maximum performance
    verb = tree.word(match["Verb"])
```
