# EXCEPT/OPTIONAL Implementation Tasks

## Pattern Changes
- [ ] Add `except_patterns` and `optional_patterns` fields to Pattern struct
- [ ] Update Pattern constructors to initialize new fields as empty vectors

## Grammar & Parser
- [ ] Rename `option_block` to `optional_block` and add `except_block` in grammar
- [ ] Update query.rs parser to collect EXCEPT/OPTIONAL blocks into Pattern fields
- [ ] Add validation: unique new variable names across EXCEPT/OPTIONAL blocks

## Searcher Refactoring
- [ ] Extract `solve_with_bindings()` helper from `find_all_matches()`
- [ ] Implement `has_any_match()` short-circuit function for EXCEPT
- [ ] Implement `process_optionals()` for OPTIONAL block handling
- [ ] Update `find_all_matches()` with EXCEPT filtering and OPTIONAL extension

## Testing
- [ ] Add tests for EXCEPT basic functionality
- [ ] Add tests for OPTIONAL basic functionality
- [ ] Add tests for combined EXCEPT + OPTIONAL queries
- [ ] Add tests for variable name validation errors
- [ ] Verify MATCH-only queries have no performance regression

## Ongoing
- [ ] Check plan + check development status + revise task list
