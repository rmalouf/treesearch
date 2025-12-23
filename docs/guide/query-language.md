# Query Language Reference

Complete reference for the treesearch query language.

## Overview

Queries consist of two parts:

1. **Variable declarations**: Define pattern elements and their constraints
2. **Edge constraints**: Specify relationships between variables

## Variable Declarations

### Basic Syntax

```
MATCH {
VariableName [constraint];
}
```

Example:

```
MATCH {
V [upos="VERB"];
}
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

**Negation:** Use `!=` to exclude values:

```
MATCH {
# Not a verb
NotVerb [upos!="VERB"];

# Not the lemma "be"
NotBe [lemma!="be"];
}
```

### Multiple Constraints

Combine constraints with commas (AND logic):

```
MATCH {
PastVerb [upos="VERB", xpos="VBD"];
HelpVerb [lemma="help", upos="VERB"];
}
```

### Empty Constraints

Match any word:

```
MATCH {
AnyWord [];
}
```

This is useful when you care about structure but not specific properties.

## Edge Constraints

### Dependency Edges

Specify parent-child relationships:

```
MATCH {
Parent -[deprel]-> Child;
}
```

The arrow direction indicates the dependency:

- `->`: Points from parent to child

### Edge Examples

```
MATCH {
# Verb has object child
V -[obj]-> N;

# Any child relationship
V -> Child;
}
```

### Unlabeled Edges

Omit the relation to match any dependency:

```
MATCH {
# V has any child
V -> X;
}
```

### Negative Edge Constraints

Negate edges to require their **absence**:

```
MATCH {
# V does NOT have any edge to W
V !-> W;

# V does NOT have obj edge to W (but may have other edges)
V !-[obj]-> W;
}
```

**Semantics:**
- `X !-> Y` - X has no edge of any type to Y
- `X !-[label]-> Y` - X has no edge with the specific label to Y (but may have edges with other labels)

**Examples:**

```
MATCH {
# Find verbs that don't have objects
V [upos="VERB"];
Obj [];
V !-[obj]-> Obj;

# Find words not connected to a specific word
Help [lemma="help"];
To [lemma="to"];
Help !-> To;

# Combine positive and negative constraints
# V has xcomp to Y but NOT obj to W
V [];
Y [];
W [];
V -[xcomp]-> Y;
V !-[obj]-> W;
}
```

### Anonymous Variables with Negation

Use anonymous variable `_` with negation for common patterns:

```
MATCH {
# Find root words (no incoming edges)
Root [];
_ !-> Root;

# Find words that are not anyone's object
NotObj [];
_ !-[obj]-> NotObj;
}
```

## Precedence Constraints

Specify linear word order:

| Operator | Meaning | Example |
|----------|---------|---------|
| `<` | Directly precedes | `A < B;` |
| `<<` | Precedes (transitively) | `A << B;` |

### Precedence Examples

```
MATCH {
# "to" directly precedes verb
To < V;

# Help comes before V (anywhere)
Help << V;
}
```

## Complete Examples

### Subject-Verb-Object

```
MATCH {
V [upos="VERB"];
Subj [upos="NOUN"];
Obj [upos="NOUN"];
V -[nsubj]-> Subj;
V -[obj]-> Obj;
}
```

### help-to-infinitive Construction

```
MATCH {
Help [lemma="help"];
To [lemma="to"];
V [upos="VERB"];
Help -[xcomp]-> To;
To -[mark]-> V;
Help << To;
To < V;
}
```

### Passive Voice

```
MATCH {
Verb [upos="VERB"];
Aux [lemma="be"];
Subj [];
Verb -[aux:pass]-> Aux;
Verb -[nsubj:pass]-> Subj;
}
```

### Relative Clause

```
MATCH {
Noun [upos="NOUN"];
RelPron [upos="PRON"];
Verb [upos="VERB"];
Noun -[acl:relcl]-> Verb;
Verb -[nsubj]-> RelPron;
}
```

### Coordination

TODO: Fix this

```
MATCH {
First [];
Second [];
Conj [lemma="and"];
First -[conj]-> Second;
Second <-[cc]- Conj;
}
```

## Comments

Use `//` or `#` for comments:

```
MATCH {
// This is a comment
V [upos="VERB"];  # This is also a comment
N [upos="NOUN"];
V -[obj]-> N;
}
```

## Case Sensitivity

- Variable names are case-sensitive: `V` and `v` are different
- Constraint values are case-sensitive: `"VERB"` ≠ `"verb"`
- Keywords (like `upos`) are case-insensitive

## Best Practices

### Naming Variables

Use descriptive names:

```
MATCH {
# Good
Main [upos="VERB"];
Auxiliary [lemma="have"];

# Less clear
V1 [upos="VERB"];
V2 [lemma="have"];
}
```

### Start Simple

Build complex queries incrementally:

```
MATCH {
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
}
```

### Test Queries

Test on small data first:

```python
# Search one file first
for tree, match in treesearch.search("sample.conllu", pattern):
    print(match)

# Then scale to full corpus
for tree, match in treesearch.search("corpus/*.conllu", pattern):
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
