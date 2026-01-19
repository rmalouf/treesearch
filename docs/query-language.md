# Query Language Reference

## Query Structure

```
MATCH {
    VariableName [constraints];
    ...
    edge_constraints;
}
EXCEPT {
    # Reject if this pattern matches
}
OPTIONAL {
    # Extend match with these bindings if possible
}
```

A query consists of a required MATCH block followed by zero or more EXCEPT and OPTIONAL blocks.

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

### Constraint Values

Constraint values can be:
- **Literal strings** (in quotes): `lemma="run"` - exact match
- **Regular expressions** (in slashes): `lemma=/run.*/` - pattern match

### Regular Expressions

Regex patterns are **automatically anchored** for full-string matching (consistent with literal behavior):

| Pattern | Matches | Description |
|---------|---------|-------------|
| `/run/` | "run" only | Exact match (like `"run"`) |
| `/run.*/` | "run", "runs", "running" | Starts with "run" |
| `/.*ing/` | "running", "helping" | Ends with "ing" |
| `/.*el.*/` | "helped", "hello" | Contains "el" |
| `/VERB\|AUX/` | "VERB" or "AUX" | Alternation |

**Examples:**

```
# Find progressive verbs (ending in -ing)
V [upos="VERB" & form=/.*ing/];

# Find modal verbs
M [lemma=/(can|may|must|will|shall|could|might|should|would)/];

# Find verbs NOT starting with "be"
V [upos="VERB" & lemma!=/be.*/];

# Find past or present tense
V [feats.Tense=/Past|Pres/];
```

**Note:** Patterns use Rust [regex syntax](https://docs.rs/regex/latest/regex/#syntax). Invalid patterns cause a compile error.

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

### Progressive Construction (with Regex)

```
MATCH {
    Aux [lemma=/be.*/];     # be, is, was, were, etc.
    V [form=/.*ing/];       # any word ending in -ing
    Aux -[aux]-> V;
}
```

### Modal Verb Construction (with Regex)

```
MATCH {
    Modal [lemma=/(can|may|must|will|shall|could|might|should|would)/];
    Verb [upos="VERB"];
    Modal -> Verb;
}
```

## EXCEPT Blocks

Reject matches where a condition is true. Multiple EXCEPT blocks use ANY semantics (reject if any matches).

```
MATCH {
    V [upos="VERB"];
}
EXCEPT {
    M [upos="ADV"];
    V -[advmod]-> M;
}
```

This finds verbs that do NOT have an adverb modifier.

EXCEPT blocks can reference variables from MATCH:

```
MATCH {
    V [upos="VERB"];
    S [upos="NOUN"];
    V -[nsubj]-> S;
}
EXCEPT {
    Aux [upos="AUX"];
    Aux -> V;
}
```

This finds verb-subject pairs where the verb is not governed by an auxiliary.

## OPTIONAL Blocks

Extend matches with additional variables if possible. If the OPTIONAL pattern doesn't match, the base match is kept with the optional variables absent from bindings.

```
MATCH {
    V [upos="VERB"];
}
OPTIONAL {
    O [upos="NOUN"];
    V -[obj]-> O;
}
```

This finds all verbs, and if they have an object, binds it to `O`. Check for optional bindings with `match.get("O")`.

**Multiple OPTIONAL blocks**: Create cross-product of all extensions.

```
MATCH { V [upos="VERB"]; }
OPTIONAL { S []; V -[nsubj]-> S; }
OPTIONAL { O []; V -[obj]-> O; }
```

If V has 2 subjects and 3 objects, this produces 6 matches (2 × 3).

**Variable scoping**: EXCEPT/OPTIONAL blocks can reference MATCH variables but cannot reference variables from other EXCEPT/OPTIONAL blocks. New variable names must be unique across all extension blocks.

## Case Sensitivity

- Variable names: case-sensitive (`V` ≠ `v`)
- Constraint values: case-sensitive (`"VERB"` ≠ `"verb"`)
- Keywords: case-sensitive (`upos` only, not `UPOS`)

## Common Errors

| Error | Problem | Fix |
|-------|---------|-----|
| `V [upos=VERB]` | Missing quotes | `V [upos="VERB"]` |
| `V [pos="VERB"]` | Wrong keyword | `V [upos="VERB"]` |
| `V -[obj]-> N` | N not declared | Add `N [];` first |
