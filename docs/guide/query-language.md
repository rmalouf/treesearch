# Query Language Reference

Complete reference for the treesearch query language.

## Overview

Queries consist of two parts:

1. **Variable declarations**: Define pattern elements and their constraints
2. **Edge constraints**: Specify relationships between variables

## Variable Declarations

### Basic Syntax

```
VariableName [constraint];
```

Example:

```
V [upos="VERB"];
```

### Node Constraints

Constrain variables by word properties:

| Constraint | Description | Example |
|------------|-------------|---------|
| `upos` | Universal POS tag | `[upos="VERB"]` |
| `xpos` | Language-specific POS | `[xpos="VBD"]` |
| `lemma` | Dictionary form | `[lemma="help"]` |
| `form` | Surface form | `[form="helping"]` |
| `deprel` | Dependency relation | `[deprel="root"]` |

### Multiple Constraints

Combine constraints with commas (AND logic):

```
PastVerb [upos="VERB", xpos="VBD"];
HelpVerb [lemma="help", upos="VERB"];
```

### Empty Constraints

Match any word:

```
AnyWord [];
```

This is useful when you care about structure but not specific properties.

## Edge Constraints

### Dependency Edges

Specify parent-child relationships:

```
Parent -[deprel]-> Child;
```

The arrow direction indicates the dependency:

- `->`: Points from parent to child
- `<-`: Points from child to parent (same as `->` but reversed)

### Edge Examples

```
# Verb has object child
V -[obj]-> N;

# Noun has verb parent (equivalent to above)
N <-[obj]- V;

# Any child relationship
V -> Child;

# Any parent relationship
Child <- Parent;
```

### Unlabeled Edges

Omit the relation to match any dependency:

```
# V has any child
V -> X;

# Y has any parent
Y <- Z;
```

## Precedence Constraints

Specify linear word order:

| Operator | Meaning | Example |
|----------|---------|---------|
| `<` | Directly precedes | `A < B;` |
| `<<` | Precedes (transitively) | `A << B;` |

### Precedence Examples

```
# "to" directly precedes verb
To < V;

# Help comes before V (anywhere)
Help << V;
```

## Complete Examples

### Subject-Verb-Object

```
V [upos="VERB"];
Subj [upos="NOUN"];
Obj [upos="NOUN"];
V -[nsubj]-> Subj;
V -[obj]-> Obj;
```

### help-to-infinitive Construction

```
Help [lemma="help"];
To [lemma="to"];
V [upos="VERB"];
Help -[xcomp]-> To;
To -[mark]-> V;
Help << To;
To < V;
```

### Passive Voice

```
Verb [upos="VERB"];
Aux [lemma="be"];
Subj [];
Verb <-[aux:pass]- Aux;
Verb -[nsubj:pass]-> Subj;
```

### Relative Clause

```
Noun [upos="NOUN"];
RelPron [upos="PRON"];
Verb [upos="VERB"];
Noun -[acl:relcl]-> Verb;
Verb -[nsubj]-> RelPron;
```

### Coordination

```
First [];
Second [];
Conj [lemma="and"];
First -[conj]-> Second;
Second <-[cc]- Conj;
```

## Comments

Use `//` or `#` for comments:

```
// This is a comment
V [upos="VERB"];  # This is also a comment
N [upos="NOUN"];
V -[obj]-> N;
```

## Case Sensitivity

- Variable names are case-sensitive: `V` and `v` are different
- Constraint values are case-sensitive: `"VERB"` ≠ `"verb"`
- Keywords (like `upos`) are case-insensitive

## Best Practices

### Naming Variables

Use descriptive names:

```
# Good
Main [upos="VERB"];
Auxiliary [lemma="have"];

# Less clear
V1 [upos="VERB"];
V2 [lemma="have"];
```

### Start Simple

Build complex queries incrementally:

```
# Step 1: Find verbs
V [upos="VERB"];

# Step 2: Add object constraint
V [upos="VERB"];
Obj [upos="NOUN"];
V -[obj]-> Obj;

# Step 3: Add specificity
V [upos="VERB", lemma="eat"];
Obj [upos="NOUN"];
V -[obj]-> Obj;
```

### Test Queries

Test on small data first:

```python
# Search one file first
for tree, match in treesearch.search_file("sample.conllu", pattern):
    print(match)

# Then scale to full corpus
for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
    print(match)
```

## Error Messages

Common errors and solutions:

### "Expected constraint"

```
V [upos=VERB];  # Missing quotes
```

Fix: Add quotes around values:

```
V [upos="VERB"];
```

### "Unknown constraint"

```
V [pos="VERB"];  # Should be 'upos'
```

Fix: Use correct constraint names (see table above).

### "Undefined variable"

```
V -[obj]-> N;  # N not declared
```

Fix: Declare all variables before using them:

```
V [upos="VERB"];
N [upos="NOUN"];
V -[obj]-> N;
```

## Next Steps

- [Searching Guide](searching.md) - How to use compiled patterns
- [Working with Results](results.md) - Navigate trees and extract data
- [Finding Constructions](../workflows/constructions.md) - Real-world examples
