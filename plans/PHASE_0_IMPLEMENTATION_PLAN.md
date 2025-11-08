# Phase 0: Pattern Matching VM Implementation Plan

## Overview

This phase focuses on building and testing the core pattern matching virtual machine before integrating with CoNLL-U parsing or query language parsing. The VM is the heart of the system, so we want to get it right first.

**Duration Estimate**: 2-3 weeks of focused work

**Success Criteria**:
- VM can execute all instruction types correctly
- Pattern compiler selects optimal anchors and generates efficient bytecode
- Wildcard patterns work with BFS and bounded search
- Backtracking handles complex patterns correctly
- Test suite covers edge cases and validates match semantics
- Performance is reasonable on test fixtures (baseline for future optimization)

---

## Task Breakdown

### Task 1: Complete Core VM Instructions (3-4 days)

**Goal**: Implement all basic navigation and constraint-checking instructions.

#### Subtasks:

**1.1: Navigation Instructions**
- [ ] `MoveToChild` - with optional constraint filtering
- [ ] `MoveLeft` / `MoveRight` - sibling navigation
- [ ] Add helper method `get_child_matching(node, constraint)` on Tree
- [ ] Add helper method `get_sibling(node, direction)` on Tree
- **Tests**: Navigate trees with various shapes (linear, branching, deep)

**1.2: Additional Constraint Checking**
- [ ] `CheckForm` - match word form
- [ ] `CheckDepRel` - match dependency relation
- [ ] Compound constraint evaluation (And, Or from pattern.rs)
- **Tests**: Match nodes with various attribute combinations

**1.3: Control Flow Instructions**
- [ ] `Jump(offset)` - relative instruction pointer movement
- [ ] `Choice` - create backtrack point with alternatives
- [ ] `Commit` - discard backtrack points (cut operation)
- **Tests**: Patterns requiring branching control flow

**Deliverable**: VM can execute simple patterns like:
```rust
// Match: VERB with NOUN child having "nsubj" relation
vec![
    CheckPOS("VERB"),
    Bind(0),
    MoveToChild(Some(Constraint::POS("NOUN"))),
    CheckDepRel("nsubj"),
    Bind(1),
    Match,
]
```

---

### Task 2: Wildcard Search with BFS (3-4 days)

**Goal**: Implement bounded wildcard searches that guarantee shortest-path matches.

#### Subtasks:

**2.1: Descendant Search**
- [ ] Implement `scan_descendants(node, constraint, max_depth)` helper
- [ ] Use `VecDeque` for BFS traversal
- [ ] Track visited nodes to avoid cycles
- [ ] Return first match (shortest path guarantee)
- [ ] Respect depth limit (default: 7)
- **Tests**:
  - Find nodes at various depths
  - Verify shortest path selected when multiple matches exist
  - Ensure depth limit prevents runaway searches

**2.2: Ancestor Search**
- [ ] Implement `scan_ancestors(node, constraint, max_depth)` helper
- [ ] Walk up parent chain until match or root reached
- [ ] Respect depth limit
- **Tests**: Find ancestors at various distances

**2.3: Sibling Search**
- [ ] Implement `scan_siblings(node, constraint, direction)` helper
- [ ] Search left or right siblings in linear order
- [ ] Return first match (leftmost/rightmost)
- **Tests**: Find siblings in various positions

**2.4: Integration with VM**
- [ ] `ScanDescendants` instruction implementation
- [ ] `ScanAncestors` instruction implementation
- [ ] `ScanSiblings` instruction implementation
- [ ] Handle no-match cases gracefully (trigger backtracking)

**Deliverable**: VM can execute wildcard patterns like:
```rust
// Match: VERB ... REL (verb with any descendant that's a relative pronoun)
vec![
    CheckPOS("VERB"),
    Bind(0),
    ScanDescendants(Constraint::POS("REL")),
    Bind(1),
    Match,
]
```

---

### Task 3: Backtracking System (2-3 days)

**Goal**: Enable controlled backtracking for patterns with multiple possible matches.

#### Subtasks:

**3.1: Choice Point Management**
- [ ] Enhance `ChoicePoint` struct to properly track alternatives
- [ ] Implement `create_choice_point()` - save current state
- [ ] Implement `restore_choice_point()` - restore saved state and try alternative
- [ ] Handle nested choice points correctly
- **Tests**: Patterns requiring multiple levels of backtracking

**3.2: Alternative Ordering**
- [ ] Sort alternatives by preference (leftmost position first, then depth)
- [ ] Implement ordering helper for node comparisons
- **Tests**: Verify leftmost, shortest-path semantics

**3.3: Backtracking Instructions**
- [ ] Fully implement `Choice` instruction with alternatives
- [ ] Handle `Fail` instruction by triggering backtrack
- [ ] Implement `Commit` to prune search space
- **Tests**:
  - Pattern that succeeds on second alternative
  - Pattern that exhausts all alternatives and fails
  - Pattern with commit that prevents backtracking

**3.4: Memoization (Optional Optimization)**
- [ ] Add memoization table to VMState
- [ ] Cache results of subpattern matches
- [ ] Key: `(node_id, instruction_position)` → `Option<Bindings>`
- [ ] Check cache before executing expensive operations
- **Tests**: Verify performance improvement on patterns with repeated substructures

**Deliverable**: VM can handle ambiguous patterns correctly:
```rust
// Match: NOUN with either DET or ADJ child (tries DET first, backtracks to ADJ if needed)
vec![
    CheckPOS("NOUN"),
    Bind(0),
    Choice,  // Creates choice point
    MoveToChild(Some(Constraint::POS("DET"))),  // Try DET first
    Bind(1),
    Match,
    // If DET fails, backtrack here and try ADJ...
]
```

---

### Task 4: Pattern Compilation (3-4 days)

**Goal**: Compile high-level Pattern AST into optimized VM bytecode.

#### Subtasks:

**4.1: Anchor Selection**
- [ ] Implement `estimate_selectivity(constraint)` helper
  - Lemma constraints: high selectivity
  - POS constraints: medium selectivity
  - Any/wildcard: low selectivity
- [ ] Implement `select_anchor(pattern)` - choose most selective element
- [ ] Add fallback: if all equal selectivity, choose first
- **Tests**: Verify correct anchor selection for various patterns

**4.2: Constraint Compilation**
- [ ] Implement `compile_constraint(constraint) -> Vec<Instruction>`
- [ ] Handle `Lemma`, `POS`, `Form`, `DepRel` constraints
- [ ] Handle compound constraints (`And`, `Or`)
  - `And`: compile all checks sequentially
  - `Or`: compile with `Choice` and alternatives
- **Tests**: Complex constraint expressions compile correctly

**4.3: Edge Compilation**
- [ ] Implement `compile_edge(edge) -> Vec<Instruction>`
- [ ] Handle relation types:
  - `Child` → `MoveToChild`
  - `Parent` → `MoveToParent`
  - `Descendant` → `ScanDescendants`
  - `Ancestor` → `ScanAncestors`
  - `Precedes`/`Follows` → `ScanSiblings`
- [ ] Add edge label constraints (deprel matching)
- **Tests**: Each relation type compiles correctly

**4.4: Interleaved Verification Strategy**
- [ ] Implement `compile_pattern(pattern) -> (Vec<Instruction>, usize)`
- [ ] Start at anchor, verify its constraints
- [ ] Alternate between backward and forward verification
- [ ] Use `PushState`/`RestoreState` to manage multi-directional search
- [ ] Generate final `Match` instruction
- **Algorithm**:
```rust
1. Select anchor position
2. Emit constraint checks for anchor
3. Emit Bind(anchor_pos)
4. Set back_idx = anchor - 1, forward_idx = anchor + 1
5. Loop while progress possible:
   a. If back_idx valid:
      - PushState
      - Compile edge from back_idx to back_idx+1
      - Compile constraints for back_idx
      - Bind(back_idx)
      - back_idx -= 1
   b. If forward_idx valid:
      - RestoreState (back to anchor)
      - Compile edge from forward_idx-1 to forward_idx
      - Compile constraints for forward_idx
      - Bind(forward_idx)
      - forward_idx += 1
6. Emit Match
```
- **Tests**:
  - Linear patterns (A → B → C)
  - Branching patterns (A → B, A → C)
  - Patterns with wildcards

**4.5: Optimization Pass (Optional)**
- [ ] Instruction reordering: most selective constraints first
- [ ] Eliminate redundant checks
- [ ] Combine adjacent navigation instructions where possible
- **Tests**: Optimized bytecode produces same results faster

**Deliverable**: Can compile this pattern:
```rust
// Help [lemma="help"];
// To [lemma="to"];
// YHead [];
// Help -[xcomp]-> To;
// To -[obj]-> YHead;

let mut pattern = Pattern::new();
pattern.add_element(PatternElement::new("Help", Constraint::Lemma("help".into())));
pattern.add_element(PatternElement::new("To", Constraint::Lemma("to".into())));
pattern.add_element(PatternElement::new("YHead", Constraint::Any));
pattern.add_edge(PatternEdge {
    from: "Help".into(),
    to: "To".into(),
    relation: RelationType::Child,
    label: Some("xcomp".into()),
});
pattern.add_edge(PatternEdge {
    from: "To".into(),
    to: "YHead".into(),
    relation: RelationType::Child,
    label: Some("obj".into()),
});

let bytecode = compile_pattern(&pattern);
// Bytecode should anchor on "to" (most selective), verify relationships bidirectionally
```

---

### Task 5: Comprehensive Test Suite (2-3 days)

**Goal**: Build confidence in the VM through extensive testing.

#### Subtasks:

**5.1: Test Fixture Creation**
- [ ] Create `tests/fixtures.rs` with hand-coded test trees
- [ ] Fixture 1: Simple sentence "The dog runs"
- [ ] Fixture 2: Complex sentence with relative clause "The dog that barks runs"
- [ ] Fixture 3: Sentence with coordination "John and Mary ran"
- [ ] Fixture 4: Deep nesting (5+ levels)
- [ ] Fixture 5: Wide tree (10+ children)
- [ ] Helper function: `build_test_tree(structure) -> Tree`

**5.2: Match Semantics Tests**
- [ ] Test leftmost semantics: when multiple matches exist, return leftmost
- [ ] Test shortest-path semantics: for wildcards, return shortest
- [ ] Test deterministic behavior: same pattern on same tree always returns same match
- **Tests**: Create ambiguous patterns and verify consistent results

**5.3: Edge Case Tests**
- [ ] Empty tree
- [ ] Single-node tree
- [ ] Pattern with no matching nodes
- [ ] Pattern that matches root
- [ ] Pattern that matches leaves
- [ ] Wildcard with max_depth exceeded
- [ ] Circular pattern attempt (should fail)
- **Tests**: All edge cases handled gracefully (no panics)

**5.4: Complex Pattern Tests**
- [ ] Pattern with multiple wildcards
- [ ] Pattern with nested wildcards (A ... B ... C)
- [ ] Pattern with both ancestor and descendant searches
- [ ] Pattern requiring backtracking through 3+ alternatives
- [ ] Pattern with compound constraints (And, Or)
- **Tests**: Integration tests in `tests/integration_tests.rs`

**5.5: Failure Mode Tests**
- [ ] Pattern that should fail to match
- [ ] Pattern that starts matching but fails midway
- [ ] Pattern that exhausts all backtracking options
- [ ] Invalid pattern (disconnected components)
- **Tests**: Verify graceful failure and appropriate error handling

**Deliverable**: Test suite with 50+ tests covering:
- All instruction types
- All relation types
- Match semantics guarantees
- Edge cases
- Complex real-world-like patterns

---

### Task 6: Indexing Integration (1-2 days)

**Goal**: Connect the index lookup phase with VM execution.

#### Subtasks:

**6.1: Searcher Implementation**
- [ ] Create `src/searcher.rs` module
- [ ] Implement `TreeSearcher` struct combining index + VM
- [ ] Method: `search(tree, pattern) -> impl Iterator<Match>`
- **Algorithm**:
```rust
1. Build index from tree
2. Compile pattern to bytecode
3. Use anchor element to query index (most selective lookup)
4. For each candidate node:
   a. Run VM starting at candidate
   b. If match found, yield it
   c. If first_match_only, stop
5. If no candidates found, return empty
```

**6.2: Candidate Selection Strategy**
- [ ] Choose index lookup based on anchor constraint type
- [ ] Lemma → `index.get_by_lemma()`
- [ ] POS → `index.get_by_pos()`
- [ ] Form → `index.get_by_form()`
- [ ] DepRel → `index.get_by_deprel()`
- [ ] Any → return all nodes (fallback)
- **Tests**: Verify correct index used for each constraint type

**6.3: Integration Tests**
- [ ] End-to-end: pattern → searcher → matches
- [ ] Verify index reduces candidate set effectively
- [ ] Measure speedup vs. brute force (test all nodes)
- **Tests**: Full pipeline tests in `tests/search_tests.rs`

**Deliverable**: Working end-to-end search:
```rust
let tree = build_help_sentence_tree();  // "I help to write code"
let pattern = parse_help_pattern();      // Help -[xcomp]-> To -[obj]-> YHead
let searcher = TreeSearcher::new();
let matches: Vec<Match> = searcher.search(&tree, &pattern).collect();
assert_eq!(matches.len(), 1);
assert_eq!(matches[0].get_binding("YHead").lemma, "write");
```

---

### Task 7: Performance Baseline (1-2 days)

**Goal**: Establish performance baselines for future optimization.

#### Subtasks:

**7.1: Benchmark Suite Setup**
- [ ] Create `benches/vm_benchmark.rs` using Criterion
- [ ] Benchmark: Simple pattern (2 nodes, 1 edge)
- [ ] Benchmark: Medium pattern (4 nodes, 3 edges)
- [ ] Benchmark: Complex pattern (6+ nodes, wildcards)
- [ ] Benchmark: Worst case (pattern matches nothing, full tree scan)

**7.2: Profiling**
- [ ] Run benchmarks with various tree sizes (10, 50, 100, 500 nodes)
- [ ] Identify bottlenecks with `cargo flamegraph` (optional)
- [ ] Document performance characteristics

**7.3: Performance Targets**
- [ ] Simple pattern on 50-node tree: < 10μs
- [ ] Complex pattern on 50-node tree: < 100μs
- [ ] Wildcard pattern on 100-node tree: < 500μs
- **If targets not met**: Note for Phase 2 optimization, don't block now

**Deliverable**:
- Benchmark suite runs successfully
- Baseline performance documented
- No egregious performance problems (>10ms for single sentence)

---

### Task 8: Documentation & Examples (1 day)

**Goal**: Make the VM understandable and usable for next phase.

#### Subtasks:

**8.1: API Documentation**
- [ ] Add rustdoc comments to all public types and methods
- [ ] Document instruction semantics clearly
- [ ] Document match guarantees (leftmost, shortest-path)
- [ ] Document compilation strategy

**8.2: Usage Examples**
- [ ] Create `examples/simple_match.rs` - basic pattern matching
- [ ] Create `examples/wildcard_search.rs` - using wildcards
- [ ] Create `examples/complex_pattern.rs` - multi-edge pattern
- [ ] Add comments explaining what's happening

**8.3: Phase 0 Retrospective**
- [ ] Update PROJECT_SUMMARY.md with Phase 0 completion status
- [ ] Document any design decisions that changed during implementation
- [ ] List any deferred optimizations or features for later phases

**Deliverable**:
- `cargo doc --open` produces readable documentation
- Examples run successfully
- Clear handoff to Phase 1

---

## Testing Strategy

### Unit Tests
- Each module has `#[cfg(test)] mod tests` with focused unit tests
- Test individual functions and methods in isolation
- Use simple, hand-crafted test data

### Integration Tests
- `tests/` directory contains full end-to-end tests
- Test complete workflows: pattern → compilation → execution → results
- Use realistic test fixtures

### Property-Based Testing (Optional)
- Consider `proptest` or `quickcheck` for:
  - Match semantics invariants
  - Bytecode validity
  - No panics on arbitrary patterns
- Defer to Phase 2 if time-constrained

### Test Coverage Target
- Aim for >80% code coverage (use `cargo tarpaulin`)
- Critical paths (VM execution, BFS) should have 100% coverage

---

## Development Workflow

### Daily Workflow
1. Pick next subtask from current task
2. Write tests first (TDD approach)
3. Implement minimal code to pass tests
4. Refactor for clarity
5. Run full test suite: `cargo test`
6. Check for warnings: `cargo clippy`
7. Format code: `cargo fmt`
8. Commit with clear message

### Weekly Checkpoints
- Review progress against plan
- Adjust estimates if needed
- Document any blockers or design questions
- Update PROJECT_SUMMARY.md

### Branch Strategy
- `main` branch: always buildable, tests pass
- Feature branches: `phase0/task-N-description`
- Merge to main after each task completion
- Tag: `v0.1.0-phase0` when complete

---

## Success Metrics

At completion of Phase 0, we should have:

- ✅ VM executes all instruction types correctly
- ✅ Pattern compiler generates efficient bytecode
- ✅ Wildcard searches work with BFS and bounds
- ✅ Backtracking handles ambiguous patterns
- ✅ 50+ tests passing, >80% coverage
- ✅ End-to-end search pipeline works (index → VM → results)
- ✅ Performance baselines documented
- ✅ Examples demonstrating core functionality
- ✅ Code is well-documented and maintainable

**Definition of Done**:
- All tasks marked complete
- `cargo test` passes with 0 failures
- `cargo clippy` produces no warnings
- Documentation complete
- Ready to begin Phase 1 (CoNLL-U integration)

---

## Risk Mitigation

### Risk: Backtracking too complex to implement correctly
**Mitigation**: Start with simple non-backtracking patterns, add backtracking incrementally. If needed, defer advanced backtracking to Phase 2.

### Risk: Performance not meeting targets
**Mitigation**: Focus on correctness first. Note performance issues for Phase 2 optimization. Don't prematurely optimize.

### Risk: Match semantics ambiguous in edge cases
**Mitigation**: Write tests for ambiguous cases early. Document decisions clearly. When in doubt, err on side of simplicity.

### Risk: Wildcard searches too slow/complex
**Mitigation**: Implement depth limits strictly. Consider simpler wildcard semantics if needed.

---

## Next Steps After Phase 0

Once Phase 0 is complete, proceed to **Phase 1: MVP Integration**:
1. CoNLL-U file reader
2. Full tree data structures (with all CoNLL-U fields)
3. Query language parser (using nom or pest)
4. Python bindings for basic usage
5. Single-file corpus processing

Phase 0 provides the solid foundation for all subsequent work.
