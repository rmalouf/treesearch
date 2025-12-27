# Query Language Reference

## Query Structure

```
MATCH {
    VariableName [constraints];
    ...
    edge_constraints;
}
```

## Node Constraints

| Constraint | Description | Example |
|------------|-------------|---------|
| `upos` | Universal POS tag | `[upos="VERB"]` |
| `xpos` | Language-specific POS | `[xpos="VBD"]` |
| `lemma` | Dictionary form | `[lemma="help"]` |
| `form` | Surface form | `[form="helping"]` |
| `deprel` | Dependency relation | `[deprel="root"]` |
| `feats.X` | Morphological feature | `[feats.Tense="Past"]` |
| `misc.X` | Miscellaneous annotation | `[misc.SpaceAfter="No"]` |

**Multiple constraints** (AND): `V [upos="VERB" & lemma="run"];`

**Empty constraint** (any word): `X [];`

**Negation**: `V [upos!="VERB"];`

## Edge Constraints

### Positive Edges

```
V -[nsubj]-> N;     # V has nsubj edge to N
V -> N;             # V has any edge to N
```

### Negative Edges

```
V !-[obj]-> N;      # V does NOT have obj edge to N
V !-> N;            # V has no edge to N
```

### Anonymous Variable

Use `_` to check existence without binding:

```
V -[obj]-> _;       # V has some object
V !-[obj]-> _;      # V has no object (intransitive)
_ !-> Root;         # Root has no incoming edge
```

## Precedence Constraints

| Operator | Meaning |
|----------|---------|
| `A < B` | A immediately precedes B |
| `A << B` | A precedes B (anywhere before) |

## Comments

```
V [upos="VERB"];  # inline comment
// full line comment
```

## Examples

### Passive Construction

```
MATCH {
    V [upos="VERB"];
    Subj [];
    V -[aux:pass]-> _;
    V -[nsubj:pass]-> Subj;
}
```

### Relative Clause

```
MATCH {
    Noun [upos="NOUN"];
    Verb [upos="VERB"];
    Noun -[acl:relcl]-> Verb;
}
```

### Intransitive Verb

```
MATCH {
    V [upos="VERB"];
    V -[nsubj]-> _;
    V !-[obj]-> _;
}
```

### Word Order

```
MATCH {
    V [upos="VERB"];
    Obj [upos="NOUN"];
    V -[obj]-> Obj;
    V < Obj;          # verb before object
}
```

## Case Sensitivity

- Variable names: case-sensitive (`V` ≠ `v`)
- Constraint values: case-sensitive (`"VERB"` ≠ `"verb"`)
- Keywords: case-insensitive (`upos` = `UPOS`)

## Common Errors

| Error | Problem | Fix |
|-------|---------|-----|
| `V [upos=VERB]` | Missing quotes | `V [upos="VERB"]` |
| `V [pos="VERB"]` | Wrong keyword | `V [upos="VERB"]` |
| `V -[obj]-> N` | N not declared | Add `N [];` first |
