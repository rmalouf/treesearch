# Quick Start

Get up and running with treesearch in 5 minutes.

## Download Sample Data

To follow along with these examples, let's download a small sample corpus from Universal Dependencies:

```python
import shutil
from urllib.request import urlretrieve


url = "https://raw.githubusercontent.com/UniversalDependencies/UD_English-EWT/master/en_ewt-ud-dev.conllu"
urllib.request.urlretrieve(url, "corpus.conllu")
print("Downloaded corpus.conllu")
```




This downloads a development set with about 2,000 sentences (~25,000 words) from the English Web Treebank, which is perfect for learning and testing.

## Your First Search

### Step 1: Import treesearch and load a treebank

```python
import treesearch

treebank = treesearch.load('UD_English-ParTUT-r2.17/*.conllu.gz')
```

### Step 2: Write a Query

Let's find all verbs in our corpus:

```python
query = """
MATCH {
    V [lemma="note", upos="VERB"];
}
"""
```

### Step 3: Compile the Pattern

```python
pattern = treesearch.compile_query(query)
```

### Step 4: Search a File

```python
for tree, match in treebank.search(pattern):
    word = tree[match['V']]
    print(f'{word.form}: {tree.sentence_text}')
```

## A More Complex Example

Let's find *by* passive constructions:

```python
import treesearch

query = """
MATCH {
    V [upos="VERB"];
    Subj [upos="NOUN"];
    By [lemma="by"];
    Obl [upos="NOUN"];
    V -[aux:pass]-> _;
    V -[nsubj:pass]-> Subj;
    V -[obl]-> Obl;
    Obl -[case]-> By;
}
"""

pattern = treesearch.compile_query(query)


for i,(tree, match) in enumerate(treebank.search(pattern)):
    print(f"{i} {tree[match['Subj']].form} {tree[match['V']].form} by {tree[match['Obl']].form}")
```

## Understanding Matches

Matches are returned as dictionaries mapping variable names to word IDs:

```python
match = {"V": 3, "N": 7}  # Example match
```

Use these IDs to retrieve Word objects from the tree:

```python
verb = tree["V"]
print(verb.form)      # Surface form
print(verb.lemma)     # Dictionary form
print(verb.pos)       # POS tag
print(verb.deprel)    # Dependency relation
```

## Working with Multiple Files

Search across many files in parallel:

```python
pattern = treesearch.compile_query('MATCH { V [upos="VERB"]; }')

for tree, match in treesearch.search("data/*.conllu", pattern):
    verb = tree[match["V"]]
    print(f"{verb.form}: {tree.sentence_text}")
```

## Basic Query Syntax

All queries must be wrapped in a `MATCH { }` block. Below are examples of syntax elements shown within complete queries:

### Node Constraints

Specify properties words must have:

```python
# Match any verb
MATCH {
    V [upos="VERB"];
}

# Match specific lemma
MATCH {
    Help [lemma="help"];
}

# Multiple constraints (AND)
MATCH {
    Past [upos="VERB", xpos="VBD"];
}
```

### Edge Constraints

Specify dependency relationships:

```python
# V has child N with deprel "obj"
MATCH {
    V [];
    N [];
    V -[obj]-> N;
}

# V has any child To
MATCH {
    V [];
    To [];
    V -> To;
}
```

### Empty Constraints

Match any word:

```python
# Any word
MATCH {
    X [];
}
```

## Common Patterns

### Finding Subjects

```python
query = """
MATCH {
    V [upos="VERB"];
    Subj [upos="NOUN"];
    V -[nsubj]-> Subj;
}
"""
```

### Auxiliary Constructions

```python
query = """
MATCH {
    Main [upos="VERB"];
    Aux [lemma="have", upos="AUX"];
    Main -[aux]-> Aux;
}
"""
```

### Control Verbs

```python
query = """
MATCH {
    Main [upos="VERB"];
    Comp [upos="VERB"];
    Main -[xcomp]-> Comp;
}
"""
```

### Excluding Patterns with Negation

Intransitive verbs (verbs that **don't** have objects):

```python
query = """
MATCH {
    V [upos="VERB"];
    V -[nsubj]-> _;
    V !-[obj]-> _;
}
"""
```

Find words that have no incoming edges (root words):

```python
query = """
MATCH {
    Root [];
    _ !-> Root;
}
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
