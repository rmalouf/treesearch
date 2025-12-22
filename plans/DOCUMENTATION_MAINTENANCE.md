# Documentation Maintenance Notes

## Lesson: Always Check Source of Truth First

### The Problem (Dec 2024)
Updating API.md took 4-5 iterations because I didn't check the actual implementation first. Made assumptions about what the API was, then incrementally fixed issues as they were discovered.

### The Right Approach

**When updating API documentation:**

1. **FIRST: Map the actual API** by reading source files:
   ```bash
   # Python wrapper API
   grep -E "^def |^class " python/treesearch/__init__.py

   # Rust bindings
   grep -E "fn [a-z_]+\(" src/python.rs | grep "#\[pyo"
   ```

2. **Create a reference checklist** of:
   - All public functions with signatures
   - All classes with their methods
   - All properties and attributes
   - Correct parameter names and types

3. **Compare docs to checklist** systematically:
   - What's in docs but not in API? (remove/update)
   - What's in API but not in docs? (add)
   - What has wrong signatures? (fix)

4. **Make ALL fixes at once** using the checklist

### For Treesearch Specifically

**Source of truth files:**
- `python/treesearch/__init__.py` - All public functions
- `src/python.rs` - All classes and their methods
- `python/treesearch/treesearch.pyi` - Type hints (should match reality)

**Current API surface (Dec 2024):**

Functions:
- `load(source)` → Treebank
- `from_string(text)` → Treebank
- `compile_query(query)` → Pattern
- `trees(source, ordered=True)` → TreeIterator
- `search(source, query, ordered=True)` → MatchIterator
- `search_trees(trees, query)` → MatchIterator

Classes:
- `Treebank` with methods:
  - `.trees(ordered=True)`
  - `.search(pattern, ordered=True)`
- `Tree` with methods:
  - `.word(id)` - raises IndexError
  - `[id]` - indexing syntax
- `Word` with properties:
  - `.form`, `.lemma`, `.upos`, `.deprel`, etc.
  - `.parent()`, `.children()`, `.children_by_deprel()`

**Watch out for:**
- Function renames (e.g., `parse_query` → `compile_query`)
- Method renames (e.g., `.matches()` → `.search()`)
- Old synonyms that were removed (e.g., `open()` vs `load()`)

### Quick Verification

Before committing doc changes:
```bash
# Check for old function names
grep -E "parse_query|get_trees|get_matches|\.matches\(" API.md

# Check for consistency with Python API
diff <(grep "^def " python/treesearch/__init__.py) \
     <(grep -E "^#### \`[a-z_]+\(" API.md)
```

### General Principle

**Code is truth, docs are derivative.**

When docs and code disagree, code wins. Always check implementation first, then update docs to match.
