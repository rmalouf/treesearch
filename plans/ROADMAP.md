# Treesearch Development Roadmap

**Last Updated**: December 2025

This document outlines the planned features and enhancements for Treesearch.

---

## Priority 1: PyPI Publishing

**Goal**: Enable `pip install treesearch` for users.

**Status**: ⏳ Not started

### Requirements
- [ ] Version number strategy (start with 0.1.0?)
- [ ] Package metadata in `pyproject.toml` (description, keywords, classifiers)
- [ ] Prepare README.md for PyPI landing page
- [ ] Create release workflow (manual or GitHub Actions)
- [ ] Test installation from TestPyPI first
- [ ] Documentation URL (point to GitHub docs/ or Read the Docs?)

### Implementation Notes
- Maturin already configured in `pyproject.toml`
- Need to verify all metadata is complete
- Consider: Semantic versioning strategy going forward
- Platform wheels: maturin can build for multiple platforms (Linux, macOS, Windows)

### Commands
```bash
# Build wheel
maturin build --release

# Publish to TestPyPI
maturin publish --repository testpypi

# Publish to PyPI
maturin publish
```

### Open Questions
- What version to start with? (Suggest: 0.1.0 for initial release)
- Need PyPI account credentials
- Should we automate releases with GitHub Actions?

---

## Priority 2: Regular Expressions in Node Constraints

**Goal**: Support regex patterns in node constraints for flexible matching.

**Status**: ⏳ Not started

### Syntax Design

**Proposed syntax**:
```
MATCH {
    # Exact match (current)
    V [lemma="run"];

    # Regex match (new)
    V [lemma=~/run|walk|jump/];
    V [form=~/.*ing$/];  # Words ending in -ing
    V [form=~/^[A-Z]/];   # Capitalized words
}
```

### Implementation Plan

1. **Query parser** (`src/query.rs`):
   - Add regex literal syntax to grammar: `~/.../`
   - Parse to new `ConstraintValue` variant: `Regex(String)`

2. **Pattern AST** (`src/pattern.rs`):
   - Extend `NodeConstraint` enum to support regex
   - Store compiled regex or pattern string

3. **CSP Solver** (`src/searcher.rs`):
   - When checking constraints, test against regex if applicable
   - Use `regex` crate for matching

4. **Python bindings**: Transparent - regex patterns work in query strings

### Dependencies
- Add `regex = "1.11"` to `Cargo.toml`

### Testing
- Match words by pattern (e.g., `-ing` endings)
- Case-insensitive matching with `(?i)` flag
- Anchor patterns (`^`, `$`)
- Character classes (`[A-Z]`, `\d`)

### Performance Considerations
- Compile regexes once during pattern compilation, not per-match
- Regex matching is slower than exact string comparison
- Consider caching compiled regexes in Pattern struct

---

## Priority 3: Disjunctions in Node Constraints

**Goal**: Allow OR logic within node constraints.

**Status**: ⏳ Not started

### Syntax Design

**Option A: Pipe syntax** (recommended):
```
MATCH {
    # Single field disjunction
    N [upos="NOUN" | upos="PROPN"];

    # Multiple values for same field
    V [lemma="run" | lemma="walk" | lemma="jump"];

    # Can combine with regex
    V [lemma="run" | lemma=~/walk.*/];
}
```

**Option B: Multiple constraint blocks**:
```
MATCH {
    N [upos="NOUN"] | [upos="PROPN"];
}
```

**Recommendation**: Option A - clearer for same-field disjunctions, most common use case.

### Implementation Plan

1. **Query parser** (`src/query.rs`):
   - Extend grammar to allow `|` in constraint lists
   - Parse `[upos="NOUN" | upos="PROPN"]` into disjunction AST

2. **Pattern AST** (`src/pattern.rs`):
   - Options:
     - **A**: Group constraints by field, store as `Vec<Value>` per field
     - **B**: Add `Or` variant to `NodeConstraint` enum
   - Recommendation: Option A if simple same-field OR, Option B for general case

3. **CSP Solver** (`src/searcher.rs`):
   - Domain filtering: word matches if it satisfies ANY constraint in disjunction
   - For `[upos="NOUN" | upos="PROPN"]`, accept if upos is either value

4. **Python bindings**: Transparent

### Interaction with Regex
Should support mixing:
```
V [lemma="run" | lemma=~/walk.*/ | lemma="jump"]
```

### Testing
- Simple OR: `[upos="NOUN" | upos="PROPN"]`
- Multiple values: `[deprel="nsubj" | deprel="obj" | deprel="iobj"]`
- Cross-field OR (if supported): `[upos="NOUN" | lemma="thing"]`
- Empty disjunction behavior

---

## Priority 4: Wildcards in Dependency Constraints

**Goal**: Allow pattern matching in edge labels (deprels).

**Status**: ⏳ Not started

### Syntax Design

**Proposed syntax**:
```
MATCH {
    V [upos="VERB"];
    N [upos="NOUN"];

    # Any dependency relation (new)
    V -[*]-> N;

    # Prefix wildcard (new)
    V -[nsubj:*]-> N;  # Matches nsubj, nsubj:pass, nsubj:outer, etc.

    # Suffix wildcard (new)
    V -[*:pass]-> N;   # Matches nsubj:pass, aux:pass, etc.

    # Regex in deprel (if combining with Priority 2)
    V -[~/.*subj.*/]-> N;
}
```

### Implementation Plan

1. **Query parser** (`src/query.rs`):
   - Allow `*` in edge labels
   - Parse wildcards into pattern representation

2. **Pattern AST** (`src/pattern.rs`):
   - Extend `EdgeConstraint` to support:
     - `AnyLabel` - matches any deprel
     - `PrefixPattern(String)` - matches prefix
     - `SuffixPattern(String)` - matches suffix
     - `RegexPattern(Regex)` - full regex (if Priority 2 done first)

3. **CSP Solver** (`src/searcher.rs`):
   - When checking edge constraints, test against pattern
   - For `nsubj:*`, check if actual deprel starts with `nsubj:`

4. **Python bindings**: Transparent

### Common Use Cases
- `V -[*:pass]-> N` - Any passive relation
- `V -[nsubj:*]-> N` - Any kind of subject (nsubj, nsubj:pass, nsubj:outer)
- `V -[*]-> N` - Any relation at all

### Testing
- Match all edges: `X -[*]-> Y`
- Prefix matching: `X -[nsubj:*]-> Y`
- Suffix matching: `X -[*:pass]-> Y`
- Combining with negation: `X !-[aux:*]-> Y` (no auxiliary relations)

---

## Priority 5: Export to CoNLL-U Subcorpus

**Goal**: Save matching sentences/trees back to CoNLL-U format.

**Status**: ⏳ Not started

### Use Cases
- Extract subcorpus of sentences matching specific patterns
- Create training data for ML models
- Share examples with colleagues
- Filter large corpora

### API Design

**Python API**:
```python
import treesearch as ts

pattern = ts.parse_query('MATCH { V [upos="VERB"]; }')

# Option A: Function that writes matches to file
ts.export_matches("corpus.conllu", pattern, output="verbs.conllu")

# Option B: Export from treebank
treebank = ts.Treebank.from_file("corpus.conllu")
treebank.export_matches(pattern, output="verbs.conllu")

# Option C: Collect trees and export
trees = [tree for tree, match in treebank.matches(pattern)]
ts.export_trees(trees, output="verbs.conllu")
```

**Recommendation**: Implement all three - different use cases need different workflows.

### Implementation Plan

1. **Rust Core** (`src/conllu.rs`):
   - Add `Tree::to_conllu() -> String` method
   - Format tree back to CoNLL-U (reverse of parsing)
   - Preserve metadata and sentence_text

2. **Python bindings** (`src/python.rs`):
   - Add export functions to module
   - Accept file path or file-like object
   - Option to preserve or deduplicate trees

3. **Metadata preservation**:
   - Keep all `# key = value` metadata
   - Keep `# text = ...` line
   - Maintain original formatting where possible

### Open Questions
- Should we deduplicate trees? (If pattern has multiple matches per sentence, include once or multiple times?)
- Preserve original line numbers in comments?
- Support for writing to gzip files?
- Include match information in comments? (e.g., `# match_count = 3`)

### Testing
- Export single tree
- Export multiple trees
- Verify round-trip: parse → export → parse → identical
- Preserve metadata
- Handle gzip output

---

## Priority 6: Add DEPS to Query Language

**Goal**: Support querying enhanced dependencies (DEPS) in the query language.

**Status**: ⏳ Not started (MISC field access already implemented)

### Background
CoNLL-U format includes two additional fields:
- **DEPS**: Enhanced dependencies (graph structure beyond tree) - NOT YET queryable
- **MISC**: Miscellaneous annotations (SpaceAfter, etc.) - ✅ ALREADY ACCESSIBLE via Word.misc property

Currently treesearch parses DEPS but doesn't expose it in queries. MISC is already fully accessible via the Python API.

### Syntax Design

**DEPS (Enhanced Dependencies)**:
```
MATCH {
    V [upos="VERB"];
    N [upos="NOUN"];

    # Enhanced dependency relation
    V -[deps:obl]-> N;  # or different syntax?
}
```

**MISC (Miscellaneous Annotations)** - ✅ Already accessible via Word.misc property:
```python
# MISC is already accessible in Python
for tree, match in treebank.search(pattern):
    word = tree.word(match["W"])
    if word.misc:  # Access MISC field
        print(f"MISC: {word.misc}")
```

For future query language support:
```
MATCH {
    # Check MISC field key-value pairs (NOT YET IMPLEMENTED)
    W [misc.SpaceAfter="No"];
    W [misc.Gloss="running"];
}
```

### Implementation Challenges

**DEPS complexity**:
- DEPS can have multiple head-deprel pairs per word (graph, not tree)
- Format: `4:nsubj|6:conj` (colon-separated, pipe-separated)
- Need to decide: new constraint type or extend edge constraints?

**MISC complexity**:
- MISC is key-value pairs: `SpaceAfter=No|Gloss=example`
- Need structured access, not just string matching
- Common keys: SpaceAfter, Gloss, Translit

### Implementation Plan

1. **Data structures** (`src/tree.rs`):
   - Already parsed: `deps: Option<String>` and `misc: Option<String>`
   - Consider parsing into structured form:
     - `deps: Vec<(usize, String)>` - list of (head, deprel) pairs
     - `misc: HashMap<String, String>` - key-value map

2. **Query parser** (`src/query.rs`):
   - Add syntax for DEPS constraints
   - Add syntax for MISC constraints with key access

3. **Pattern AST** (`src/pattern.rs`):
   - Add `DepsConstraint` for enhanced dependencies
   - Add `MiscConstraint` for misc annotations

4. **CSP Solver** (`src/searcher.rs`):
   - Check DEPS constraints during edge constraint validation
   - Check MISC constraints during node constraint validation

5. **Python bindings**: Expose deps in Word class (misc already exposed)

### Open Questions
- **DEPS syntax**: Should we use `deps:` prefix or new operator?
- **DEPS semantics**: Match any enhanced dep or all of them?
- **MISC parsing**: Parse at load time or query time?
- **Priority**: Is DEPS/MISC commonly used in target corpora?

### Testing
- Query DEPS relations
- Query MISC fields (basic access already tested)
- Combine with regular constraints
- Handle missing DEPS/MISC (most words don't have them)

---

## Implementation Priority & Dependencies

### Suggested Order

**Phase 1: Infrastructure** (Independent, can do in any order)
1. **PyPI Publishing** - Makes project accessible to users
2. **Export to CoNLL-U** - Valuable standalone feature

**Phase 2: Query Language Extensions** (Build on each other)
3. **Regular Expressions** - Foundation for pattern matching
4. **Wildcards in Dependencies** - Uses similar pattern matching
5. **Disjunctions** - Logical extension of constraints
6. **DEPS and MISC** - Additional fields (independent of above)

### Recommendation
Start with **PyPI Publishing** to get the project out to users with current functionality, then focus on query language extensions based on user feedback.

---

## Future Considerations (Beyond Current Roadmap)

- **Ancestor/Descendant relations**: `X <<- Y` (X is ancestor of Y)
- **Sibling relations**: `X ~ Y` (X and Y share parent)
- **Distance constraints**: `X <-[2..5]- Y` (path length 2-5)
- **Optional matches**: `X -[nsubj]->? Y` (Y is optional)
- **Capture groups**: Store intermediate matches
- **Query optimization**: Reorder constraints for performance
- **Indexing**: Pre-index corpus for faster repeated queries

---

## Version Planning

Suggested version numbers for releases:

- **v0.1.0**: Current state + PyPI publishing
- **v0.2.0**: Add regex + wildcards + disjunctions
- **v0.3.0**: Add DEPS/MISC + export functionality
- **v1.0.0**: Stable API, comprehensive documentation, production-ready

---

## Success Metrics

For each feature:
- [ ] Tests pass (Rust + Python)
- [ ] Documentation updated (README.md, API.md, docs/)
- [ ] Examples added to `examples/` directory
- [ ] Benchmarks show acceptable performance
- [ ] User feedback incorporated (after PyPI release)

---

## Notes

- Keep backward compatibility within minor versions (0.x.y)
- All query language changes should be additive (no breaking changes)
- Prioritize features based on user feedback after PyPI release
