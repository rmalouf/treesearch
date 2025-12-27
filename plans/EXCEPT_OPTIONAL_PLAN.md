# Plan: EXCEPT and OPTIONAL Blocks for CSP Solver

## Summary

Extend the CSP solver to support EXCEPT and OPTIONAL blocks, enabling:
- **EXCEPT**: Find matches where MATCH succeeds but EXCEPT fails (negative existential)
- **OPTIONAL**: Bind additional variables if possible, otherwise continue without them

## Design Decisions

- **EXCEPT logic**: ANY - reject if ANY EXCEPT block matches
- **Unbound OPTIONAL vars**: Absent from bindings dict (check with `bindings.get()`)
- **Multiple OPTIONAL matches**: If an OPTIONAL block matches N ways, produce N extended solutions
- **Multiple OPTIONAL blocks**: Cross-product semantics (combine all extensions)
- **Validation**: Allow disconnected blocks (no forced connection to MATCH vars)
- **Variable scoping**:
  - EXCEPT/OPTIONAL blocks can reference MATCH variables (e.g., `V -> M` where V is from MATCH)
  - EXCEPT/OPTIONAL blocks can introduce their own NEW local variables (e.g., M)
  - EXCEPT/OPTIONAL blocks CANNOT reference variables from other EXCEPT/OPTIONAL blocks
  - Each extension block is evaluated independently against the MATCH solution
- **AllDifferent across blocks**: Extension vars must bind to different words than MATCH vars
- **Processing order**: EXCEPT filtering first, then OPTIONAL extension
- **Keyword**: Rename `OPTION` to `OPTIONAL` in grammar

## Architecture

### Key Insight: DFS Already Handles Partial Assignments

The existing `dfs()` function already:
- Filters for unassigned variables (`assign[var_id].is_none()`)
- Works correctly with any initial state

We don't need a separate search function - just provide different initial state to the same DFS.

### Approach: Extend Pattern, Keep API Stable

Instead of introducing a new `Query` type:
1. **Add fields to Pattern itself** - `except_patterns: Vec<Pattern>` and `optional_patterns: Vec<Pattern>`
2. **Refactor initialization** - Extract a helper `solve_with_bindings(tree, pattern, initial_bindings)`
3. **Keep `find_all_matches` signature unchanged** - internally handles EXCEPT/OPTIONAL
4. **Add `has_any_match()` for EXCEPT** - short-circuits on first match (optimization)

Normal patterns have empty vectors → zero overhead for basic queries.

## Files to Modify

| File | Changes |
|------|---------|
| `src/query_grammar.pest` | Rename `option_block` to `optional_block`, add `except_block` |
| `src/query.rs` | Parse EXCEPT/OPTIONAL blocks into Pattern fields |
| `src/pattern.rs` | Add `except_patterns` and `optional_patterns` fields to Pattern |
| `src/searcher.rs` | Add `solve_with_bindings()`, `has_any_match()`, update `find_all_matches()` |

## Implementation Phases

### Phase 1: Pattern Changes

**pattern.rs** - Add fields:
```rust
pub struct Pattern {
    // ... existing fields ...
    pub except_patterns: Vec<Pattern>,
    pub optional_patterns: Vec<Pattern>,
}
```

Default to empty vectors in constructors.

### Phase 2: Grammar and Parser

**query_grammar.pest**:
```pest
query = { SOI ~ match_block ~ (except_block | optional_block)* ~ EOI }
except_block = { "EXCEPT" ~ "{" ~ statement* ~ "}" }
optional_block = { "OPTIONAL" ~ "{" ~ statement* ~ "}" }
```

**query.rs** - Collect blocks into Pattern fields.

### Phase 3: Searcher Refactoring

**searcher.rs** - Extract initialization helper:

```rust
/// Search with pre-bound variables from initial_bindings.
/// Variables in initial_bindings are pre-assigned; others are solved.
fn solve_with_bindings(
    tree: &Tree,
    pattern: &Pattern,
    initial_bindings: &Bindings,
) -> Vec<Bindings>
```

Algorithm:
1. For each var in pattern, check if it's in initial_bindings
2. If yes: pre-assign in `assign`, mark word in `assigned_words`
3. If no: compute domain via node consistency
4. Run existing DFS
5. Return merged bindings (initial + new)

### Phase 4: EXCEPT Implementation

Add short-circuit helper:
```rust
/// Returns true if any match exists (short-circuits on first find).
fn has_any_match(
    tree: &Tree,
    pattern: &Pattern,
    initial_bindings: &Bindings,
) -> bool
```

Update `find_all_matches()`:
```rust
pub fn find_all_matches(tree: Tree, pattern: &Pattern) -> Vec<Match> {
    // Solve main pattern
    let base_solutions = solve_with_bindings(&tree, pattern, &HashMap::new());

    // Fast path: no extensions
    if pattern.except_patterns.is_empty() && pattern.optional_patterns.is_empty() {
        return base_solutions.into_iter()
            .map(|bindings| Match { tree: Arc::clone(&tree), bindings })
            .collect();
    }

    let mut results = Vec::new();
    for base in base_solutions {
        // EXCEPT: reject if ANY matches
        let rejected = pattern.except_patterns.iter()
            .any(|except| has_any_match(&tree, except, &base));

        if rejected { continue; }

        // OPTIONAL: extend with additional bindings
        let extended = process_optionals(&tree, &base, &pattern.optional_patterns);
        for bindings in extended {
            results.push(Match { tree: Arc::clone(&tree), bindings });
        }
    }
    results
}
```

### Phase 5: OPTIONAL Implementation

OPTIONAL semantics:
- Each OPTIONAL is evaluated independently against base MATCH bindings
- OPTIONAL blocks extend existing matches, they don't create new ones from nothing
- If OPTIONAL has 0 matches: keep solution unchanged (vars absent from bindings)
- If OPTIONAL has N matches: produce N extended solutions
- Multiple OPTIONAL blocks: cross-product of all extensions

```rust
fn process_optionals(
    tree: &Tree,
    base_bindings: &Bindings,
    optional_patterns: &[Pattern],
) -> Vec<Bindings> {
    if optional_patterns.is_empty() {
        return vec![base_bindings.clone()];
    }

    // For each OPTIONAL, collect possible extensions
    let mut extension_sets: Vec<Vec<Bindings>> = Vec::new();
    for optional in optional_patterns {
        let extensions = solve_with_bindings(tree, optional, base_bindings);
        extension_sets.push(extensions);
    }

    // Compute cross-product of all extensions
    let mut results = vec![base_bindings.clone()];

    for extensions in extension_sets {
        if extensions.is_empty() {
            // No match for this OPTIONAL - keep results unchanged
            continue;
        }
        // Replace each current result with extended versions
        let mut new_results = Vec::new();
        for result in &results {
            for ext in &extensions {
                let mut combined = result.clone();
                for (k, v) in ext {
                    if !combined.contains_key(k) {
                        combined.insert(k.clone(), *v);
                    }
                }
                new_results.push(combined);
            }
        }
        results = new_results;
    }

    results
}
```

### Phase 6: Tests

Add comprehensive tests for:
1. EXCEPT basic: Pattern with matching EXCEPT → rejected
2. EXCEPT non-matching: Pattern without matching EXCEPT → accepted
3. EXCEPT multiple blocks: Reject if ANY matches
4. OPTIONAL found: Variable present in bindings
5. OPTIONAL not found: Variable absent from bindings
6. OPTIONAL multiple matches: Returns all extended solutions
7. Multiple OPTIONAL blocks: Cross-product of extensions
8. Combined EXCEPT + OPTIONAL
9. Performance: MATCH-only queries unchanged (empty vectors)

## Performance Considerations

### No Regression for Basic Queries

- Empty `except_patterns` and `optional_patterns` vectors → fast-path return
- No overhead when only MATCH block is present
- Same DFS algorithm, same code path

### Extension Search Efficiency

- `has_any_match()` short-circuits for EXCEPT (returns on first match)
- Domain computation only for NEW variables (pre-assigned vars skip domain init)
- Reuses existing optimized DFS with MRV heuristic
- AllDifferent enforced via passed `assigned_words`

## Example Queries

### EXCEPT: Find verbs without adverb modifiers
```
MATCH {
    V [upos="VERB"];
    S [upos="NOUN"];
    V -[nsubj]-> S;
}
EXCEPT {
    M [upos="ADV"];
    V -[advmod]-> M;
}
```
Returns V-S pairs only when V has no advmod child.

### OPTIONAL: Capture object if present
```
MATCH {
    V [upos="VERB"];
    S [upos="NOUN"];
    V -[nsubj]-> S;
}
OPTIONAL {
    O [upos="NOUN"];
    V -[obj]-> O;
}
```
Returns `{V, S, O}` if object exists, `{V, S}` otherwise.

### Combined
```
MATCH { V [upos="VERB"]; }
EXCEPT { Aux [upos="AUX"]; Aux -> V; }
OPTIONAL { O []; V -[obj]-> O; }
```
Find verbs not governed by auxiliaries, optionally capturing their objects.

## Implementation Order

1. **Pattern changes**: Add `except_patterns` and `optional_patterns` fields
2. **Grammar**: Rename `option_block` → `optional_block`, add `except_block`
3. **Parser**: Collect EXCEPT/OPTIONAL blocks into Pattern fields
4. **Searcher**: Extract `solve_with_bindings()` helper
5. **Searcher**: Add `has_any_match()` for EXCEPT short-circuit
6. **Searcher**: Update `find_all_matches()` with EXCEPT/OPTIONAL logic
7. **Tests**: Add comprehensive test coverage
8. **Docs**: Update query language documentation
