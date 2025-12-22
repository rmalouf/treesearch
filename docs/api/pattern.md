# Pattern class

Compiled query pattern for tree matching.

## Overview

Pattern objects represent compiled queries created by `parse_query()`. Patterns are opaque, reusable, and thread-safe objects used with search functions to find matches in dependency trees.

Patterns should be compiled once and reused across multiple searches for best performance.

## Properties

---

## Creating patterns

Patterns are created using the `parse_query()` function:

```python
pattern = treesearch.parse_query(query_string)
```

**Example:**

```python
# Simple pattern
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')

# Complex pattern with multiple constraints
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Subj [upos="NOUN"];
        Obj [upos="NOUN"];
        V <-[nsubj]- Subj;
        V -[obj]-> Obj;
    }
""")
```

**See also:** [parse_query()](functions.md#parse_query)

---

## Using patterns

Patterns are used with search functions and methods.

### With search()

Search a single tree.

```python
for match in treesearch.search(tree, pattern):
    # Process match
    verb = tree.get_word(match["V"])
```

**See also:** [search()](functions.md#search)

---

### With get_matches()

Search a single CoNLL-U file.

```python
for tree, match in treesearch.get_matches("corpus.conllu", pattern):
    # Process match
    verb = tree.get_word(match["V"])
```

**See also:** [get_matches()](functions.md#get_matches)

---

### With get_matchess()

Search multiple CoNLL-U files with automatic parallel processing.

```python
for tree, match in treesearch.get_matchess("data/*.conllu", pattern):
    # Process match
    verb = tree.get_word(match["V"])
```

**See also:** [get_matchess()](functions.md#get_matchess)

---

### With Treebank.matches()

Search a treebank using the object-oriented API.

```python
tb = treesearch.Treebank.from_files("data/*.conllu")
for tree, match in tb.matches(pattern):
    # Process match
    verb = tree.get_word(match["V"])
```

**See also:** [Treebank.matches()](treebank.md#matches)

---

## Pattern reuse

Compile patterns once and reuse them across multiple searches.

### Best practice

```python
# Compile once
pattern = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        N [upos="NOUN"];
        V -[obj]-> N;
    }
""")

# Reuse across files
for file in file_list:
    for tree, match in treesearch.get_matches(file, pattern):
        process(match)
```

### Anti-pattern

```python
# Bad: Re-compiling every iteration
for file in file_list:
    pattern = treesearch.parse_query(query)  # Wasteful!
    for tree, match in treesearch.get_matches(file, pattern):
        process(match)
```

---

## Pattern properties

### Thread safety

Patterns are thread-safe and can be shared across threads.

```python
from concurrent.futures import ThreadPoolExecutor

pattern = treesearch.parse_query(query)

def get_matches(path):
    return list(treesearch.get_matches(path, pattern))

with ThreadPoolExecutor() as executor:
    results = executor.map(get_matches, file_paths)
```

---

### Immutability

Patterns cannot be modified after creation.

```python
pattern = treesearch.parse_query('MATCH { V [upos="VERB"]; }')
# No way to change constraints or add variables
# Must create new pattern instead
```

---

### Memory efficiency

Patterns are lightweight and cheap to clone.

```python
pattern1 = treesearch.parse_query(query)
pattern2 = pattern1  # Shares underlying data
```

---

## Examples

### Passive constructions

```python
passive = treesearch.parse_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V <-[aux:pass]- Aux;
    }
""")

for tree, match in treesearch.get_matches("corpus.conllu", passive):
    verb = tree.get_word(match["V"])
    print(f"Passive: {tree.sentence_text}")
```

### Control verbs

```python
help_infinitive = treesearch.parse_query("""
    MATCH {
        Main [lemma="help"];
        Inf [upos="VERB"];
        Main -[xcomp]-> Inf;
    }
""")

for tree, match in treesearch.get_matchess("data/*.conllu", help_infinitive):
    main = tree.get_word(match["Main"])
    inf = tree.get_word(match["Inf"])
    print(f"{main.form} ... {inf.form}: {tree.sentence_text}")
```

### Relative clauses

```python
relative = treesearch.parse_query("""
    MATCH {
        Head [upos="NOUN"];
        Rel [upos="PRON"];
        V [upos="VERB"];
        Head -[acl:relcl]-> V;
        V <-[nsubj]- Rel;
    }
""")

tb = treesearch.Treebank.from_files("data/*.conllu")
for tree, match in tb.matches(relative):
    head = tree.get_word(match["Head"])
    rel = tree.get_word(match["Rel"])
    verb = tree.get_word(match["V"])
    print(f"{head.form} {rel.form} {verb.form}")
```

### Checking pattern validity

```python
try:
    pattern = treesearch.parse_query("MATCH { V [invalid syntax] }")
except ValueError as e:
    print(f"Invalid query: {e}")
```

---

## Pattern compilation

When you call `parse_query()`, treesearch performs several steps:

1. **Parsing** - Converts query string into an abstract syntax tree
2. **Validation** - Checks variable names and constraint syntax
3. **Optimization** - Determines efficient variable ordering using MRV heuristic
4. **Compilation** - Creates internal representation for constraint satisfaction

The compiled pattern contains:

- Variable definitions and constraints (lemma, form, POS, deprel)
- Edge constraints (dependency relations and precedence)
- Optimized variable ordering for search efficiency
- Precomputed constraint checks

---

## Error handling

Pattern compilation can fail with `ValueError` for invalid queries.

```python
try:
    pattern = treesearch.parse_query("MATCH { V [upos='VERB' }")  # Missing ]
except ValueError as e:
    print(f"Parse error: {e}")

try:
    pattern = treesearch.parse_query("MATCH { V [nosuchfield='X']; }")
except ValueError as e:
    print(f"Invalid constraint: {e}")
```

**See also:** [Query language guide](../guide/query-language.md)

---

## See also

- [parse_query()](functions.md#parse_query) - Creating patterns
- [Query language](../guide/query-language.md) - Query syntax reference
- [search()](functions.md#search) - Using patterns with single trees
- [get_matches()](functions.md#get_matches) - Using patterns with files
- [get_matchess()](functions.md#get_matchess) - Using patterns with multiple files
- [Treebank](treebank.md) - Object-oriented pattern search
