# Tree & Word API Reference

Complete reference for Tree and Word objects.

## Tree

Represents a dependency tree (parsed sentence) from a CoNLL-U file.

### Properties

#### sentence_text

Reconstructed sentence text.

```python
tree.sentence_text -> str | None
```

**Returns:**

- Sentence text if available from CoNLL-U metadata, None otherwise

**Example:**

```python
for tree in treesearch.read_trees("corpus.conllu"):
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
print(tree.metadata)
# {'sent_id': '1', 'text': 'Hello world.'}
```

---

### Methods

#### get_word()

Get a word by its ID.

```python
tree.get_word(id: int) -> Word | None
```

**Parameters:**

- `id` (int): Word index (0-based)

**Returns:**

- Word object if ID is valid, None otherwise

**Example:**

```python
word = tree.get_word(3)
if word:
    print(f"{word.form} ({word.pos})")
```

---

#### find_path()

Find the dependency path between two words.

```python
tree.find_path(x: Word, y: Word) -> list[Word] | None
```

**Parameters:**

- `x` (Word): Starting word
- `y` (Word): Target word

**Returns:**

- List of words forming the path from x to y, or None if no path exists

**Example:**

```python
verb = tree.get_word(match["V"])
noun = tree.get_word(match["N"])

path = tree.find_path(verb, noun)
if path:
    print(" -> ".join(w.form for w in path))
```

---

#### \_\_len\_\_()

Get the number of words in the tree.

```python
len(tree) -> int
```

**Returns:**

- Number of words (including root)

**Example:**

```python
print(f"Tree has {len(tree)} words")
```

---

## Word

Represents a single word (node) in a dependency tree.

### Properties

All properties are read-only attributes.

#### id

Word ID (0-based index in tree).

```python
word.id -> int
```

#### token_id

Token ID from CoNLL-U file (1-based).

```python
word.token_id -> int
```

#### form

Surface form of the word.

```python
word.form -> str
```

**Example:**

```python
print(word.form)  # "running"
```

#### lemma

Lemma (dictionary form) of the word.

```python
word.lemma -> str
```

**Example:**

```python
print(word.lemma)  # "run"
```

#### pos

Universal POS tag (UPOS field in CoNLL-U).

```python
word.pos -> str
```

**Example:**

```python
print(word.pos)  # "VERB"
```

#### xpos

Language-specific POS tag (XPOS field).

```python
word.xpos -> str | None
```

**Returns:**

- XPOS tag, or None if unspecified (`_`)

**Example:**

```python
if word.xpos:
    print(word.xpos)  # "VBG"
```

#### deprel

Dependency relation to parent.

```python
word.deprel -> str
```

**Example:**

```python
print(word.deprel)  # "nsubj"
```

#### head

Head word ID (0-based index of parent word).

```python
word.head -> int | None
```

**Returns:**

- Parent word ID, or None for root words

**Example:**

```python
if word.head is not None:
    parent = tree.get_word(word.head)
```

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
parent = word.parent()
if parent:
    print(f"{word.form} is child of {parent.form}")
else:
    print(f"{word.form} is root")
```

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
for child in word.children():
    print(f"{child.form} ({child.deprel})")
```

---

#### children_by_deprel()

Get children with a specific dependency relation.

```python
word.children_by_deprel(deprel: str) -> list[Word]
```

**Parameters:**

- `deprel` (str): Dependency relation name (e.g., "nsubj", "obj")

**Returns:**

- List of child Word objects with the specified deprel (may be empty)

**Example:**

```python
# Get all objects of a verb
objects = verb.children_by_deprel("obj")
for obj in objects:
    print(f"Object: {obj.form}")

# Get subject
subjects = verb.children_by_deprel("nsubj")
if subjects:
    print(f"Subject: {subjects[0].form}")
```

---

#### children_ids

List of child word IDs (0-based indices).

```python
word.children_ids -> list[int]
```

**Example:**

```python
print(word.children_ids)  # [4, 7, 9]
```

---

## Common Patterns

### Iterating Through a Tree

```python
for tree in treesearch.read_trees("corpus.conllu"):
    for word_id in range(len(tree)):
        word = tree.get_word(word_id)
        if word:
            print(f"{word.form}: {word.pos}")
```

### Finding Specific Relations

```python
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])

    # Get all children
    for child in verb.children():
        print(f"Dependent: {child.form} ({child.deprel})")

    # Get specific relation
    subjects = verb.children_by_deprel("nsubj")
    objects = verb.children_by_deprel("obj")
```

### Navigating Tree Structure

```python
word = tree.get_word(5)

# Go up
parent = word.parent()
if parent:
    grandparent = parent.parent()

# Go down
for child in word.children():
    for grandchild in child.children():
        print(grandchild.form)
```

### Accessing Sentence Information

```python
for tree in treesearch.read_trees("corpus.conllu"):
    # Get sentence text
    print(f"Sentence: {tree.sentence_text}")

    # Get metadata
    if 'sent_id' in tree.metadata:
        print(f"ID: {tree.metadata['sent_id']}")

    # Get word count
    print(f"Words: {len(tree)}")
```

## Next Steps

- [Functions API](functions.md) - Search and read functions
- [Pattern API](pattern.md) - Pattern object reference
- [Working with Results](../guide/results.md) - Practical examples
