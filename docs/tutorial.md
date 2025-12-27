# Tutorial

Complete walkthrough of treesearch, from installation to advanced usage.

## Installation

```bash
pip install treesearch
```

Or install from source (requires Rust toolchain):

```bash
git clone https://github.com/rmalouf/treesearch.git
cd treesearch
pip install -e .
```

## Basic Usage

```python
import treesearch as ts

# Load a treebank
treebank = ts.load("corpus/*.conllu")

# Search with a query
for tree, match in treebank.search('MATCH { V [upos="VERB"]; }'):
    verb = tree.word(match["V"])
    print(f"{verb.lemma}: {tree.sentence_text}")
```

## Writing Queries

Queries define patterns to match in dependency trees. Every query has the form:

```
MATCH {
    Variable [constraints];
    ...
    edges;
}
```

### Node Constraints

```python
# Match by POS tag
'MATCH { V [upos="VERB"]; }'

# Match by lemma
'MATCH { Help [lemma="help"]; }'

# Multiple constraints (AND)
'MATCH { V [upos="VERB" & lemma="run"]; }'

# Any word (no constraints)
'MATCH { X []; }'

# Morphological features
'MATCH { Past [feats.Tense="Past"]; }'

# Negation
'MATCH { NotVerb [upos!="VERB"]; }'
```

### Edge Constraints

```python
# Specific dependency relation
'MATCH { V []; N []; V -[obj]-> N; }'

# Any dependency
'MATCH { V []; N []; V -> N; }'

# Negative: V does NOT have obj edge to N
'MATCH { V []; N []; V !-[obj]-> N; }'
```

### Precedence

```python
# V immediately precedes N
'MATCH { V []; N []; V < N; }'

# V precedes N (anywhere before)
'MATCH { V []; N []; V << N; }'
```

### Anonymous Variables

Use `_` when you need to check for existence without binding:

```python
# Verb with any subject (don't need the subject itself)
'MATCH { V [upos="VERB"]; V -[nsubj]-> _; }'

# Verb WITHOUT an object
'MATCH { V [upos="VERB"]; V !-[obj]-> _; }'
```

## Working with Results

Matches are dictionaries mapping variable names to word IDs (0-indexed):

```python
for tree, match in treebank.search(query):
    # Get words by ID
    verb = tree.word(match["V"])  # or tree[match["V"]]

    # Word properties
    print(verb.form)      # surface form
    print(verb.lemma)     # dictionary form
    print(verb.upos)      # universal POS
    print(verb.deprel)    # dependency relation
    print(verb.feats)     # morphological features dict

    # Navigate tree
    parent = verb.parent()
    children = verb.children()
    objects = verb.children_by_deprel("obj")

    # Tree properties
    print(tree.sentence_text)
    print(tree.metadata)
```

## Example: Finding Passives

```python
import treesearch as ts
from collections import Counter

query = """
MATCH {
    V [upos="VERB"];
    Subj [];
    V -[aux:pass]-> _;
    V -[nsubj:pass]-> Subj;
}
"""

treebank = ts.load("corpus/*.conllu")
verb_counts = Counter()

for tree, match in treebank.search(query):
    verb = tree.word(match["V"])
    verb_counts[verb.lemma] += 1

for lemma, count in verb_counts.most_common(10):
    print(f"{lemma}: {count}")
```

## Example: Collecting Examples

```python
import treesearch as ts
import json

query = 'MATCH { V [lemma="help"]; V -[xcomp]-> Inf; }'

examples = []
for tree, match in ts.search("corpus/*.conllu", query):
    examples.append({
        "sentence": tree.sentence_text,
        "verb": tree.word(match["V"]).form,
        "infinitive": tree.word(match["Inf"]).form
    })
    if len(examples) >= 100:
        break

with open("examples.json", "w") as f:
    json.dump(examples, f, indent=2)
```

## Performance Tips

1. **Compile once** when reusing patterns:
   ```python
   pattern = ts.compile_query(query)  # compile once
   for file in files:
       for tree, match in ts.search(file, pattern):
           ...
   ```

2. **Use `ordered=False`** when order doesn't matter:
   ```python
   for tree, match in treebank.search(query, ordered=False):
       ...
   ```

3. **Use gzip** - `.conllu.gz` files often faster due to less I/O

4. **Stream results** - don't collect into lists unless needed

## Next Steps

- [Query Language Reference](query-language.md) - Complete syntax
- [API Reference](api.md) - All functions and classes
