# Phase 0 Progress Notes

**Date**: 2025-11-07
**Status**: Tasks 1-4 Complete + Query Parser (56 tests passing)

## Completed Tasks

### ✅ Task 1: Core VM Instructions
**Implemented**:
- All constraint checking instructions: CheckLemma, CheckPOS, CheckForm, CheckDepRel
- Navigation instructions: MoveParent, MoveChild (with optional constraint filtering), MoveLeft, MoveRight
- Control flow: Jump, Choice (basic), Commit
- State management: PushState, RestoreState
- Compound constraint evaluation: And/Or support in check_constraint helper

**Tests**: 19 total
- Constraint checks (lemma, POS, form, deprel)
- Navigation with/without constraints
- Sibling navigation and boundary conditions
- State management (push/restore)
- Compound constraints (And/Or)

### ✅ Task 2: Wildcard Search with BFS
**Implemented**:
- `scan_descendants`: BFS traversal with shortest-path guarantee, returns all matches at minimum depth
- `scan_ancestors`: Walk up parent chain, returns closest match
- `scan_siblings`: Search left/right siblings in order of proximity
- All scan operations return Vec<NodeId> to support backtracking
- Depth limit enforcement (default: 7 levels)
- Cycle detection with HashSet

**Tests**: 31 total (+12)
- BFS shortest-path verification
- Depth limit enforcement
- Ancestor/descendant searches at various depths
- Sibling searches in both directions
- Complex wildcard combinations
- Edge cases (no match, no parent, boundary conditions)

### ✅ Task 3: Backtracking System
**Implemented**:
- Enhanced ChoicePoint struct (removed unused node_id field)
- `create_choice_point`: Saves IP, bindings, and ordered alternatives
- `order_alternatives`: Ensures leftmost semantics (currently uses node ID as proxy)
- Modified navigation instructions to create choice points automatically
- Fixed backtrack logic: restores state and sets IP to instruction after choice
- Commit instruction properly clears backtrack stack

**Tests**: 39 total (+8)
- Success on second/third alternative
- Exhausting all alternatives (failure)
- Commit preventing backtracking
- Nested backtracking (multiple levels)
- Backtracking with scan operations
- Leftmost semantics verification
- Constraint filtering reduces alternatives

### ✅ Task 4: Pattern Compilation
**Implemented**:
- Selectivity estimation: High (lemma/form), Medium (POS/deprel), Low (any)
- Anchor selection: Chooses most selective element
- Constraint compilation: Generates check instructions, handles And (sequential), basic Or (first alternative)
- Edge compilation: Maps RelationType to navigation instructions
- Full pattern compilation: BFS from anchor, emits PushState + navigation + checks + Bind
- Returns (bytecode, anchor_index) tuple

**Tests**: 50 total (+11)
- Selectivity estimation
- Anchor selection with various constraints
- Constraint/edge compilation
- End-to-end: simple patterns
- End-to-end: parent-child patterns
- End-to-end: descendant patterns
- End-to-end: complex multi-edge patterns

### ✅ Query Language Parser (Phase 1 moved earlier)
**Implemented**:
- Pest-based parser for query language syntax
- Node declarations: `Name [constraint, constraint];`
- Edge declarations: `Parent -[label]-> Child;`
- Constraint types: lemma, pos, form, deprel
- Multiple constraints combined with And
- Comments support

**Tests**: 56 total (+6)
- Empty constraints
- Single and multiple constraints
- Edge declarations
- Complex multi-node queries
- All constraint types

**Example query**:
```
Help [lemma="help"];
To [lemma="to"];
YHead [];

Help -[xcomp]-> To;
To -[mark]-> YHead;
```

**Rationale for early implementation**: With the VM and compiler working, adding the parser makes testing and benchmarking more natural. Instead of manually constructing Pattern objects, we can now write realistic queries as strings.

## Current Architecture

```
Query String
    ↓
Parser (pest) → Pattern AST
    ↓
Compiler (select anchor, estimate selectivity)
    ↓
Bytecode (Instructions)
    ↓
VM (execute with backtracking)
    ↓
Match (bindings)
```

## Known Limitations & TODOs

### Compiler
1. **Or constraint compilation**: Currently only compiles first alternative. Need proper Choice-based implementation for full Or support.
2. **Interleaved verification**: Currently uses simple BFS from anchor. The plan mentions alternating backward/forward verification which is more complex.
3. **Parent-to-child edges**: Current implementation only handles edges FROM anchor TO other nodes. Reverse edges (e.g., NOUN -parent-> VERB) need MoveParent support.
4. **Disconnected patterns**: No validation that pattern graph is connected.
5. **Optimization pass**: No instruction reordering or redundant check elimination yet.

### Leftmost Semantics
- Currently uses **node ID ordering** as proxy for leftmost position
- In real CoNLL-U data, need to use actual **linear token position**
- This is acceptable for Phase 0 testing but must be fixed in Phase 1

### Tree Representation
- Minimal test-only implementation
- Missing: XPOS, features, head, enhanced dependencies
- No linear ordering information
- Phase 1 will need full CoNLL-U representation

### Backtracking
- Works correctly for all tested scenarios
- Choice instruction itself is currently a no-op (choice points created by navigation)
- May need explicit Choice instruction for Or constraint compilation

### Query Parser
- Currently only supports `->` arrow (child relations)
- Future extensions: Different arrow types for parent, ancestor, descendant relations
- Future extensions: Wildcard syntax (`...` for descendants)
- Future extensions: Or constraints in query syntax (currently must use multiple queries)

## Next Steps (Remaining Phase 0 Tasks)

**Revised Priority**: With query parser complete, we can now:
1. Write realistic tests using query syntax (easier than manual Pattern construction)
2. Prepare for indexing integration (Task 6) - the critical performance piece
3. Add benchmarks with realistic queries

The parser enables more natural development workflows going forward.

### Task 5: Comprehensive Test Suite (2-3 days)
**Priority: Medium** - Current 56 tests are good coverage, but could add:
- Test fixtures (`tests/fixtures.rs`) with hand-coded trees
- More edge cases: empty tree, single node, circular references
- Complex patterns: nested wildcards, multiple wildcards, compound constraints
- Failure mode tests: invalid patterns, disconnected graphs
- Match semantics tests: deterministic behavior, leftmost guarantees

**Suggested fixtures**:
- Simple: "The dog runs" (3 nodes, 2 edges)
- Complex: "The dog that barks runs" (relative clause)
- Coordination: "John and Mary ran"
- Deep nesting: 5+ levels
- Wide tree: 10+ children

### Task 6: Indexing Integration (1-2 days)
**Priority: High** - Critical for performance

**Create `src/searcher.rs`**:
```rust
pub struct TreeSearcher {
    // Combines index + pattern compilation + VM execution
}

impl TreeSearcher {
    pub fn search(&self, tree: &Tree, pattern: &Pattern) -> impl Iterator<Match> {
        // 1. Build index from tree
        // 2. Compile pattern
        // 3. Use anchor to query index (get candidates)
        // 4. Run VM on each candidate
        // 5. Yield matches
    }
}
```

**Anchor → Index mapping**:
- Lemma constraint → `index.get_by_lemma()`
- POS constraint → `index.get_by_pos()`
- Form constraint → `index.get_by_form()`
- DepRel constraint → `index.get_by_deprel()`
- Any constraint → all nodes (fallback)

**Tests**:
- Verify index reduces candidate set
- Compare indexed vs. brute-force performance
- End-to-end: Pattern → Searcher → Matches

### Task 7: Performance Baseline (1-2 days)
**Priority: Low** - Nice to have but not blocking

**Setup `benches/vm_benchmark.rs`**:
- Simple pattern (2 nodes, 1 edge): target < 10μs on 50-node tree
- Medium pattern (4 nodes, 3 edges): target < 100μs on 50-node tree
- Complex pattern (6+ nodes, wildcards): target < 500μs on 100-node tree
- Worst case (no match, full scan): measure baseline

**Optional profiling**:
- `cargo flamegraph` to identify hotspots
- Document bottlenecks for Phase 2 optimization

### Task 8: Documentation & Examples (1 day)
**Priority: Medium** - Important for handoff

**Rustdoc**:
- Document all public types and methods
- Document instruction semantics
- Document match guarantees (leftmost, shortest-path)
- Document compilation strategy

**Examples** (`examples/`):
- `simple_match.rs`: Basic pattern matching
- `wildcard_search.rs`: Using wildcards (descendants, ancestors)
- `complex_pattern.rs`: Multi-edge pattern like help-to-write
- `compiler_demo.rs`: Show compilation process

**Update planning docs**:
- Mark Task 1-4 complete in `PHASE_0_IMPLEMENTATION_PLAN.md`
- Document design decisions that changed
- List deferred optimizations for Phase 2

## Phase 1 Readiness

**What's ready**:
- ✅ Core matching VM fully functional
- ✅ Pattern compilation working
- ✅ Backtracking with leftmost semantics
- ✅ Wildcard search (BFS, shortest-path)
- ✅ Query language parser (implemented early for testing convenience)

**What Phase 1 needs**:
1. **CoNLL-U parser**: Read real treebank files
2. **Full tree representation**: All CoNLL-U fields + linear position
3. ~~**Query language parser**: User-friendly syntax → Pattern AST~~ ✅ Already done
4. **Multi-file processing**: Rayon parallelization
5. **Python bindings**: PyO3 wrapper for ease of use
6. **Fix leftmost semantics**: Use actual token positions, not node IDs

## Design Notes

### Why BFS for Descendants?
- Guarantees shortest path (closest match)
- Returns all matches at same depth for backtracking
- Prevents exponential blowup from deep searches
- Depth limit provides safety net

### Why Anchor-Based Compilation?
- Two-phase strategy: Index lookup → VM verification
- Most selective element = fewest candidates to verify
- Dramatically reduces search space (potentially 1000x+)
- Critical for 500M+ token corpora

### Why Stack-Based VM?
- Simple instruction set
- Easy to compile from AST
- Natural backtracking support (save/restore state)
- Fast execution (no interpretation overhead)
- Easy to add new instructions

### Backtracking vs. Deterministic Matching
- VM supports non-deterministic matching (multiple paths)
- **Always returns leftmost, shortest-path match** (deterministic)
- Backtracking enables this: try alternatives in order until first success
- Choice points created automatically by navigation instructions
- Alternative: Generate all matches (not implemented, not needed for current use case)

## Warnings to Address (Low Priority)

Current compiler warnings (non-critical):
- Unused imports: `std::rc::Rc`, `std::cell::RefCell` in tree.rs
- Unused import: `crate::tree::NodeId` in pattern.rs
- Unused import: `super::*` in lib.rs
- Missing feature: `python` in Cargo.toml (Phase 1)

Can be fixed with: `cargo fix --lib -p treesearch`

## Testing Strategy

**Current**: 56 unit/integration tests, all passing
- 50 VM/compiler tests
- 6 parser tests

**Coverage**: Good coverage of happy paths and common edge cases
**Missing**: Property-based testing (proptest/quickcheck) for invariants

**Future considerations**:
- Fuzz testing for pattern compilation and query parsing
- Property: Same pattern on same tree always returns same match
- Property: Leftmost match is always leftmost
- Property: Shortest-path match is always shortest

## Performance Notes

**Current performance**: Not measured (Phase 0 focus: correctness)

**Expected bottlenecks** (for Phase 2 optimization):
1. Constraint checking (called frequently in BFS)
2. HashMap lookups for bindings
3. Vec allocations for alternatives
4. BFS queue operations

**Potential optimizations**:
- Inline constraint checks
- SmallVec for alternatives (most patterns have few alternatives)
- Arena allocation for VM state
- Memoization for expensive operations

## Acknowledgments

This implementation closely follows the design in `plans/pattern_matching_vm_design.md` with some adaptations:
- Simplified state management (no separate saved_states stack, just state_stack)
- Automatic choice point creation (not manual Choice instruction)
- BFS returns all matches at depth (not just first)
- Simplified compiler (no full interleaved verification yet)

The core algorithms and semantics match the specification.
