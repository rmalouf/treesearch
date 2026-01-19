# Tutorial

Complete walkthrough of treesearch, from installation to advanced usage.

## Installation

```bash
pip install treesearch-ud
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

### Regular Expressions

Use `/pattern/` for regex matching (automatically anchored for full-string match):

```python
# Words ending in -ing
'MATCH { Prog [form=/.*ing/]; }'

# Lemmas starting with "run"
'MATCH { V [lemma=/run.*/]; }'

# Match VERB or AUX
'MATCH { V [upos=/VERB|AUX/]; }'

# Modal verbs
'MATCH { M [lemma=/(can|may|must|will|could|might|should|would)/]; }'

# NOT starting with "be"
'MATCH { V [lemma!=/be.*/]; }'

# Past or present tense
'MATCH { V [feats.Tense=/Past|Pres/]; }'
```

**Note:** Patterns are anchored automatically, so `/run/` matches exactly "run" (not "running"). Use `.*` for partial matches: `/run.*/` matches "run", "runs", "running".

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

### EXCEPT Blocks

Reject matches where a condition is true:

```python
# Find verbs that are NOT modified by adverbs
'''
MATCH { V [upos="VERB"]; }
EXCEPT { Adv [upos="ADV"]; V -[advmod]-> Adv; }
'''
```

### OPTIONAL Blocks

Capture additional bindings when available:

```python
# Find verbs, optionally capturing their object
query = '''
MATCH { V [upos="VERB"]; }
OPTIONAL { O []; V -[obj]-> O; }
'''

for tree, match in treebank.search(query):
    verb = tree.word(match["V"])
    if "O" in match:  # check if optional matched
        obj = tree.word(match["O"])
        print(f"{verb.lemma} -> {obj.form}")
    else:
        print(f"{verb.lemma} (intransitive)")
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

## Example: Progressive Constructions (with Regex)

```python
import treesearch as ts
from collections import Counter

# Find all progressive constructions (be + -ing)
query = """
MATCH {
    Aux [lemma=/be.*/];        # be, is, was, were, etc.
    Prog [form=/.*ing/];       # any -ing form
    Aux -[aux]-> Prog;
}
"""

treebank = ts.load("corpus/*.conllu")
verbs = Counter()

for tree, match in treebank.search(query):
    prog = tree.word(match["Prog"])
    verbs[prog.lemma] += 1

print("Most common progressive verbs:")
for lemma, count in verbs.most_common(20):
    print(f"{lemma}: {count}")
```

## Performance Tips

1. **Use `filter()` for existence checks** - when you only need matching trees:
   ```python
   # Efficient: stops after first match per tree
   for tree in treebank.filter(query):
       print(tree.sentence_text)

   # Count matching trees
   count = sum(1 for _ in treebank.filter(query))
   ```

2. **Compile once** when reusing patterns:
   ```python
   pattern = ts.compile_query(query)  # compile once
   for file in files:
       for tree, match in ts.search(file, pattern):
           ...
   ```

3. **Use `ordered=False`** when order doesn't matter:
   ```python
   for tree, match in treebank.search(query, ordered=False):
       ...
   ```

4. **Use gzip** - `.conllu.gz` files often faster due to less I/O

5. **Stream results** - don't collect into lists unless needed

## Next Steps

- [Query Language Reference](query-language.md) - Complete syntax
- [API Reference](api.md) - All functions and classes
