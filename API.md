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
for match in ts.search_file("corpus.conllu", pattern):
    verb_idx, noun_idx = match
    print(f"Match: verb={verb_idx}, noun={noun_idx}")

# 3. Or work with individual trees
for tree in ts.read_trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        verb = tree.words[match[0]]
        noun = tree.words[match[1]]
        print(f"{verb.form} ← {noun.form}")
```

## Query Language

### Pattern Elements

Define nodes with constraints:

```
VariableName [constraint];
```

**Available constraints:**
- `pos="VERB"` - Part-of-speech tag
- `lemma="run"` - Lemma
- `form="running"` - Word form
- `deprel="nsubj"` - Dependency relation (to parent)

**Multiple constraints** (AND):
```
V [pos="VERB", lemma="be"];
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
V [pos="VERB"];
N [pos="NOUN"];
V -[nsubj]-> N;

// Verb with xcomp (control verb)
Main [pos="VERB"];
Comp [pos="VERB"];
Main -[xcomp]-> Comp;

// Complex: VERB → NOUN → ADJ
V [pos="VERB"];
N [pos="NOUN"];
A [pos="ADJ"];
V -[obj]-> N;
N -[amod]-> A;
```

## Python API Reference

### Core Functions

#### `parse_query(query: str) -> Pattern`

Parse a query string into a Pattern object.

```python
pattern = ts.parse_query("""
    Verb [pos="VERB"];
    Noun [pos="NOUN"];
    Verb -[nsubj]-> Noun;
""")
```

#### `search(tree: Tree, pattern: Pattern) -> Iterator[list[int]]`

Search a single tree for pattern matches.

```python
for match in ts.search(tree, pattern):
    # match is a list of word indices
    verb_idx, noun_idx = match
    verb = tree.words[verb_idx]
    noun = tree.words[noun_idx]
```

#### `read_trees(path: str) -> Iterator[Tree]`

Read trees from a CoNLL-U file (supports gzip).

```python
for tree in ts.read_trees("corpus.conllu"):
    print(f"Tree has {len(tree.words)} words")
```

#### `search_file(path: str, pattern: Pattern) -> Iterator[list[int]]`

Search a single file for pattern matches.

```python
for match in ts.search_file("corpus.conllu", pattern):
    # match is a list of word indices
    print(match)
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

#### `search_files(glob_pattern: str, query_pattern: Pattern, parallel: bool = True) -> Iterator[list[int]]`

Search multiple files matching a glob pattern.

```python
pattern = ts.parse_query("Verb [pos=\"VERB\"];")

# Parallel search across all files
for match in ts.search_files("data/*.conllu", pattern):
    print(match)
```

### Data Classes

#### `Tree`

Represents a dependency tree.

**Attributes:**
- `words: list[Word]` - List of words in the tree (index 0 is ROOT)
- `metadata: dict[str, str]` - Tree metadata from CoNLL-U comments

```python
tree = next(ts.read_trees("corpus.conllu"))
print(f"Sentence has {len(tree.words)} words")
for word in tree.words:
    print(f"{word.id}: {word.form}")
```

#### `Word`

Represents a single word/node in the tree.

**Attributes:**
- `id: int` - Word ID (1-indexed in CoNLL-U)
- `form: str` - Word form
- `lemma: str` - Lemma
- `upos: str` - Universal POS tag
- `xpos: str | None` - Language-specific POS tag
- `feats: dict[str, str]` - Morphological features
- `head: int` - Head word ID (0 for root)
- `deprel: str` - Dependency relation
- `deps: str | None` - Enhanced dependencies
- `misc: str | None` - Miscellaneous annotations

```python
word = tree.words[5]
print(f"Form: {word.form}")
print(f"Lemma: {word.lemma}")
print(f"POS: {word.upos}")
print(f"DepRel: {word.deprel}")
if word.xpos:
    print(f"XPOS: {word.xpos}")
```

#### `Pattern`

Represents a parsed query pattern. Created by `parse_query()`.

```python
pattern = ts.parse_query("Verb [pos=\"VERB\"];")
# Use pattern with search functions
```

## Complete Example

```python
import treesearch as ts

# Find all control verbs (VERB with VERB xcomp)
query = """
    Main [pos="VERB"];
    Comp [pos="VERB"];
    Main -[xcomp]-> Comp;
"""

# Parse query once
pattern = ts.parse_query(query)

# Search trees and display results
for tree in ts.read_trees("corpus.conllu"):
    for match in ts.search(tree, pattern):
        main_idx, comp_idx = match
        main = tree.words[main_idx]
        comp = tree.words[comp_idx]
        print(f"  Main = {main.form} (lemma: {main.lemma})")
        print(f"  Comp = {comp.form} (lemma: {comp.lemma})")
        print()

# Or search files directly (more efficient)
for match in ts.search_file("corpus.conllu", pattern):
    main_idx, comp_idx = match
    print(f"Match: main={main_idx}, comp={comp_idx}")
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
pattern = ts.parse_query("Verb [pos=\"VERB\"];")
for match in ts.search_files("data/*.conllu", pattern, parallel=True):
    # Searches run in parallel across files
    pass

# Sequential processing (if needed)
for match in ts.search_files("data/*.conllu", pattern, parallel=False):
    # Process files one at a time
    pass
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
for match in ts.search_file("corpus.conllu", pattern):
    # More efficient than reading trees manually

# ✅ GOOD: Use parallel=True for multiple files
for match in ts.search_files("data/*.conllu", pattern):
    # Parallel processing by default
```
