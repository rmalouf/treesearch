# Treesearch API Reference

**Quick reference for querying dependency treebanks**

## Overview

Treesearch provides a functional API for searching linguistic dependency trees using a pattern-matching query language. The typical workflow is:

1. Parse query string to `Pattern` with `parse_query()`
2. Search files with `search_file()` or `search_files()`
3. Or read trees with `read_trees()` and search with `search()`
4. Access matched nodes by index

## Basic Usage (Python)

```python
import treesearch as ts

# 1. Parse your query once
query = """
    MATCH {
        Verb [upos="VERB"];
        Noun [upos="NOUN"];
        Verb -[nsubj]-> Noun;
    }
"""
pattern = ts.parse_query(query)

# 2. Search a single file
for tree, match in ts.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["Verb"])
    noun = tree.get_word(match["Noun"])
    print(f"Match: {verb.form} ← {noun.form}")

# 3. Or work with individual trees
for tree in ts.read_trees("corpus.conllu"):
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

#### `read_trees(path: str) -> Iterator[Tree]`

Read trees from a CoNLL-U file (supports gzip).

```python
for tree in ts.read_trees("corpus.conllu"):
    print(f"Tree has {len(tree)} words")
    print(f"Sentence: {tree.sentence_text}")
```

#### `search_file(path: str, pattern: Pattern) -> Iterator[tuple[Tree, dict[str, int]]]`

Search a single file for pattern matches. Returns an iterator of (tree, match) tuples.

```python
for tree, match in ts.search_file("corpus.conllu", pattern):
    # match is a dictionary: {"Verb": 3, "Noun": 5}
    verb = tree.get_word(match["Verb"])
    print(f"Found: {verb.form} in '{tree.sentence_text}'")
```

#### `read_trees_glob(pattern: str) -> Iterator[Tree]`

Read trees from multiple files matching a glob pattern. Automatically processes files in parallel for better performance.

```python
# Automatic parallel processing
for tree in ts.read_trees_glob("data/*.conllu"):
    # Trees from multiple files processed in parallel automatically
    pass
```

#### `search_files(glob_pattern: str, query_pattern: Pattern) -> Iterator[tuple[Tree, dict[str, int]]]`

Search multiple files matching a glob pattern. Returns an iterator of (tree, match) tuples. Automatically processes files in parallel for better performance.

```python
pattern = ts.parse_query("MATCH { Verb [upos=\"VERB\"]; }")

# Parallel search across all files
for tree, match in ts.search_files("data/*.conllu", pattern):
    verb = tree.get_word(match["Verb"])
    print(f"{verb.form}: {tree.sentence_text}")
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

**Properties:**
- `n_vars: int` - Number of variables in the pattern

```python
pattern = ts.parse_query("MATCH { Verb [upos=\"VERB\"]; }")
print(f"Pattern has {pattern.n_vars} variables")
# Reuse pattern across multiple searches
for tree, match in ts.search_files("data/*.conllu", pattern):
    # ...process matches...
    pass
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

# Search trees and display results
for tree in ts.read_trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        main = tree.get_word(match["Main"])
        comp = tree.get_word(match["Comp"])
        print(f"  Main = {main.form} (lemma: {main.lemma})")
        print(f"  Comp = {comp.form} (lemma: {comp.lemma})")
        print(f"  Sentence: {tree.sentence_text}")
        print()

# Or search files directly (more efficient)
for tree, match in ts.search_file("corpus.conllu", pattern):
    main = tree.get_word(match["Main"])
    comp = tree.get_word(match["Comp"])
    print(f"Match: {main.form} -[xcomp]-> {comp.form}")
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
    for tree in ts.read_trees("nonexistent.conllu"):
        pass
except Exception as e:
    print(f"File error: {e}")

try:
    # Malformed CoNLL-U
    for tree in ts.read_trees("bad_format.conllu"):
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

### Parallel Processing

Multi-file operations (`read_trees_glob` and `search_files`) automatically process files in parallel using bounded channels and rayon for optimal throughput.

```python
# Automatic parallel file reading
for tree in ts.read_trees_glob("data/*.conllu"):
    # Trees from different files processed in parallel automatically
    pass

# Automatic parallel search across files
pattern = ts.parse_query("MATCH { Verb [upos=\"VERB\"]; }")
for tree, match in ts.search_files("data/*.conllu", pattern):
    # Searches run in parallel across files automatically
    verb = tree.get_word(match["Verb"])
    print(verb.form)
```

### Best Practices

```python
# ✅ GOOD: Parse query once, reuse
pattern = ts.parse_query(query_string)
for tree in ts.read_trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        # Process match

# ❌ BAD: Re-parsing query every iteration
for tree in ts.read_trees("corpus.conllu"):
    pattern = ts.parse_query(query_string)  # Wasteful!
    for match in ts.search(tree, pattern):
        # Process match

# ✅ GOOD: Use search_file for simple cases
for tree, match in ts.search_file("corpus.conllu", pattern):
    # More efficient than reading trees manually
    verb = tree.get_word(match["Verb"])

# ✅ GOOD: Multi-file operations use automatic parallel processing
for tree, match in ts.search_files("data/*.conllu", pattern):
    # Parallel processing happens automatically
    verb = tree.get_word(match["Verb"])
```
