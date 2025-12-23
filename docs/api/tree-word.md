# Tree and Word classes

Classes for representing and navigating dependency trees.

## Tree

Represents a dependency tree (parsed sentence) from a CoNLL-U file.

### Properties

#### sentence_text

Reconstructed sentence text from CoNLL-U metadata.

```python
tree.sentence_text -> str | None
```

**Returns:**

- Sentence text if available from CoNLL-U `# text =` comment, None otherwise

**Example:**

```python
for tree in treesearch.trees("corpus.conllu"):
    if tree.sentence_text:
        print(tree.sentence_text)
```

---

#### metadata

CoNLL-U metadata from sentence comments.

```python
tree.metadata -> dict[str, str]
```

**Returns:**

- Dictionary of metadata key-value pairs from `#` comments

**Example:**

```python
for tree in treesearch.trees("corpus.conllu"):
    if 'sent_id' in tree.metadata:
        print(f"Sentence {tree.metadata['sent_id']}: {tree.sentence_text}")
```

---

### Methods

#### get_word()

Get a word by its ID.

```python
tree.get_word(id: int) -> Word | None
```

**Parameters:**

- `id` (int) - Word index (0-based)

**Returns:**

- Word object if ID is valid, None otherwise

**Example:**

```python
for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    if verb:
        print(f"{verb.form} ({verb.pos})")
```

**Notes:**

- Word IDs are 0-based indices into the tree
- Returns None if ID is out of range

**See also:** Word properties and methods

---

#### find_path()

Find the dependency path between two words.

```python
tree.find_path(x: Word, y: Word) -> list[Word] | None
```

**Parameters:**

- `x` (Word) - Starting word
- `y` (Word) - Target word

**Returns:**

- List of words forming the dependency path from x to y, or None if no path exists

**Example:**

```python
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        N [upos="NOUN"];
        V -[obj]-> N;
    }
""")

for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    noun = tree.get_word(match["N"])

    path = tree.find_path(verb, noun)
    if path:
        path_str = " -> ".join(f"{w.form}({w.deprel})" for w in path)
        print(f"Path: {path_str}")
```

**Notes:**

- Path includes both start and end words
- Returns None if words are in disconnected components
- Path follows dependency edges (not linear word order)

---

#### \_\_len\_\_()

Get the number of words in the tree.

```python
len(tree) -> int
```

**Returns:**

- Number of words in the tree

**Example:**

```python
for tree in treesearch.trees("corpus.conllu"):
    print(f"Sentence has {len(tree)} words: {tree.sentence_text}")
```

---

## Word

Represents a single word (node) in a dependency tree.

### Properties

All properties are read-only.

#### id

Word ID (0-based index in tree).

```python
word.id -> int
```

**Example:**

```python
verb = tree.get_word(match["V"])
print(f"Verb at position {verb.id}")
```

---

#### token_id

Token ID from CoNLL-U file (1-based).

```python
word.token_id -> int
```

**Notes:**

- Corresponds to the first column in CoNLL-U format
- 1-based indexing (first word is 1, not 0)
- Use `word.id` for accessing words via `tree.get_word()`

---

#### form

Surface form of the word.

```python
word.form -> str
```

**Example:**

```python
for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"Found verb: {verb.form}")
```

---

#### lemma

Lemma (dictionary form) of the word.

```python
word.lemma -> str
```

**Example:**

```python
for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form} -> {verb.lemma}")
    # Output: "running -> run"
```

---

#### pos

Universal POS tag (UPOS field in CoNLL-U).

```python
word.pos -> str
```

**Example:**

```python
verb = tree.get_word(match["V"])
if verb.pos == "VERB":
    print(f"Confirmed verb: {verb.form}")
```

**Notes:**

- Returns the Universal Dependencies UPOS tag
- See [Universal POS tags](https://universaldependencies.org/u/pos/) for tag list

---

#### xpos

Language-specific POS tag (XPOS field in CoNLL-U).

```python
word.xpos -> str | None
```

**Returns:**

- XPOS tag string, or None if unspecified (`_` in CoNLL-U)

**Example:**

```python
verb = tree.get_word(match["V"])
if verb.xpos:
    print(f"{verb.pos} (language-specific: {verb.xpos})")
    # Output: "VERB (language-specific: VBG)"
```

**Notes:**

- Tag set varies by language and treebank
- Returns None when CoNLL-U has `_` in XPOS field

---

#### deprel

Dependency relation to parent.

```python
word.deprel -> str
```

**Example:**

```python
for child in verb.children():
    print(f"{child.form}: {child.deprel}")
    # Output: "quickly: advmod"
    # Output: "ball: obj"
```

**Notes:**

- See [Universal Dependencies relations](https://universaldependencies.org/u/dep/) for relation types
- Root words typically have `deprel="root"`

---

#### head

Head word ID (0-based index of parent word).

```python
word.head -> int | None
```

**Returns:**

- Parent word ID (0-based), or None for root words

**Example:**

```python
word = tree.get_word(5)
if word.head is not None:
    parent = tree.get_word(word.head)
    print(f"{word.form} is child of {parent.form}")
else:
    print(f"{word.form} is root")
```

**See also:** parent()

---

### Methods

#### parent()

Get the parent word in the dependency tree.

```python
word.parent() -> Word | None
```

**Returns:**

- Parent Word object, or None for root words

**Example:**

```python
word = tree.get_word(match["N"])
parent = word.parent()
if parent:
    print(f"{word.form} ({word.deprel}) <- {parent.form}")
else:
    print(f"{word.form} is root")
```

**See also:** children(), head

---

#### children()

Get all child words (dependents).

```python
word.children() -> list[Word]
```

**Returns:**

- List of child Word objects (may be empty)

**Example:**

```python
verb = tree.get_word(match["V"])
for child in verb.children():
    print(f"{child.form} ({child.deprel})")
# Output: "quickly (advmod)"
# Output: "ball (obj)"
```

**See also:** children_by_deprel(), parent()

---

#### children_by_deprel()

Get children with a specific dependency relation.

```python
word.children_by_deprel(deprel: str) -> list[Word]
```

**Parameters:**

- `deprel` (str) - Dependency relation name (e.g., `"nsubj"`, `"obj"`)

**Returns:**

- List of child Word objects with the specified deprel (may be empty)

**Example:**

```python
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])

    # Get subjects
    subjects = verb.children_by_deprel("nsubj")
    if subjects:
        print(f"Subject: {subjects[0].form}")

    # Get objects
    objects = verb.children_by_deprel("obj")
    for obj in objects:
        print(f"Object: {obj.form}")

    # Check for passive auxiliary
    if verb.children_by_deprel("aux:pass"):
        print("Passive construction")
```

**Notes:**

- Returns empty list if no children match the relation
- More efficient than filtering results from `children()`
- Useful for checking existence: `if word.children_by_deprel("xcomp"):`

**See also:** children()

---

#### children_ids

List of child word IDs (0-based indices).

```python
word.children_ids -> list[int]
```

**Returns:**

- List of child word IDs

**Example:**

```python
verb = tree.get_word(match["V"])
print(f"Children at positions: {verb.children_ids}")
# Output: "Children at positions: [3, 7, 9]"
```

**Notes:**

- Returns IDs, not Word objects
- Use `tree.get_word(id)` to get Word objects
- More efficient than `children()` if you only need IDs

---

## Examples

### Navigating dependency structure

```python
# Find verbs with subjects and objects
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Subj [upos="NOUN"];
        Obj [upos="NOUN"];
        V <-[nsubj]- Subj;
        V -[obj]-> Obj;
    }
""")

for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    subj = tree.get_word(match["Subj"])
    obj = tree.get_word(match["Obj"])

    print(f"{subj.form} {verb.form} {obj.form}")
    print(f"Sentence: {tree.sentence_text}")
```

### Exploring tree structure

```python
for tree in treesearch.trees("corpus.conllu"):
    # Iterate through all words
    for word_id in range(len(tree)):
        word = tree.get_word(word_id)
        if word:
            # Show word and its parent
            parent = word.parent()
            if parent:
                print(f"{word.form} -> {parent.form} ({word.deprel})")
            else:
                print(f"{word.form} (root)")
```

### Analyzing argument structure

```python
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])

    # Get all dependents by type
    subjects = verb.children_by_deprel("nsubj")
    objects = verb.children_by_deprel("obj")
    indirect_objects = verb.children_by_deprel("iobj")
    clausal_complements = verb.children_by_deprel("xcomp")

    # Analyze valency
    print(f"Verb: {verb.lemma}")
    print(f"  Subjects: {len(subjects)}")
    print(f"  Objects: {len(objects)}")
    print(f"  Clausal complements: {len(clausal_complements)}")
```

### Finding paths between words

```python
# Find control constructions (help + to-infinitive)
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

    # Find path between main verb and infinitive
    path = tree.find_path(main, inf)
    if path:
        path_forms = " -> ".join(w.form for w in path)
        print(f"Path: {path_forms}")
```

### Accessing metadata

```python
for tree in treesearch.trees("corpus.conllu"):
    # Get sentence metadata
    sent_id = tree.metadata.get('sent_id', 'unknown')
    text = tree.sentence_text or "no text"

    # Count words by POS
    pos_counts = {}
    for word_id in range(len(tree)):
        word = tree.get_word(word_id)
        if word:
            pos_counts[word.pos] = pos_counts.get(word.pos, 0) + 1

    print(f"Sentence {sent_id}: {text}")
    print(f"POS distribution: {pos_counts}")
```

## See also

- [Treebank](treebank.md) - Treebank class for collections of trees
- [Functions](functions.md) - Functions for reading and searching
- [Pattern](pattern.md) - Pattern class for compiled queries
