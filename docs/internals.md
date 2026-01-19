# Internals

How treesearch works under the hood.

## Architecture Overview

Treesearch treats pattern matching as a **Constraint Satisfaction Problem (CSP)**:

1. **Query Parser** (`src/query.rs`) - Pest-based parser converts query strings to Pattern AST
2. **CSP Solver** (`src/searcher.rs`) - DFS with forward checking and MRV heuristic
3. **Tree Storage** (`src/tree.rs`) - String interning (lasso + FxHash) for memory efficiency
4. **CoNLL-U Parser** (`src/conllu.rs`) - Streaming parser with automatic gzip detection
5. **Parallelization** (`src/iterators.rs`) - File-level parallelism via rayon + channels
6. **Python Bindings** (`src/python.rs`) - PyO3 with zero-copy Arc sharing

## Search Algorithm

```
1. Order variables by constraint degree (MRV heuristic)
2. For each variable:
   a. Get candidates satisfying node constraints
   b. Check arc consistency (edge constraints)
   c. Recursively assign remaining variables
   d. Backtrack if no valid assignment
3. Yield all complete assignments
```

Forward checking typically reduces search space by 90%+.

## Constraint Types

- **Node constraints** (lemma, upos, form, deprel):
  - Literals: Direct string pool comparison (O(1) via interned strings)
  - Regex: Pre-compiled patterns matched against resolved UTF-8 strings
- **Feature constraints** (feats.X, misc.X): Iterate word features for key-value match (supports both literals and regex)
- **Edge constraints**: Check parent/children relationships
- **Anonymous variables** (`_`): Create HasIncomingEdge/HasOutgoingEdge constraints without bindings

### Regex Implementation

Regex patterns are compiled once during query parsing with automatic `^...$` anchoring for full-string matching:
- `/run/` → compiled as `^run$` (exact match)
- `/run.*/` → compiled as `^run.*$` (starts with "run")
- Pattern compilation errors are caught during query parsing with clear error messages
- Compiled regex stored in `ConstraintValue::Regex(pattern, compiled_regex)` for reuse

## Parallelization

```
Files → Chunks (8) → rayon::par_iter → Process → Bounded channel (8) → Iterator
```

Results streamed through channels with backpressure. Use `ordered=False` for maximum throughput.

## Design Decisions

**Why CSP?** Natural representation of structural constraints, well-studied optimizations, exhaustive search guarantee.

**Why exhaustive search?** Corpus linguistics requires *all* matches; researchers filter in post-processing.

**Why Rust?** 10-100x faster than pure Python, memory-efficient interning, safe parallelism.

## Source Files

| File | Purpose |
|------|---------|
| `src/query.rs` | Query parser (Pest grammar), regex compilation |
| `src/query_grammar.pest` | PEG grammar for query language |
| `src/pattern.rs` | Pattern AST and constraints (ConstraintValue) |
| `src/searcher.rs` | CSP solver with regex matching |
| `src/tree.rs` | Tree/Word data structures |
| `src/bytes.rs` | String interning pool (BytestringPool) |
| `src/conllu.rs` | CoNLL-U parsing |
| `src/iterators.rs` | Parallel iteration |
| `src/python.rs` | Python bindings |
