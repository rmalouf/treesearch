# Quick Start

Get up and running with treesearch in 5 minutes.

## Your First Search

### Step 1: Import treesearch

```python
import treesearch
```

### Step 2: Write a Query

Let's find all verbs in our corpus:

```python
query = """
    V [upos="VERB"];
"""
```

### Step 3: Compile the Pattern

```python
pattern = treesearch.parse_query(query)
```

### Step 4: Search a File

```python
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"Found verb: {verb.form} (lemma: {verb.lemma})")
```

## A More Complex Example

Let's find verb-object constructions:

```python
import treesearch

# Define the pattern: verb with nominal object
query = """
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[obj]-> N;
"""

# Compile pattern
pattern = treesearch.parse_query(query)

# Search corpus
for tree, match in treesearch.search_file("corpus.conllu", pattern):
    verb = tree.get_word(match["V"])
    noun = tree.get_word(match["N"])

    print(f"{verb.form} -> {noun.form}")
    print(f"Sentence: {tree.sentence_text}")
    print()
```

## Understanding Matches

Matches are returned as dictionaries mapping variable names to word IDs:

```python
match = {"V": 3, "N": 7}  # Example match
```

Use these IDs to retrieve Word objects from the tree:

```python
verb = tree.get_word(match["V"])
print(verb.form)      # Surface form
print(verb.lemma)     # Dictionary form
print(verb.pos)       # POS tag
print(verb.deprel)    # Dependency relation
```

## Working with Multiple Files

Search across many files in parallel:

```python
pattern = treesearch.parse_query('V [upos="VERB"];')

for tree, match in treesearch.search_files("data/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    print(f"{verb.form}: {tree.sentence_text}")
```

## Basic Query Syntax

### Node Constraints

Specify properties words must have:

```python
# Match any verb
V [upos="VERB"];

# Match specific lemma
Help [lemma="help"];

# Multiple constraints (AND)
Past [upos="VERB", xpos="VBD"];
```

### Edge Constraints

Specify dependency relationships:

```python
# V has child N with deprel "obj"
V -[obj]-> N;

# V has any child To
V -> To;

# V is parent of N (equivalent to above)
N <-[obj]- V;
```

### Empty Constraints

Match any word:

```python
# Any word
X [];
```

## Common Patterns

### Finding Subjects

```python
query = """
    V [upos="VERB"];
    Subj [upos="NOUN"];
    V -[nsubj]-> Subj;
"""
```

### Auxiliary Constructions

```python
query = """
    Main [upos="VERB"];
    Aux [lemma="have"];
    Main <-[aux]- Aux;
"""
```

### Control Verbs

```python
query = """
    Main [upos="VERB"];
    Comp [upos="VERB"];
    Main -[xcomp]-> Comp;
"""
```

### Excluding Patterns with Negation

Find verbs that **don't** have objects:

```python
query = """
    V [upos="VERB"];
    Obj [];
    V !-[obj]-> Obj;
"""
```

Find words that have no incoming edges (root words):

```python
query = """
    Root [];
    _ !-> Root;
"""
```

## Next Steps

- **[Query Language](../guide/query-language.md)** - Complete syntax reference
- **[Searching Guide](../guide/searching.md)** - Advanced search techniques
- **[Working with Results](../guide/results.md)** - Navigate trees and extract data
- **[API Reference](../api/functions.md)** - Complete function documentation

## Example Data

If you don't have CoNLL-U data yet, you can:

1. Use the example files in the `examples/` directory
2. Download from [Universal Dependencies](https://universaldependencies.org/)
3. Parse your own text using tools like [spaCy](https://spacy.io/) or [Stanza](https://stanfordnlp.github.io/stanza/)
