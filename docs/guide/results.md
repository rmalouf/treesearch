# Working with Results

Guide to navigating trees and extracting data from search results.

## Match Dictionaries

Matches map variable names to word IDs:

```python
for tree, match in treesearch.search("corpus.conllu", pattern):
    print(match)
    # {"V": 3, "N": 7}
```

## Accessing Matched Words

Use `tree.get_word()` with match IDs:

```python
for tree, match in treesearch.search("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    noun = tree.get_word(match["N"])

    print(f"{verb.form} -> {noun.form}")
```

## Word Properties

Access linguistic properties:

```python
word = tree.get_word(match["V"])

print(word.form)      # Surface form: "running"
print(word.lemma)     # Lemma: "run"
print(word.pos)       # POS tag: "VERB"
print(word.xpos)      # Language-specific POS: "VBG"
print(word.deprel)    # Dependency relation: "root"
print(word.head)      # Parent word ID: 0
```

## Navigating the Tree

### Parent

```python
word = tree.get_word(match["V"])
parent = word.parent()

if parent:
    print(f"{word.form} is child of {parent.form}")
```

### Children

```python
for child in word.children():
    print(f"{child.form} ({child.deprel})")
```

### Children by Relation

```python
# Get all objects
objects = verb.children_by_deprel("obj")

# Get all subjects
subjects = verb.children_by_deprel("nsubj")

# Get auxiliaries
auxs = verb.children_by_deprel("aux")
```

## Dependency Paths

Find the path between two words:

```python
verb = tree.get_word(match["V"])
noun = tree.get_word(match["N"])

path = tree.find_path(verb, noun)
if path:
    print(" -> ".join(w.form for w in path))
```

## Extracting Context

### Full Sentence

```python
print(tree.sentence_text)
```

### Metadata

```python
print(tree.metadata)
# {'sent_id': '1', 'text': 'Hello world.'}
```

### Word Window

Get surrounding words:

```python
word_id = match["V"]
start = max(0, word_id - 2)
end = min(len(tree), word_id + 3)

for i in range(start, end):
    w = tree.get_word(i)
    if w:
        marker = "**" if i == word_id else ""
        print(f"{marker}{w.form}{marker}", end=" ")
print()
```

## Filtering Results

### By Word Properties

```python
for tree, match in treesearch.search("*.conllu", pattern):
    verb = tree.get_word(match["V"])

    # Filter by tense
    if verb.xpos == "VBD":  # Past tense
        process(tree, match)
```

### By Tree Properties

```python
for tree, match in treesearch.search("*.conllu", pattern):
    # Filter by sentence length
    if len(tree) > 20:
        process(tree, match)

    # Filter by metadata
    if 'genre' in tree.metadata:
        if tree.metadata['genre'] == 'news':
            process(tree, match)
```

### By Context

```python
for tree, match in treesearch.search("*.conllu", pattern):
    verb = tree.get_word(match["V"])

    # Filter by parent
    parent = verb.parent()
    if parent and parent.lemma == "want":
        process(tree, match)

    # Filter by children
    if verb.children_by_deprel("conj"):
        # Skip coordinated verbs
        continue
```

## Collecting Data

### Count Frequencies

```python
from collections import Counter

lemma_counts = Counter()

for tree, match in treesearch.search("*.conllu", pattern):
    verb = tree.get_word(match["V"])
    lemma_counts[verb.lemma] += 1

for lemma, count in lemma_counts.most_common(10):
    print(f"{lemma}: {count}")
```

### Save Examples

```python
examples = []

for tree, match in treesearch.search("*.conllu", pattern):
    example = {
        "sentence": tree.sentence_text,
        "match": {var: tree.get_word(id).form for var, id in match.items()}
    }
    examples.append(example)

    if len(examples) >= 100:
        break
```

## Next Steps

- [Finding Constructions](../workflows/constructions.md) - Real-world examples
- [Frequency Analysis](../workflows/frequency.md) - Count patterns
- [Tree & Word API](../api/tree-word.md) - Complete reference
