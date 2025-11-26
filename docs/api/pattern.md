# Pattern API Reference

Complete reference for Pattern objects.

## Pattern

A compiled query pattern for tree matching. Created by `parse_query()` and used with search functions.

### Overview

Patterns are opaque objects that represent compiled queries. They should be:

- Created once with `parse_query()`
- Reused across multiple searches
- Treated as immutable

### Properties

#### n_vars

Number of variables in the pattern.

```python
pattern.n_vars -> int
```

**Returns:**

- Count of variables defined in the query

**Example:**

```python
pattern = treesearch.parse_query("""
    V [upos="VERB"];
    N [upos="NOUN"];
    V -[obj]-> N;
""")

print(pattern.n_vars)  # 2
```

---

## Creating Patterns

Use `parse_query()` to create patterns:

```python
pattern = treesearch.parse_query(query_string)
```

See [parse_query()](functions.md#parse_query) for details.

---

## Using Patterns

Patterns are used with search functions:

### With search()

```python
for match in treesearch.search(tree, pattern):
    # Process matches
    pass
```

### With search_file()

```python
for tree, match in treesearch.search_file("file.conllu", pattern):
    # Process matches
    pass
```

### With search_files()

```python
for tree, match in treesearch.search_files("*.conllu", pattern):
    # Process matches
    pass
```

---

## Pattern Reuse

**Best practice:** Compile patterns once, use many times.

```python
# Good: Compile once
pattern = treesearch.parse_query(query)

for file in files:
    for tree, match in treesearch.search_file(file, pattern):
        process(match)
```

```python
# Bad: Re-compiling every iteration
for file in files:
    pattern = treesearch.parse_query(query)  # Wasteful!
    for tree, match in treesearch.search_file(file, pattern):
        process(match)
```

---

## Pattern Properties

### Thread Safety

Patterns are thread-safe and can be shared across threads:

```python
from concurrent.futures import ThreadPoolExecutor

pattern = treesearch.parse_query(query)

def search_file(path):
    return list(treesearch.search_file(path, pattern))

with ThreadPoolExecutor() as executor:
    results = executor.map(search_file, file_paths)
```

### Immutability

Patterns cannot be modified after creation:

```python
pattern = treesearch.parse_query("V [upos='VERB'];")
# No way to change constraints or add variables
# Must create new pattern instead
```

### Memory

Patterns are lightweight and cheap to clone:

```python
pattern1 = treesearch.parse_query(query)
pattern2 = pattern1  # Shares underlying data
```

---

## Examples

### Simple Pattern

```python
pattern = treesearch.parse_query('V [upos="VERB"];')
print(f"Pattern has {pattern.n_vars} variable(s)")  # 1
```

### Complex Pattern

```python
query = """
    Main [upos="VERB"];
    Aux [lemma="have"];
    Comp [upos="VERB"];
    Main <-[aux]- Aux;
    Main -[xcomp]-> Comp;
"""

pattern = treesearch.parse_query(query)
print(f"Pattern has {pattern.n_vars} variables")  # 3
```

### Checking Pattern Validity

```python
try:
    pattern = treesearch.parse_query("V [invalid]")
except ValueError as e:
    print(f"Invalid query: {e}")
```

---

## Pattern Compilation

When you call `parse_query()`, treesearch:

1. **Parses** the query string into an AST
2. **Validates** variable names and constraints
3. **Optimizes** the search order (MRV heuristic)
4. **Compiles** into an efficient internal representation

The compiled pattern contains:

- Variable definitions and constraints
- Edge constraints (dependency and precedence)
- Optimized variable ordering for search
- Precomputed constraint checks

---

## Next Steps

- [Functions API](functions.md) - Using patterns with search functions
- [Query Language](../guide/query-language.md) - Writing queries
- [Searching Guide](../guide/searching.md) - Search strategies
