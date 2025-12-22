# Performance Tips

Optimize treesearch for large-scale corpus linguistics research.

## Query Optimization

### Parse Once, Search Many

```python
# Good: compile once
pattern = treesearch.parse_query(query)
for file in files:
    for tree, match in treesearch.get_matches(file, pattern):
        process(match)

# Bad: re-compile every time
for file in files:
    pattern = treesearch.parse_query(query)  # Wasteful!
    for tree, match in treesearch.get_matches(file, pattern):
        process(match)
```

### Specific Constraints

More specific queries run faster:

```python
# Slower: matches many words
query = "MATCH { V []; }"

# Faster: specific constraint
query = 'MATCH { V [upos="VERB"]; }'

# Fastest: multiple constraints
query = 'MATCH { V [upos="VERB", lemma="run"]; }'
```

## File Processing

### Use get_matches()

Direct searching is more efficient:

```python
# Best: direct search with automatic parallelization
for tree, match in treesearch.get_matches("*.conllu", pattern):
    process(match)

# Slower: manual reading and searching
for tree in treesearch.get_trees("*.conllu"):
    for match in treesearch.search(tree, pattern):
        process(match)
```

### Automatic Parallel Processing

Multi-file operations automatically use parallel processing for optimal performance:

```python
# Automatic parallel processing
for tree, match in treesearch.get_matches("*.conllu", pattern):
    process(match)
```

### Use Gzip Compression

Compressed files are often faster due to reduced I/O:

```python
# .conllu.gz files are automatically decompressed
for tree in treesearch.get_trees("corpus.conllu.gz"):
    process(tree)
```

## Memory Efficiency

### Use Iterators

Don't collect all results:

```python
# Good: process as you go
for tree, match in treesearch.get_matches("*.conllu", pattern):
    process(match)

# Bad: collect everything first
all_matches = list(treesearch.get_matches("*.conllu", pattern))  # High memory!
for tree, match in all_matches:
    process(match)
```

### Limit Results

Stop early if you don't need all matches:

```python
count = 0
for tree, match in treesearch.get_matches("*.conllu", pattern):
    process(match)
    count += 1
    if count >= 1000:
        break
```

## Query Design

### Start Specific

Begin with the most constrained variables:

```python
# Better: start with specific verb
query = """
MATCH {
    V [lemma="help"];  # Very specific
    N [upos="NOUN"];   # Less specific
    V -> N;
}
"""

# Worse: start with unconstrained
query = """
MATCH {
    N [];              # Matches everything!
    V [lemma="help"];
    V -> N;
}
"""
```

### Avoid Unnecessary Variables

Only declare variables you need:

```python
# Good: minimal variables
query = """
MATCH {
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[obj]-> N;
}
"""

# Wasteful: extra variable not used in results
query = """
MATCH {
    V [upos="VERB"];
    N [upos="NOUN"];
    X [];  # Not used!
    V -[obj]-> N;
}
"""
```

## Benchmarking

### Time Your Queries

```python
import time

start = time.time()
count = 0

for tree, match in treesearch.get_matches("*.conllu", pattern):
    count += 1

elapsed = time.time() - start
print(f"Found {count} matches in {elapsed:.2f}s")
print(f"Rate: {count/elapsed:.0f} matches/sec")
```

### Profile Different Approaches

```python
import time

queries = {
    "broad": 'MATCH { V []; }',
    "medium": 'MATCH { V [upos="VERB"]; }',
    "narrow": 'MATCH { V [upos="VERB", lemma="run"]; }'
}

for name, query in queries.items():
    pattern = treesearch.parse_query(query)
    start = time.time()

    count = sum(1 for _ in treesearch.get_matches("sample.conllu", pattern))

    elapsed = time.time() - start
    print(f"{name}: {count} matches in {elapsed:.2f}s")
```

## Expected Performance

On a modern machine (2023):

- **Small corpus** (1M tokens): Milliseconds per query
- **Medium corpus** (100M tokens): Seconds per query
- **Large corpus** (500M+ tokens): Minutes per query with parallelization

Factors affecting speed:

- Query specificity (more constraints = faster)
- Number of variables (fewer = faster)
- File format (gzip is often faster due to less I/O)
- Number of CPU cores (more = faster with automatic parallelization)

## Next Steps

- [Architecture](architecture.md) - How treesearch works
- [Query Language](../guide/query-language.md) - Writing efficient queries
