# Frequency Analysis

Count construction frequencies across your corpus.

## Basic Counting

```python
import treesearch

pattern = treesearch.parse_query("MATCH { V [upos="VERB"]; }")

count = 0
for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    count += 1

print(f"Total verbs: {count}")
```

## Counting by Lemma

```python
from collections import Counter

pattern = treesearch.parse_query("MATCH { V [upos="VERB"]; }")
verb_counts = Counter()

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    verb_counts[verb.lemma] += 1

# Print top 20
for lemma, count in verb_counts.most_common(20):
    print(f"{lemma}: {count}")
```

## Construction Frequencies

Count specific constructions:

```python
from collections import Counter

query = """
MATCH {
    Help [lemma="help"];
    To [lemma="to"];
    V [upos="VERB"];
    Help -[xcomp]-> To;
    To < V;
}
"""

pattern = treesearch.parse_query(query)
verb_counts = Counter()

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    verb_counts[verb.lemma] += 1

print("help-to-VERB frequencies:")
for lemma, count in verb_counts.most_common(10):
    print(f"  help to {lemma}: {count}")
```

## Distribution Analysis

Track distributions across metadata:

```python
from collections import defaultdict, Counter

pattern = treesearch.parse_query("MATCH { V [upos="VERB"]; }")

# Count by genre
genre_counts = defaultdict(Counter)

for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    verb = tree.get_word(match["V"])
    genre = tree.metadata.get('genre', 'unknown')

    genre_counts[genre][verb.lemma] += 1

# Print per genre
for genre, counts in genre_counts.items():
    print(f"\n{genre}:")
    for lemma, count in counts.most_common(5):
        print(f"  {lemma}: {count}")
```

## Next Steps

- [Finding Constructions](constructions.md) - Locate syntactic patterns
- [Extracting Examples](examples.md) - Get representative sentences
