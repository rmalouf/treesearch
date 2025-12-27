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

**Current API surface (Dec 2025):**

Functions:
- `load(path)` → Treebank
- `from_string(text)` → Treebank
- `compile_query(query)` → Pattern
- `trees(source, ordered=True)` → Iterator[Tree]
- `search(source, query, ordered=True)` → Iterator[tuple[Tree, dict]]
- `search_trees(trees, query)` → Iterator[tuple[Tree, dict]]

Classes:
- `Treebank` with methods:
  - `.trees(ordered=True)`
  - `.search(query, ordered=True)`
- `Tree` with methods/properties:
  - `.word(id)` - raises IndexError
  - `[id]` - indexing syntax
  - `.sentence_text`, `.metadata`
  - `len(tree)`
- `Word` with properties:
  - `.id`, `.token_id`, `.form`, `.lemma`, `.upos`, `.xpos`, `.deprel`, `.head`
  - `.children_ids`, `.feats`, `.misc`
  - `.parent()`, `.children()`, `.children_by_deprel(deprel)`
- `Pattern` - opaque compiled pattern

**Watch out for:**
- Function renames (e.g., `parse_query` → `compile_query`)
- Method renames (e.g., `.matches()` → `.search()`)
- Old synonyms that were removed (e.g., `open()` vs `load()`)
- Property renames (e.g., `.pos` → `.upos`)

### Current Documentation Structure

```
docs/
├── index.md           # Landing page with quick start
├── tutorial.md        # Complete walkthrough
├── query-language.md  # Syntax reference
├── api.md             # Functions and classes
└── internals.md       # Architecture for contributors
```

### Quick Verification

Before committing doc changes:
```bash
# Check for old function names in docs
grep -rE "parse_query|get_trees|get_matches|\.matches\(|\.pos[^t]" docs/

# Verify API functions match
grep "^def " python/treesearch/__init__.py
```

### General Principle

**Code is truth, docs are derivative.**

When docs and code disagree, code wins. Always check implementation first, then update docs to match.
