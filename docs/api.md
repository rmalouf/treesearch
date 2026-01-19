# API Reference

## Functions

### load(path) → Treebank

Load a treebank from file(s). Accepts single files or glob patterns.

```python
tb = ts.load("corpus.conllu")
tb = ts.load("data/**/*.conllu.gz")
```

### from_string(text) → Treebank

Create a treebank from a CoNLL-U string.

```python
tb = ts.from_string("""# text = Hello world.
1	Hello	hello	INTJ	_	_	0	root	_	_
2	world	world	NOUN	_	_	1	vocative	_	_
""")
```

### compile_query(query) → Pattern

Compile a query string into a reusable Pattern. Raises `ValueError` on syntax error.

```python
pattern = ts.compile_query('MATCH { V [upos="VERB"]; }')
```

### trees(source, ordered=True) → Iterator[Tree]

Read trees from file(s).

```python
for tree in ts.trees("corpus/*.conllu"):
    print(tree.sentence_text)
```

### search(source, query, ordered=True) → Iterator[tuple[Tree, dict]]

Search file(s) for pattern matches.

```python
for tree, match in ts.search("corpus.conllu", 'MATCH { V [upos="VERB"]; }'):
    verb = tree.word(match["V"])
```

### search_trees(trees, query) → Iterator[tuple[Tree, dict]]

Search Tree object(s) for pattern matches.

```python
tree = next(ts.trees("corpus.conllu"))
for tree, match in ts.search_trees(tree, pattern):
    ...
```

### to_displacy(tree) → dict

Convert a Tree to displaCy format for visualization.

```python
data = ts.to_displacy(tree)
# Returns: {'words': [...], 'arcs': [...]}
```

### render(tree, **options) → str

Render a Tree as SVG using displaCy. Requires spaCy (`pip install treesearch-ud[viz]`).

```python
svg = ts.render(tree)
ts.render(tree, jupyter=True)  # Display in Jupyter
```

## Treebank

Collection of trees from one or more files.

### treebank.trees(ordered=True) → Iterator[Tree]

Iterate over all trees.

### treebank.search(query, ordered=True) → Iterator[tuple[Tree, dict]]

Search for pattern matches. Accepts query string or compiled Pattern.

**Parameters:**
- `ordered`: If True (default), results in corpus order. If False, faster but unordered.

### treebank.filter(query, ordered=True) → Iterator[Tree]

Find trees that have at least one match. More efficient than `search()` when you only need matching trees, not bindings—stops after first match per tree.

```python
# Count trees with passive constructions
count = sum(1 for _ in treebank.filter('MATCH { V []; V -[aux:pass]-> _; }'))

# Get matching trees for further processing
for tree in treebank.filter(pattern):
    print(tree.sentence_text)
```

## Tree

A dependency tree (parsed sentence).

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `sentence_text` | `str \| None` | Reconstructed sentence |
| `metadata` | `dict[str, str]` | CoNLL-U comments |

### Methods

- `tree.word(id) → Word` - Get word by ID (0-indexed). Raises `IndexError` if out of range.
- `tree[id] → Word` - Same as `word(id)`
- `len(tree) → int` - Number of words
- `tree.to_displacy() → dict` - Convert to displaCy format
- `tree.render(**options) → str` - Render as SVG (requires spaCy)

## Word

A single word in a tree.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `id` | `int` | 0-based index in tree |
| `token_id` | `int` | 1-based CoNLL-U ID |
| `form` | `str` | Surface form |
| `lemma` | `str` | Dictionary form |
| `upos` | `str` | Universal POS tag |
| `xpos` | `str \| None` | Language-specific POS |
| `deprel` | `str` | Dependency relation |
| `head` | `int \| None` | Parent word ID (None for root) |
| `children_ids` | `list[int]` | Child word IDs |
| `feats` | `dict[str, str]` | Morphological features |
| `misc` | `dict[str, str]` | Miscellaneous annotations |

### Methods

- `word.parent() → Word | None` - Get parent word
- `word.children() → list[Word]` - Get all children
- `word.children_by_deprel(deprel) → list[Word]` - Get children with specific relation

## Pattern

Compiled query pattern (opaque). Created by `compile_query()`, used with search functions.

## Query Language Summary

```
MATCH {
    # Node constraints
    V [upos="VERB"];              # by POS
    V [lemma="run"];              # by lemma (exact)
    V [lemma=/run.*/];            # by regex (starts with "run")
    V [upos="VERB" & lemma="run"]; # multiple (AND)
    V [feats.Tense="Past"];       # by feature
    V [];                         # any word

    # Regular expressions (automatically anchored)
    V [form=/.*ing/];             # ends with -ing
    V [upos=/VERB|AUX/];          # VERB or AUX
    V [lemma!=/(be|have)/];       # not be or have

    # Negation
    V [upos!="VERB"];             # not a verb
    V !-[obj]-> _;                # no object

    # Edges
    V -[nsubj]-> N;               # specific relation
    V -> N;                       # any relation

    # Precedence
    V < N;                        # V immediately before N
    V << N;                       # V anywhere before N
}
```

**Regex patterns** use `/pattern/` syntax and are automatically anchored for full-string matching. Use `.*` for partial matches: `/run.*/` matches "run", "runs", "running".

See [Query Language Reference](query-language.md) for complete syntax.
