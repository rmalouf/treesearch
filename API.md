# Treesearch API Reference

**Quick reference for querying dependency treebanks**

## Overview

Treesearch provides both an object-oriented and functional API for searching linguistic dependency trees using a pattern-matching query language. The typical workflow is:

1. Load a treebank with `load()` or create with `Treebank.from_*()`
2. Search with `treebank.search(query)` for all matches, or `treebank.filter(query)` for matching trees only
3. Iterate with `treebank.trees()` to access all trees
4. Access matched nodes via the `Tree` and `Word` objects

## Basic Usage (Python)

### Object-Oriented API

```python
import treesearch as ts

# Open a treebank (single file or glob pattern)
treebank = ts.load("corpus.conllu")

# Search with query string directly
query = """
    MATCH {
        Verb [upos="VERB"];
        Noun [upos="NOUN"];
        Verb -[nsubj]-> Noun;
    }
"""
for tree, match in treebank.search(query):
    verb = tree.word(match["Verb"])
    noun = tree.word(match["Noun"])
    print(f"Match: {verb.form} ← {noun.form}")

# Or compile once and reuse for better performance
pattern = ts.compile_query(query)
for tree, match in treebank.search(pattern):
    verb = tree.word(match["Verb"])
    noun = tree.word(match["Noun"])
    print(f"Match: {verb.form} ← {noun.form}")

# Multiple files with automatic parallel processing
treebank = ts.load("data/**/*.conllu.gz")
for tree, match in treebank.search('MATCH { V [upos="VERB"]; }'):
    verb = tree.word(match["V"])
    print(f"Found: {verb.form}")

# Iterate over trees without searching
for tree in treebank.trees():
    print(f"Tree has {len(tree)} words")
```

### Functional API 

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
- `misc.SpaceAfter="No"` - Miscellaneous feature (dotted notation)

**Constraint values** can be either:
- **Literal strings** (enclosed in double quotes): `lemma="run"`
- **Regular expressions** (enclosed in forward slashes): `lemma=/run.*/`

**Regular expression patterns:**

Regex patterns are **automatically anchored** for full-string matching (consistent with literal behavior). This means `/run/` matches exactly "run", not "running". Use `.*` for partial matches:

```
# Match exactly "run" (equivalent to lemma="run")
V [lemma=/run/];

# Match lemmas starting with "run" (run, runs, running, etc.)
V [lemma=/run.*/];

# Match VERB or AUX using alternation
W [upos=/VERB|AUX/];

# Match words ending in "ing"
W [form=/.*ing/];

# Match words containing "el"
W [form=/.*el.*/];

# Match past or present tense
V [feats.Tense=/Past|Pres/];

# Combine literal and regex constraints
V [upos="VERB" & lemma=/(be|have).*/];
```

**Note:** Patterns are compiled with implicit `^...$` anchors, so you don't need to add them manually. `/run/` becomes `/^run$/` internally. Regular expressions use Rust's [regex syntax](https://docs.rs/regex/latest/regex/#syntax). Invalid patterns are caught during query compilation with a clear error message.

**Multiple constraints** (AND):
```
V [upos="VERB" & lemma="be"];
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
    Noun [feats.Number="Plur" & feats.Case="Nom"];
}
```

**Negation** works with both literals and regex:
```
MATCH {
    # NOT a noun
    W [upos!="NOUN"];

    # Does NOT start with "be"
    V [lemma!=/be.*/];

    # NOT past or present tense
    V [feats.Tense!=/Past|Pres/];
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

Represents a collection of dependency trees from one or more files.

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
- `ordered` (bool): If True (default), trees are returned in corpus order. If False, trees may arrive in any order for better performance.

```python
# Ordered iteration (default)
for tree in treebank.trees():
    print(f"Tree has {len(tree)} words")
    print(f"Sentence: {tree.sentence_text}")

# Unordered for better performance
for tree in treebank.trees(ordered=False):
    print(f"Tree: {tree.sentence_text}")
```

##### `search(pattern: Pattern | str, ordered: bool = True) -> Iterator[tuple[Tree, dict[str, int]]]`

Search for pattern matches across all trees. Returns an iterator of (tree, match) tuples. Can be called multiple times. Uses automatic parallel processing for multi-file treebanks.

**Parameters:**
- `pattern` (Pattern | str): Compiled Pattern from `compile_query()` or query string
- `ordered` (bool): If True (default), matches are returned in corpus order. If False, matches may arrive in any order for better performance.

```python
# Pass query string directly (simple)
for tree, match in treebank.search('MATCH { Verb [upos="VERB"]; }'):
    verb = tree.word(match["Verb"])
    print(f"Found: {verb.form}")

# Or compile once and reuse (better for multiple searches)
pattern = ts.compile_query("MATCH { Verb [upos=\"VERB\"]; }")
for tree, match in treebank.search(pattern):
    verb = tree.word(match["Verb"])
    print(f"Found: {verb.form}")

# Unordered for better performance
for tree, match in treebank.search(pattern, ordered=False):
    verb = tree.word(match["Verb"])
    print(f"Found: {verb.form}")
```

##### `filter(pattern: Pattern | str, ordered: bool = True) -> Iterator[Tree]`

Filter trees that have at least one match for the pattern. More efficient than `search()` when you only need to know which trees match, not the specific bindings. Uses early termination—stops searching each tree after finding the first match.

**Parameters:**
- `pattern` (Pattern | str): Compiled Pattern from `compile_query()` or query string
- `ordered` (bool): If True (default), trees are returned in corpus order. If False, trees may arrive in any order for better performance.

```python
# Find all trees containing a verb
for tree in treebank.filter('MATCH { V [upos="VERB"]; }'):
    print(tree.sentence_text)

# With compiled pattern
pattern = ts.compile_query('MATCH { V [upos="VERB"]; N []; V -[nsubj]-> N; }')
for tree in treebank.filter(pattern):
    print(f"Tree with subject: {tree.sentence_text}")

# Unordered for better performance
for tree in treebank.filter(pattern, ordered=False):
    print(tree.sentence_text)
```

**Note:** Use `filter()` instead of `search()` when:
- You only need to know which trees match, not the variable bindings
- You want to count matching trees
- You're filtering trees for further processing

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

### Visualization Functions

#### `to_displacy(tree: Tree) -> dict`

Convert a Tree to displaCy's manual rendering format.

**Parameters:**
- `tree` (Tree): A Tree object to convert

**Returns:**
- `dict`: Dictionary with 'words' and 'arcs' keys in displaCy format

```python
tree = next(ts.trees("corpus.conllu"))
data = ts.to_displacy(tree)
# Returns: {'words': [{'text': '...', 'tag': '...'}, ...], 'arcs': [...]}

# Use with spaCy's displacy
from spacy import displacy
displacy.render(data, style="dep", manual=True)
```

#### `render(tree: Tree, **options) -> str`

Render a Tree as an SVG dependency visualization using displaCy.

**Requirements:** Requires spaCy to be installed (`pip install treesearch-ud[viz]` or `pip install spacy`)

**Parameters:**
- `tree` (Tree): A Tree object to render
- `**options`: Additional options passed to displacy.render()
  - `jupyter` (bool): Return HTML for Jupyter display (default: auto-detect)
  - `compact` (bool): Use compact visualization mode
  - `word_spacing` (int): Spacing between words
  - `distance` (int): Distance between dependency arcs

**Returns:**
- `str`: SVG markup string (or displays in Jupyter if jupyter=True)

**Raises:**
- `ImportError`: If spaCy is not installed

```python
# Basic usage
tree = next(ts.trees("corpus.conllu"))
svg = ts.render(tree)
print(svg)  # SVG markup

# Save to file
with open("tree.svg", "w") as f:
    f.write(svg)

# In Jupyter notebook (displays inline)
ts.render(tree, jupyter=True)

# Compact mode with custom spacing
svg = ts.render(tree, compact=True, word_spacing=50)

# Also available as Tree methods
svg = tree.render()
data = tree.to_displacy()
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
- `xpos: str | None` - Language-specific POS tag (None if not specified)
- `deprel: str` - Dependency relation to parent
- `head: int | None` - Head word ID (0-based index, None for root)
- `children_ids: list[int]` - IDs of all children words
- `feats: dict[str, str]` - Morphological features as key-value pairs
- `misc: dict[str, str]` - Miscellaneous annotations as key-value pairs

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

# Access morphological features
print(f"Features: {word.feats}")  # {'Tense': 'Past', 'VerbForm': 'Fin'}
if 'Tense' in word.feats:
    print(f"Tense: {word.feats['Tense']}")

# Access misc annotations
print(f"Misc: {word.misc}")  # {'SpaceAfter': 'No'}

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

## Complete Examples

### Example 1: Control Verbs

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

# Open treebank and search for matches (passing query string directly)
treebank = ts.load("corpus.conllu")
for tree, match in treebank.search(query):
    main = tree.word(match["Main"])
    comp = tree.word(match["Comp"])
    print(f"  Main = {main.form} (lemma: {main.lemma})")
    print(f"  Comp = {comp.form} (lemma: {comp.lemma})")
    print(f"  Sentence: {tree.sentence_text}")
    print()

# For multiple searches, compile once for better performance
pattern = ts.compile_query(query)
for tree, match in treebank.search(pattern):
    main = tree.word(match["Main"])
    comp = tree.word(match["Comp"])
    print(f"Match: {main.form} -[xcomp]-> {comp.form}")

# Or use functional API with string
for tree, match in ts.search("corpus.conllu", query):
    main = tree.word(match["Main"])
    comp = tree.word(match["Comp"])
    print(f"Match: {main.form} -[xcomp]-> {comp.form}")

# Or iterate trees manually
for tree in treebank.trees():
    for tree, match in ts.search_trees(tree, query):
        main = tree.word(match["Main"])
        comp = tree.word(match["Comp"])
        print(f"{main.form} → {comp.form}")
```

### Example 2: Using Regular Expressions

```python
import treesearch as ts

# Find progressive forms (verbs ending in -ing)
# Regex patterns are automatically anchored, so /.*ing/ matches full words ending in "ing"
query = """
MATCH {
    V [upos="VERB" & form=/.*ing/];
}
"""

treebank = ts.load("corpus.conllu")
for tree, match in treebank.search(query):
    verb = tree.word(match["V"])
    print(f"Progressive: {verb.form} (lemma: {verb.lemma})")

# Find modal verbs (can, could, may, might, must, shall, should, will, would)
# Alternation matches any of the options (full string match)
modal_query = """
MATCH {
    Modal [lemma=/(can|may|must|shall|will|could|might|should|would)/];
    Verb [upos="VERB"];
    Modal -> Verb;
}
"""

for tree, match in treebank.search(modal_query):
    modal = tree.word(match["Modal"])
    verb = tree.word(match["Verb"])
    print(f"Modal construction: {modal.form} + {verb.form}")
    print(f"  Sentence: {tree.sentence_text}")

# Find words that are NOT common auxiliaries
# Negated regex with alternation
non_aux_query = """
MATCH {
    V [upos=/VERB|AUX/ & lemma!=/(be|have|do|will|would|can|could|may|might|must|shall|should)/];
}
"""

for tree, match in treebank.search(non_aux_query):
    verb = tree.word(match["V"])
    print(f"Content verb: {verb.form} (lemma: {verb.lemma})")
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
    # Invalid regex pattern
    pattern = ts.compile_query('MATCH { V [lemma=/[unclosed/]; }')
except Exception as e:
    print(f"Regex error: {e}")
    # Error: Query error: Invalid regex pattern '[unclosed': regex parse error...

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

- **Query compilation**:
  - Pass query strings directly for one-off searches: `treebank.search('MATCH { V [upos="VERB"]; }')`
  - Compile once with `compile_query()` when reusing the same pattern multiple times
  - Regular expressions are compiled during query compilation, so reusing a compiled pattern is especially beneficial for regex-heavy queries
- **Use `filter()` for existence checks**: When you only need matching trees (not bindings), use `filter()` instead of `search()`—it stops after finding the first match in each tree
- **Regex vs. literals**: Literal string matching is faster than regex matching. Use literals when exact matches suffice:
  - Prefer `lemma="run"` over `lemma=/run/` (both match exactly "run", but literal is faster)
  - Use regex when you need pattern matching: `form=/.*ing/`, `lemma=/(be|have).*/`, `upos=/VERB|AUX/`
- **Automatic parallel processing**: Multi-file operations automatically process files in parallel for better performance
- **Memory efficient**: Iterator-based API streams results without loading entire corpus
- **Use gzipped files**: Store CoNLL-U files as `.conllu.gz` to reduce I/O time and disk usage (decompression is automatic)
- **Unordered iteration**: Use `ordered=False` for better performance when order doesn't matter

