# Extracting Examples

Get representative sentences from your corpus.

## Basic Extraction

```python
import treesearch

pattern = treesearch.parse_query("MATCH { V [upos="VERB"]; }")

examples = []
for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    examples.append(tree.sentence_text)

    if len(examples) >= 10:
        break

for example in examples:
    print(example)
```

## Filtering Examples

Get specific types of examples:

```python
pattern = treesearch.parse_query("""
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[obj]-> N;
""")

examples = []

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    verb = tree.get_word(match["V"])

    # Filter for past tense
    if verb.xpos == "VBD":
        examples.append({
            "sentence": tree.sentence_text,
            "verb": verb.form,
            "lemma": verb.lemma
        })

    if len(examples) >= 20:
        break
```

## Diverse Examples

Get one example per lemma:

```python
from collections import defaultdict

pattern = treesearch.parse_query("MATCH { V [upos="VERB"]; }")
examples_by_lemma = defaultdict(list)

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    verb = tree.get_word(match["V"])

    if len(examples_by_lemma[verb.lemma]) < 3:
        examples_by_lemma[verb.lemma].append(tree.sentence_text)

# Print examples per lemma
for lemma, sentences in examples_by_lemma.items():
    print(f"\n{lemma}:")
    for sent in sentences:
        print(f"  {sent}")
```

## Saving Examples

```python
import json

pattern = treesearch.parse_query(query)
examples = []

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    example = {
        "sentence": tree.sentence_text,
        "metadata": tree.metadata,
        "match": {var: tree.get_word(id).form for var, id in match.items()}
    }
    examples.append(example)

    if len(examples) >= 100:
        break

with open("examples.json", "w") as f:
    json.dump(examples, f, indent=2)
```

## Next Steps

- [Finding Constructions](constructions.md) - Locate syntactic patterns
- [Frequency Analysis](frequency.md) - Count pattern frequencies
