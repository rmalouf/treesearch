# Finding Syntactic Constructions

Step-by-step guide to locating specific syntactic patterns in your corpus.

## Overview

This workflow demonstrates how to:

1. Define a construction pattern
2. Search your corpus
3. Filter and refine results
4. Extract and analyze examples

## Example: help-to-infinitive Construction

We'll find instances of the *help-to* construction:

> "She **helped** us **to win** the game."

### Step 1: Understand the Structure

In dependency parses, this construction typically has:

- A main verb *help*
- The particle *to*
- An infinitive verb
- *help* has *to* as an xcomp child
- The infinitive follows *to*

### Step 2: Write the Query

```python
import treesearch

query = """
    Help [lemma="help"];
    To [lemma="to"];
    V [upos="VERB"];
    Help -[xcomp]-> To;
    To < V;
"""

pattern = treesearch.parse_query(query)
```

### Step 3: Search the Corpus

```python
count = 0

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    help_word = tree.get_word(match["Help"])
    to_word = tree.get_word(match["To"])
    verb = tree.get_word(match["V"])

    print(f"{help_word.form} ... {to_word.form} {verb.form}")
    print(f"  {tree.sentence_text}")
    print()

    count += 1

print(f"Total matches: {count}")
```

### Step 4: Filter Results

Often you'll want to filter matches:

```python
from collections import Counter

# Count infinitive verbs
verb_counts = Counter()

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    help_word = tree.get_word(match["Help"])
    verb = tree.get_word(match["V"])

    # Filter out coordinated constructions
    if help_word.children_by_deprel("conj"):
        continue

    # Filter out passives
    if help_word.children_by_deprel("aux:pass"):
        continue

    # Count the infinitive verb
    verb_counts[verb.lemma] += 1

# Print top 10
for lemma, count in verb_counts.most_common(10):
    print(f"{lemma}: {count}")
```

## More Examples

### Double Object Construction

Find verbs with two objects (e.g., "give him a book"):

```python
query = """
    V [upos="VERB"];
    Obj1 [];
    Obj2 [];
    V -[iobj]-> Obj1;
    V -[obj]-> Obj2;
"""

pattern = treesearch.parse_query(query)

for tree, match in treesearch.search_files("*.conllu", pattern):
    verb = tree.get_word(match["V"])
    obj1 = tree.get_word(match["Obj1"])
    obj2 = tree.get_word(match["Obj2"])

    print(f"{verb.form} {obj1.form} {obj2.form}")
    print(f"  {tree.sentence_text}\n")
```

### Relative Clauses

Find relative clauses:

```python
query = """
    Noun [upos="NOUN"];
    RelPron [upos="PRON"];
    Verb [upos="VERB"];
    Noun -[acl:relcl]-> Verb;
    Verb -[nsubj]-> RelPron;
"""

pattern = treesearch.parse_query(query)

for tree, match in treesearch.search_files("*.conllu", pattern):
    noun = tree.get_word(match["Noun"])
    pron = tree.get_word(match["RelPron"])
    verb = tree.get_word(match["Verb"])

    print(f"{noun.form} {pron.form} {verb.form}")
    print(f"  {tree.sentence_text}\n")
```

### Causative *get*

Find causative uses of *get* (e.g., "get him to do it"):

```python
query = """
    Get [lemma="get"];
    Obj [];
    To [lemma="to"];
    V [upos="VERB"];
    Get -[obj]-> Obj;
    Get -[xcomp]-> To;
    To -[mark]-> V;
"""

pattern = treesearch.parse_query(query)

for tree, match in treesearch.search_files("*.conllu", pattern):
    obj = tree.get_word(match["Obj"])
    verb = tree.get_word(match["V"])

    print(f"get {obj.form} to {verb.form}")
    print(f"  {tree.sentence_text}\n")
```

## Refining Searches

### Add More Constraints

Start broad, then narrow:

```python
# Broad: any verb-object
query = """
    V [upos="VERB"];
    N [];
    V -[obj]-> N;
"""

# Narrower: specific verb
query = """
    V [lemma="eat"];
    N [];
    V -[obj]-> N;
"""

# Even narrower: specific verb and object type
query = """
    V [lemma="eat"];
    N [upos="NOUN"];
    V -[obj]-> N;
"""
```

### Use Precedence

Add word order constraints:

```python
# Verb before object (standard order)
query = """
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[obj]-> N;
    V < N;
"""

# Object before verb (topicalization)
query = """
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[obj]-> N;
    N < V;
"""
```

### Post-Processing Filters

Use Python to filter results:

```python
for tree, match in treesearch.search_files("*.conllu", pattern):
    word = tree.get_word(match["V"])

    # Filter by word properties
    if word.xpos == "VBD":  # Past tense only
        process(tree, match)

    # Filter by context
    parent = word.parent()
    if parent and parent.pos == "VERB":  # Embedded clause
        process(tree, match)

    # Filter by sentence length
    if len(tree) > 20:  # Long sentences only
        process(tree, match)
```

## Collecting Statistics

### Count Matches

```python
count = 0
for tree, match in treesearch.search_files("*.conllu", pattern):
    count += 1
print(f"Total: {count}")
```

### Track Distributions

```python
from collections import Counter

# Verb lemmas
lemma_counts = Counter()

for tree, match in treesearch.search_files("*.conllu", pattern):
    verb = tree.get_word(match["V"])
    lemma_counts[verb.lemma] += 1

for lemma, count in lemma_counts.most_common(20):
    print(f"{lemma}: {count}")
```

### Save Examples

```python
import json

examples = []

for tree, match in treesearch.search_files("*.conllu", pattern):
    example = {
        "sentence": tree.sentence_text,
        "metadata": tree.metadata,
        "match": {var: tree.get_word(id).form for var, id in match.items()}
    }
    examples.append(example)

    if len(examples) >= 100:  # Limit to 100
        break

with open("examples.json", "w") as f:
    json.dump(examples, f, indent=2)
```

## Tips

1. **Start simple**: Begin with minimal constraints, then add more
2. **Test on sample**: Use a small file first to verify your query
3. **Check annotation**: Different corpora use different dependency labels
4. **Filter iteratively**: Use Python to refine results after searching
5. **Document your queries**: Save queries with comments for future reference

## Next Steps

- [Frequency Analysis](frequency.md) - Count construction frequencies
- [Extracting Examples](examples.md) - Get representative sentences
- [Query Language](../guide/query-language.md) - Complete syntax reference
