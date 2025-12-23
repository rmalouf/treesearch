# Pattern class

Compiled query pattern for tree matching.

## Overview

Pattern objects represent compiled queries created by `compile_query()`. Patterns are opaque, reusable, and thread-safe objects used with search functions to find matches in dependency trees.

Patterns should be compiled once and reused across multiple searches for best performance.

## Properties

---

## Creating patterns

Patterns are created using the `compile_query()` function:

```python
pattern = treesearch.compile_query(query_string)
```

**Example:**

```python
# Simple pattern
pattern = treesearch.compile_query('MATCH { V [upos="VERB"]; }')

# Complex pattern with multiple constraints
pattern = treesearch.compile_query("""
    MATCH {
        V [upos="VERB"];
        Subj [upos="NOUN"];
        Obj [upos="NOUN"];
        V <-[nsubj]- Subj;
        V -[obj]-> Obj;
    }
""")
```


## Examples

### Passive constructions

```python
passive = treesearch.compile_query("""
    MATCH {
        V [upos="VERB"];
        Aux [lemma="be"];
        V <-[aux:pass]- Aux;
    }
""")

for tree, match in treesearch.search("corpus.conllu", passive):
    verb = tree[match["V"]]
    print(f"Passive: {tree.sentence_text}")
```

### Control verbs

```python
help_infinitive = treesearch.compile_query("""
    MATCH {
        Main [lemma="help"];
        Inf [upos="VERB"];
        Main -[xcomp]-> Inf;
    }
""")

for tree, match in treesearch.search("data/*.conllu", help_infinitive):
    main = tree[match["Main"]]
    inf = tree[match["Inf"]]
    print(f"{main.form} ... {inf.form}: {tree.sentence_text}")
```

### Relative clauses

```python
relative = treesearch.compile_query("""
    MATCH {
        Head [upos="NOUN"];
        Rel [upos="PRON"];
        V [upos="VERB"];
        Head -[acl:relcl]-> V;
        V -[nsubj]-> Rel;
    }
""")

tb = treesearch.Treebank.from_files("data/*.conllu")
for tree, match in tb.search(relative):
    head = tree[match["Head"]]
    rel = tree[match["Rel"]]
    verb = tree[match["V"]]
    print(f"{head.form} {rel.form} {verb.form}")
```

### Checking pattern validity

```python
try:
    pattern = treesearch.compile_query("MATCH { V [invalid syntax] }")
except ValueError as e:
    print(f"Invalid query: {e}")
```

---

## See also

- [parse_query()](functions.md#parse_query) - Creating patterns
- [Query language](../guide/query-language.md) - Query syntax reference
- [search()](functions.md#search) - Using patterns with single trees
- [get_matches()](functions.md#get_matches) - Using patterns with files
- [Treebank](treebank.md) - Object-oriented pattern search
