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
    Verb [pos="VERB"];
    Noun [pos="NOUN"];
    Verb -[nsubj]-> Noun;
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

**Multiple constraints** (AND):
```
V [upos="VERB", lemma="be"];
```

**Empty constraint** (matches any node):
```
AnyNode [];
```

### Pattern Edges

Define relationships between nodes:

```
Parent -[deprel]-> Child;
```

**Dependency types:**
- `-[nsubj]->` - Specific dependency relation
- `->` - Any child (no relation specified)

**Example patterns:**

```
// VERB with nominal subject
V [upos="VERB"];
N [upos="NOUN"];
V -[nsubj]-> N;

// Verb with xcomp (control verb)
Main [upos="VERB"];
Comp [upos="VERB"];
Main -[xcomp]-> Comp;

// Complex: VERB → NOUN → ADJ
V [upos="VERB"];
N [upos="NOUN"];
A [upos="ADJ"];
V -[obj]-> N;
N -[amod]-> A;
```

## Python API Reference

### Core Functions

#### `parse_query(query: str) -> Pattern`

Parse a query string into a Pattern object.

```python
pattern = ts.parse_query("""
    Verb [upos="VERB"];
    Noun [upos="NOUN"];
    Verb -[nsubj]-> Noun;
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

#### `read_trees_glob(pattern: str, parallel: bool = True) -> Iterator[Tree]`

Read trees from multiple files matching a glob pattern.

```python
# Sequential
for tree in ts.read_trees_glob("data/*.conllu", parallel=False):
    # Process tree...
    pass

# Parallel (default)
for tree in ts.read_trees_glob("data/*.conllu"):
    # Trees from multiple files processed in parallel
    pass
```

#### `search_files(glob_pattern: str, query_pattern: Pattern, parallel: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]`

Search multiple files matching a glob pattern. Returns an iterator of (tree, match) tuples.

```python
pattern = ts.parse_query("Verb [upos=\"VERB\"];")

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
- `id: int` - Word ID (1-indexed in CoNLL-U)
- `form: str` - Word form
- `lemma: str` - Lemma
- `pos: str` - Universal POS tag (upos)
- `xpos: str` - Language-specific POS tag
- `deprel: str` - Dependency relation to parent
- `head: int | None` - Head word ID (None for root)

**Methods:**
- `parent() -> Word | None` - Get parent word
- `children() -> list[Word]` - Get all children
- `children_by_deprel(deprel: str) -> list[Word]` - Get children with specific relation

```python
word = tree.get_word(5)
print(f"Form: {word.form}")
print(f"Lemma: {word.lemma}")
print(f"POS: {word.pos}")
print(f"DepRel: {word.deprel}")

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
pattern = ts.parse_query("Verb [upos=\"VERB\"];")
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
    Main [upos="VERB"];
    Comp [upos="VERB"];
    Main -[xcomp]-> Comp;
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

## Performance Notes

- **Parse queries once**: `Pattern` objects are reusable across searches
- **Exhaustive search**: Finds ALL matches, not just first/leftmost
- **CSP-based matching**: Forward checking prevents exponential blowup
- **Parallel processing**: Use `parallel=True` (default) for multi-file operations
- **Memory efficient**: Iterator-based API streams results

### Parallel Processing

```python
# Parallel file reading (default)
for tree in ts.read_trees_glob("data/*.conllu", parallel=True):
    # Trees from different files processed in parallel
    pass

# Parallel search across files (default)
pattern = ts.parse_query("Verb [upos=\"VERB\"];")
for tree, match in ts.search_files("data/*.conllu", pattern, parallel=True):
    # Searches run in parallel across files
    verb = tree.get_word(match["Verb"])
    print(verb.form)

# Sequential processing (if needed)
for tree, match in ts.search_files("data/*.conllu", pattern, parallel=False):
    # Process files one at a time
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

# ✅ GOOD: Use parallel=True for multiple files
for tree, match in ts.search_files("data/*.conllu", pattern):
    # Parallel processing by default
    verb = tree.get_word(match["Verb"])
```
