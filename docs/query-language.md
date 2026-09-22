# Query Language Reference

## Query Structure

```
MATCH {
    statements
}
EXCEPT {
    statements
}
OPTIONAL {
    statements
}
```

A query has exactly one `MATCH` block, followed by any number of `EXCEPT` and `OPTIONAL` blocks in any order. A block contains three kinds of statements:

- **Node declarations**: `Name [constraints];`
- **Edge constraints**: `Parent -[label]-> Child;`
- **Precedence constraints**: `A < B;` or `A << B;`

Trailing semicolons are optional. Statements may appear in any order.

A match binds each variable to a word in the tree. **Different variables always bind different words.** The search is exhaustive: every assignment of words to variables that satisfies all constraints is returned as a separate match.

An empty block, `MATCH { }`, matches every tree once with no bindings.

## Variables

Variable names start with an ASCII letter, followed by letters, digits, or underscores (`V`, `Subj`, `head_2`). Names are case-sensitive.

Every variable must be declared, and at most once per block; using an undeclared variable or declaring one twice is an error. Use `[]` to declare a variable that matches any word.

```
MATCH { V []; O []; V -[obj]-> O; }
```

## Node Constraints

| Constraint | Field | Example |
|------------|-------|---------|
| `upos` | Universal POS tag | `[upos="VERB"]` |
| `xpos` | Language-specific POS tag | `[xpos="VBD"]` |
| `lemma` | Lemma | `[lemma="help"]` |
| `form` | Surface form | `[form="helping"]` |
| `deprel` | Relation to the word's head | `[deprel="nsubj"]` |
| `feats.Key` | Morphological feature | `[feats.Tense="Past"]` |
| `misc.Key` | MISC column attribute | `[misc.SpaceAfter="No"]` |

`X []` matches any word.

### Operators

| Syntax | Meaning |
|--------|---------|
| `key="value"` | Equal |
| `key!="value"` | Not equal |
| `c1 & c2` | Both |
| `c1 \| c2` | Either |
| `( ... )` | Grouping |

`&` binds more tightly than `|`, so `upos="PRON" | upos="AUX" & lemma="do"` means `upos="PRON" | (upos="AUX" & lemma="do")`.

```
N [upos="NOUN" | upos="PROPN"];
S [(upos="NOUN" | upos="PRON") & feats.Case="Nom"];
```

A `feats.Key` or `misc.Key` constraint fails if the word doesn't have that key, so `feats.Key!=...` succeeds for words without it: `[feats.Tense!="Past"]` matches every word except past-tense ones, including words with no `Tense` feature.

### Values

A value is either a string literal or a regular expression:

- `"value"`: exact match. String literals don't support escape sequences, so they can't contain `"` or `\`.
- `/regex/`: regex match. Write `\/` for a literal slash.

Regexes are **anchored**: `/run/` is compiled as `^run$` and matches only "run". Use `.*` for partial matches.

| Pattern | Matches |
|---------|---------|
| `/run/` | "run" only |
| `/run.*/` | "run", "runs", "running", "runway" |
| `/.*ing/` | "running", "helping" |
| `/.*el.*/` | "helped", "hello" |
| `/VERB\|AUX/` | "VERB" or "AUX" |
| `/(?i)help/` | "help", "Help", "HELP" |

Regexes use Rust [regex syntax](https://docs.rs/regex/latest/regex/#syntax). An invalid regex is a query compilation error.

```
V [upos="VERB" & form=/.*ing/];                              # -ing verb forms
M [lemma=/can|may|must|will|shall|could|might|should|would/]; # modals
V [upos="VERB" & lemma!=/be|have/];                          # verbs other than be/have
V [feats.Tense=/Past|Pres/];                                 # past or present tense
```

## Edge Constraints

`A -> B` means A is the head of B (B is a dependent of A).

| Syntax | Meaning |
|--------|---------|
| `A -> B` | B is a dependent of A |
| `A -[rel]-> B` | B is a dependent of A with deprel exactly `rel` |
| `A -/regex/-> B` | B is a dependent of A with deprel matching `regex` |
| `A !-> B` | B is not a dependent of A |
| `A !-[rel]-> B` | B is not a `rel` dependent of A |
| `A !-/regex/-> B` | B is not a dependent of A with deprel matching `regex` |

Labels match the full deprel, including any subtype: `-[nsubj]->` doesn't match `nsubj:pass`. Use a regex for families of relations:

| Edge | Matches |
|------|---------|
| `-[nsubj]->` | `nsubj` only |
| `-/nsubj.*/->` | `nsubj`, `nsubj:pass`, `nsubj:outer`, ... |
| `-/obj\|iobj/->` | `obj` or `iobj` |
| `-/.*mod/->` | `amod`, `advmod`, `nummod`, ... |

Negative edges only rule out one relationship between A and B. Both variables still have to be bound to words. For example, `V !-[obj]-> N` matches every pair of distinct words where N isn't an `obj` dependent of V. To say that a word has no dependent of some kind, use `_` (see below) or an `EXCEPT` block.

### Anonymous Variable `_`

`_` can be used on either side of an edge to check whether some matching word exists, without binding it:

| Constraint | Meaning |
|------------|---------|
| `V -[obj]-> _` | V has an `obj` dependent |
| `V !-[obj]-> _` | V has no `obj` dependent |
| `V -> _` | V has at least one dependent |
| `V !-> _` | V has no dependents |
| `_ -[nsubj]-> N` | N has a head and its deprel is `nsubj` |
| `_ -> N` | N has a head (N isn't the root) |
| `_ !-> N` | N is the root |

Each `_` is independent of the other variables, and the AllDifferent rule doesn't apply to it. For example, `V -[obj]-> O; V -[obj]-> _;` matches a verb with one object, because `_` can be the same word as `O`. `_` can't be used in precedence constraints.

## Precedence Constraints

| Syntax | Meaning |
|--------|---------|
| `A < B` | A immediately precedes B |
| `A << B` | A precedes B (anywhere earlier in the sentence) |

Precedence uses the order of syntactic words. Multiword token lines (`1-2`) are ignored.

## EXCEPT Blocks

An `EXCEPT` block rejects a match if the block can be satisfied given the match's bindings. With more than one `EXCEPT` block, a match is rejected if any of them can be satisfied.

```
MATCH {
    V [upos="VERB"];
}
EXCEPT {
    V -[advmod]-> M;
    M [upos="ADV"];
}
```

This finds verbs that have no adverb modifier.

Variables that are new in the `EXCEPT` block are existential: the match is rejected if there is any binding for them that satisfies the block. Because new variables never bind a word already bound by `MATCH`, this query finds verbs with exactly one subject:

```
MATCH  { V [upos="VERB"]; S []; V -[nsubj]-> S; }
EXCEPT { X []; V -[nsubj]-> X; }
```

## OPTIONAL Blocks

An `OPTIONAL` block extends a match with additional bindings when it can. If it can't be satisfied, the match is kept and the block's variables are left unbound.

```
MATCH {
    V [upos="VERB"];
}
OPTIONAL {
    O [];
    V -[obj]-> O;
}
```

This finds all verbs and binds their objects to `O` when there are objects. In Python, check with `"O" in match` or `match.get("O")`.

If an `OPTIONAL` block can be satisfied in more than one way, each way produces a separate match. Each `OPTIONAL` block is matched independently against the `MATCH` bindings, and the results are combined as a cross product:

```
MATCH { V [upos="VERB"]; }
OPTIONAL { S []; V -[nsubj]-> S; }
OPTIONAL { O []; V -[obj]-> O; }
```

If V has 2 subjects and 3 objects, this gives 6 matches (2 × 3). If V has 2 subjects and no objects, it gives 2 matches, with `O` unbound.

`EXCEPT` blocks are checked against the `MATCH` bindings before `OPTIONAL` blocks are applied.

## Scoping

- `EXCEPT` and `OPTIONAL` blocks can use `MATCH` variables in edge and precedence constraints without declaring them, but can't redeclare them: `EXCEPT { V [lemma="be"]; }` is an error. Put node constraints on `MATCH` variables in `MATCH` (e.g., `V [upos="VERB" & lemma!="be"]`).
- A new variable in one `EXCEPT` or `OPTIONAL` block can't appear in any other `EXCEPT` or `OPTIONAL` block. Using the same name twice is an error.
- New variables in `EXCEPT` and `OPTIONAL` blocks never bind a word that is already bound by `MATCH`. Variables in different `OPTIONAL` blocks are matched independently and may bind the same word.

## Lexical Details

- Keywords are case-sensitive: `MATCH`, `EXCEPT`, `OPTIONAL`, and the constraint names `upos`, `xpos`, `lemma`, `form`, `deprel`, `feats`, `misc`.
- Values are case-sensitive: `"VERB"` ≠ `"verb"`. Use `(?i)` in a regex to ignore case.
- Comments start with `#` or `//` and run to the end of the line.
- Whitespace and newlines are ignored.

## Examples

### Passive

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

### Verb with a Subject and No Object

```
MATCH {
    V [upos="VERB"];
    V -[nsubj]-> _;
    V !-[obj]-> _;
}
```

### Verb Before Its Object

```
MATCH {
    V [upos="VERB"];
    Obj [upos="NOUN"];
    V -[obj]-> Obj;
    V << Obj;
}
```

### Progressive

```
MATCH {
    V [upos="VERB" & form=/.*ing/];
    Aux [lemma="be"];
    V -[aux]-> Aux;
}
```

### Modal + Verb

```
MATCH {
    V [upos="VERB"];
    Modal [lemma=/can|may|must|will|shall|could|might|should|would/];
    V -[aux]-> Modal;
}
```

### Any Subject Relation

```
MATCH {
    V [upos="VERB"];
    S [upos="NOUN" | upos="PROPN"];
    V -/nsubj.*/-> S;
}
```

## Common Errors

| Query | Problem | Fix |
|-------|---------|-----|
| `V [upos=VERB]` | Value not quoted | `V [upos="VERB"]` |
| `V [pos="VERB"]` | Unknown constraint name | `V [upos="VERB"]` |
| `V [upos="VERB", lemma="be"]` | Constraints separated by a comma | `V [upos="VERB" & lemma="be"]` |
| `V [UPOS="VERB"]` | Constraint names are lowercase | `V [upos="VERB"]` |
| `V []; V [upos="VERB"];` | Duplicate declaration | `V [upos="VERB"];` |
| `V []; V -[obj]-> O;` | `O` not declared | `V []; O []; V -[obj]-> O;` |
| `MATCH { V []; } EXCEPT { V [lemma="be"]; }` | `MATCH` variable redeclared | `MATCH { V [lemma!="be"]; }` |
| `V -[nsubj]-> S` on `nsubj:pass` | Labels match exactly | `V -/nsubj.*/-> S` |
