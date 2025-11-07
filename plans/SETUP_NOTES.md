# Setup Notes & Considerations

**Date**: 2025-11-07
**Status**: Project setup complete, ready for Phase 0 implementation

---

## Current State

### ✅ Completed
- Rust project initialized with proper structure
- All core modules created with skeleton implementations
- Python bindings configured (PyO3 + maturin)
- Dependencies selected and configured
- Basic tests passing (5 tests)
- Comprehensive planning documents written
- Initial commit created (`f09bf31`)

### 📊 Project Stats
- 13 files
- ~2000 lines (mostly documentation)
- ~750 lines of actual Rust code (skeletons with basic tests)
- Compiles cleanly with 6 warnings (all minor: unused imports, unused fields)

---

## Quick Start (When Resuming)

```bash
# Verify everything still works
cargo test

# Check for any dependency updates
cargo update

# Start Phase 0, Task 1: Complete Core VM Instructions
# See PHASE_0_IMPLEMENTATION_PLAN.md, Task 1.1
```

---

## Technical Decisions Made

### 1. **Module Structure**
- `tree.rs` - Simple Node/Tree structs (no arena allocation yet)
- `pattern.rs` - AST-based pattern representation
- `vm.rs` - Instruction enum + VMState + VM executor
- `index.rs` - HashMap-based inverted indices

**Note**: Current tree implementation uses direct Vec indexing. May need arena allocation or Rc/RefCell later for more complex navigation.

### 2. **Dependencies**
- **PyO3 0.22**: Slightly older but stable (0.27 available)
- **rayon 1.10**: Latest stable for parallelism
- **hashbrown 0.15**: Fast HashMap (0.16 available)
- **criterion 0.5**: For benchmarks (0.7 available)

**Note**: Intentionally chose slightly older versions for stability. Can update later if needed.

### 3. **VM Design Choices**
- Instruction-based (not stack-based) - more direct for tree navigation
- State includes backtracking stack for choice points
- Separate PushState/RestoreState for bidirectional search
- NodeId is usize (simple, works for single-sentence trees)

---

## Observations & Considerations

### Potential Issues to Watch

**1. Tree Navigation Efficiency**
Current implementation:
```rust
pub fn parent(&self, node_id: NodeId) -> Option<&Node> {
    self.get_node(node_id)
        .and_then(|node| node.parent)
        .and_then(|parent_id| self.get_node(parent_id))
}
```
This does double lookup. Consider caching or different access pattern.

**2. Pattern Compilation Not Yet Implemented**
The `Pattern` → `Vec<Instruction>` compilation is the most complex part. The plan outlines interleaved verification strategy, but implementation details will need careful thought.

**3. Backtracking Complexity**
The `ChoicePoint` struct currently has an unused `node_id` field (compiler warning). The backtracking logic needs to properly track which alternative to try next.

**4. Constraint Matching**
The `Constraint` enum has `And` and `Or` variants, but the matching logic isn't implemented yet. This will need recursive evaluation.

**5. BFS for Wildcards**
Need to carefully handle:
- Visited set (avoid infinite loops in malformed trees)
- Depth limits (prevent runaway searches)
- Shortest-path guarantee (BFS order ensures this)

---

## Recommendations for Phase 0

### Start Small
- Implement basic instructions first (CheckPOS, CheckLemma, MoveParent)
- Get simple linear patterns working before wildcards
- Add complexity incrementally

### Test-Driven Development
- Write test fixtures first (hand-coded trees)
- Write failing test for each instruction
- Implement minimal code to pass
- Refactor for clarity

### Pattern Compilation Strategy
- Start with manual bytecode construction for tests
- Implement anchor selection early (it's critical)
- Defer optimization passes until basic compilation works

### Performance
- Don't optimize prematurely
- Focus on correctness first
- Establish baseline benchmarks
- Profile before optimizing

---

## Things That Might Need Refactoring

### 1. Tree Representation
Current approach uses Vec with direct indexing. Alternatives to consider:
- **Arena allocation** (e.g., `typed-arena` crate) - better memory locality
- **Rc/RefCell** - more flexible but overhead
- **Petgraph** - full graph library (might be overkill)

Wait until VM implementation reveals actual access patterns before changing.

### 2. VM State Management
The state stack for bidirectional search might be clunky. Consider:
- Separate cursors for backward/forward search
- More explicit state machine for interleaved verification
- Helper methods to reduce boilerplate

### 3. Error Handling
Currently using `Result<bool, ()>` for instruction execution. This loses error information. Consider:
```rust
enum VMError {
    NavigationFailed,
    ConstraintFailed,
    UnexpectedEndOfProgram,
    // etc.
}
```

Better error messages will help debugging during Phase 0.

### 4. Match Result Structure
Currently just `HashMap<usize, NodeId>`. Eventually need:
- Named bindings (variable names from pattern)
- Matched sentence context
- Maybe span information for linear order

Can defer until Phase 1 (Python bindings).

---

## Warnings to Address

Current `cargo check` produces 6 warnings:

1. **`feature = "python"` not defined** - Remove `#[cfg(feature = "python")]` or add feature to Cargo.toml
2. **Unused imports in tree.rs** - Remove `std::rc::Rc` and `std::cell::RefCell`
3. **Unused import in pattern.rs** - Remove `crate::tree::NodeId`
4. **Unused import in vm.rs** - Remove `Node`
5. **Unused field `node_id` in ChoicePoint** - Will be used when backtracking is implemented
6. **Unused import in lib.rs tests** - Remove `use super::*;`

These are all minor and don't affect functionality, but clean them up in first task.

---

## Questions for Future Consideration

### Architecture
- Should we support multiple sentences in one Tree? (probably not for Phase 0)
- How to handle multi-word tokens in CoNLL-U? (defer to Phase 1)
- Should patterns be compiled once and reused? (yes, eventually)

### Query Language
- Do we need negation (~[pos="NOUN"])? (defer)
- Do we need precedence operators (A . B)? (defer to Phase 2)
- Should we support regex in constraints? (yes, soon after literals)

### Performance
- Pre-compile common patterns? (defer)
- Cache compiled bytecode? (yes, eventually)
- Parallel sentence processing? (Phase 2)
- Incremental indexing? (Phase 2 or later)

### Python API
- Iterator-based or return all matches? (iterator - more flexible)
- How to expose tree navigation in Python? (Phase 1 decision)
- Should Python users see VM bytecode? (probably not)

---

## Nice-to-Have Features (Not Blocking)

### Development Tools
- `cargo watch` for continuous testing during development
- `cargo-tarpaulin` for code coverage metrics
- `cargo-flamegraph` for profiling
- `cargo-expand` to see macro expansions (PyO3 uses macros heavily)

### Documentation
- Rustdoc examples that actually run (doc tests)
- Architecture diagrams (draw.io or mermaid)
- Query language BNF grammar (when we parse it)

### Testing
- Property-based tests with `proptest`
- Fuzzing with `cargo-fuzz`
- Comparison with reference implementations (Semgrex, CQP)

---

## Success Criteria Checklist (For Later)

Phase 0 is complete when:

- [ ] All VM instructions implemented and tested
- [ ] BFS wildcard search working with depth limits
- [ ] Backtracking handles ambiguous patterns correctly
- [ ] Pattern compilation from AST to bytecode works
- [ ] 50+ tests passing with >80% coverage
- [ ] End-to-end: Pattern → Index → VM → Match works
- [ ] Performance baseline documented
- [ ] Examples demonstrate core functionality
- [ ] Code is clean and documented
- [ ] Ready for Phase 1 (CoNLL-U parsing)

---

## Resources & References

### Rust Learning
- https://doc.rust-lang.org/book/ - The Rust Book
- https://doc.rust-lang.org/rust-by-example/ - Learn by example
- https://docs.rs/ - Crate documentation

### Relevant Crates
- PyO3: https://pyo3.rs/
- Rayon: https://docs.rs/rayon/
- Criterion: https://docs.rs/criterion/

### Similar Tools
- **Semgrex** (Stanford) - Dependency pattern matching in Java
- **CQP** (IMS Stuttgart) - Corpus Query Processor
- **Tgrep2** - Tree pattern matching
- **PML-TQ** - Complex linguistic queries

### CoNLL-U Format
- https://universaldependencies.org/format.html
- https://universaldependencies.org/

---

## Final Thoughts

The project is well-structured and ready for implementation. The algorithm-first approach is sound - getting the VM right will inform all other design decisions.

Key principles moving forward:
1. **Simplicity** - Start minimal, add complexity only when needed
2. **Correctness** - Match semantics must be deterministic and predictable
3. **Performance** - Rust gives us headroom; don't over-optimize early
4. **Testability** - Good tests are the foundation for refactoring
5. **Pragmatism** - Perfect is the enemy of done; iterate based on real usage

The detailed Phase 0 plan provides clear guidance. Take it one task at a time, one subtask at a time. By the end of Phase 0, we'll have a solid foundation for the rest of the project.

Good luck with Phase 0 implementation! 🚀
