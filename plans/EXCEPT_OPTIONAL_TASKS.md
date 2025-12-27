# EXCEPT/OPTIONAL Implementation Tasks

## Status: ✅ COMPLETED

All tasks completed. EXCEPT and OPTIONAL query blocks are now fully implemented and tested.

## Pattern Changes
- [x] Add `except_patterns` and `optional_patterns` fields to Pattern struct
- [x] Update Pattern constructors to initialize new fields as empty vectors

## Grammar & Parser
- [x] Rename `option_block` to `optional_block` and add `except_block` in grammar
- [x] Update query.rs parser to collect EXCEPT/OPTIONAL blocks into Pattern fields
- [x] Add validation: unique new variable names across EXCEPT/OPTIONAL blocks

## Searcher Refactoring
- [x] Extract `solve_with_bindings()` helper from `find_all_matches()`
- [x] Implement `has_any_match()` short-circuit function for EXCEPT
- [x] Implement `process_optionals()` for OPTIONAL block handling
- [x] Update `find_all_matches()` with EXCEPT filtering and OPTIONAL extension

## Testing
- [x] Add tests for EXCEPT basic functionality
- [x] Add tests for OPTIONAL basic functionality
- [x] Add tests for combined EXCEPT + OPTIONAL queries
- [x] Add tests for variable name validation errors
- [x] Verify MATCH-only queries have no performance regression

## Summary

- **Total tests**: 96 (was 92, added 4 new tests)
- **Files modified**:
  - `src/pattern.rs` (added fields + bug fix for negated edges)
  - `src/query_grammar.pest` (added EXCEPT/OPTIONAL blocks)
  - `src/query.rs` (updated parser + validation)
  - `src/searcher.rs` (new helpers + integration)
- **Bug fixed**: Missing `from` variable addition for negated labeled edges
