# Phase 1: MVP Integration Plan

**Status**: In Progress (Nov 2025)
**Goal**: Integrate CoNLL-U parsing, complete end-to-end pipeline, add Python bindings

## Prerequisites (Completed in Phase 0)
- ✅ Core VM with all instructions
- ✅ Pattern compiler
- ✅ Query language parser (bonus early completion)
- ✅ Index implementation
- ✅ 56 tests passing

## Phase 1 Objectives

1. ✅ Parse real CoNLL-U treebank files
2. ✅ Full tree representation with all linguistic annotations
3. ✅ End-to-end search pipeline (index → candidates → VM → results)
4. ✅ Fix leftmost semantics to use actual token positions
5. ✅ Basic Python bindings for query execution
6. ✅ Single-file corpus processing

---

## Task 1: Full CoNLL-U Tree Structure (1-2 days)

### Goal
Extend tree.rs to support all CoNLL-U fields and linear position tracking.

### Current State
- Minimal Node: id, form, lemma, pos, deprel
- Missing: xpos, feats, head, deps, misc, position

### Implementation Plan

**1.1: Enhanced Node Structure**
```rust
pub struct Node {
    // Position tracking
    pub id: TokenId,           // Can be int, range, or decimal
    pub position: usize,       // Linear position in sentence (for leftmost semantics)

    // CoNLL-U fields
    pub form: String,          // Word form
    pub lemma: String,         // Lemma
    pub upos: String,          // Universal POS
    pub xpos: Option<String>,  // Language-specific POS
    pub feats: Features,       // Morphological features
    pub head: Option<NodeId>,  // Dependency head (was parent)
    pub deprel: String,        // Dependency relation
    pub deps: Vec<Dep>,        // Enhanced dependencies
    pub misc: Misc,            // Miscellaneous annotations

    // Tree navigation (computed)
    pub(crate) children: Vec<NodeId>,
}
```

**1.2: Supporting Types**
```rust
pub enum TokenId {
    Single(usize),              // Normal token: 1, 2, 3
    Range(usize, usize),        // Multiword token: 1-2
    Decimal(usize, usize),      // Empty node: 2.1
}

// Type aliases for simplicity (internal API)
pub type Features = HashMap<String, String>;
pub type Misc = HashMap<String, String>;

pub struct Dep {
    pub head: Option<NodeId>,  // None = root attachment (head=0)
    pub deprel: String,
}
```

**1.3: Tree Enhancements**
```rust
pub struct Tree {
    pub nodes: Vec<Node>,
    pub root_id: Option<NodeId>,
    pub sentence_text: Option<String>,  // Original text
    pub metadata: HashMap<String, String>, // sent_id, text, etc.
}
```

**Tests**:
- Create nodes with all fields
- Parse features and deps correctly
- Position tracking for leftmost semantics

---

## Task 2: CoNLL-U Parser (2-3 days)

### Goal
Parse CoNLL-U files into Tree structures.

### Implementation Plan

**2.1: Create `src/conllu.rs`**
- Line-by-line parsing
- Handle comments (starting with #)
- Handle multiword tokens
- Handle empty nodes
- Sentence-level metadata

**2.2: Parser Structure**
```rust
pub struct CoNLLUReader {
    // Iterator over sentences
}

impl CoNLLUReader {
    pub fn from_file(path: &Path) -> Result<Self, ParseError>;
    pub fn from_str(text: &str) -> Result<Self, ParseError>;
}

impl Iterator for CoNLLUReader {
    type Item = Result<Tree, ParseError>;
    // Yields one sentence at a time
}
```

**2.3: Field Parsing**
```rust
fn parse_line(line: &str) -> Result<Node, ParseError>;
fn parse_features(s: &str) -> Result<Features, ParseError>;
fn parse_deps(s: &str) -> Result<Vec<Dep>, ParseError>;
fn parse_misc(s: &str) -> Result<Misc, ParseError>;
```

Note: All parsing functions return Result to catch malformed data instead of silently skipping invalid entries.

**Tests**:
- Simple sentence (3-5 tokens)
- Sentence with metadata
- Sentence with multiword tokens
- Sentence with empty nodes
- Sentence with complex features
- Invalid input (error handling)

---

## Task 3: TreeSearcher Integration (1-2 days)

### Goal
Create end-to-end search combining index + compiler + VM.

### Implementation Plan

**3.1: Create `src/searcher.rs`**
```rust
pub struct TreeSearcher {
    // Stateless - can be reused
}

impl TreeSearcher {
    pub fn new() -> Self;

    pub fn search<'a>(
        &self,
        tree: &'a Tree,
        pattern: &Pattern,
    ) -> impl Iterator<Item = Match> + 'a;

    pub fn search_query<'a>(
        &self,
        tree: &'a Tree,
        query: &str,
    ) -> Result<impl Iterator<Item = Match> + 'a, ParseError>;
}
```

**3.2: Search Algorithm**
```rust
1. Build index from tree
2. Compile pattern to (bytecode, anchor_idx)
3. Get anchor element constraint
4. Query index for candidates based on constraint:
   - Lemma constraint → index.get_by_lemma()
   - POS constraint → index.get_by_pos()
   - Form constraint → index.get_by_form()
   - Any → all nodes
5. For each candidate:
   a. Create VM with bytecode
   b. Execute from candidate node
   c. If match, yield it
6. Return iterator over matches
```

**Tests**:
- End-to-end: query string → matches
- Index filtering reduces candidates
- All match results are valid

---

## Task 4: Fix Leftmost Semantics (0.5 day)

### Goal
Use actual token position instead of node ID for ordering.

### Changes Needed

**4.1: Update `order_alternatives` in vm.rs**
```rust
fn order_alternatives(nodes: Vec<NodeId>, tree: &Tree) -> Vec<NodeId> {
    let mut nodes_with_pos: Vec<_> = nodes
        .into_iter()
        .map(|id| (id, tree.get_node(id).unwrap().position))
        .collect();
    nodes_with_pos.sort_by_key(|(_, pos)| *pos);
    nodes_with_pos.into_iter().map(|(id, _)| id).collect()
}
```

**4.2: Update all calls to `order_alternatives`**
- Pass tree reference
- Update tests to use position-based ordering

**Tests**:
- Verify leftmost node selected based on position, not ID
- Test with nodes where ID order ≠ position order

---

## Task 5: Python Bindings (2-3 days)

### Goal
PyO3 bindings for basic query execution.

### Implementation Plan

**5.1: Python Module Structure**
```python
# treesearch.pyi stub file
from typing import Iterator, Dict

class Tree:
    @staticmethod
    def from_conllu(path: str) -> Tree: ...

class Match:
    bindings: Dict[str, Node]

class Node:
    form: str
    lemma: str
    upos: str
    # ... other fields

class Searcher:
    def search(self, tree: Tree, query: str) -> Iterator[Match]: ...
```

**5.2: Rust PyO3 Implementation** (`src/python.rs`)
```rust
#[pyclass]
struct PyTree {
    inner: Tree,
}

#[pymethods]
impl PyTree {
    #[staticmethod]
    fn from_conllu(path: &str) -> PyResult<Self>;
}

#[pyclass]
struct PySearcher {
    inner: TreeSearcher,
}

#[pymethods]
impl PySearcher {
    #[new]
    fn new() -> Self;

    fn search(&self, tree: &PyTree, query: &str) -> PyResult<PyMatchIterator>;
}
```

**5.3: Build System**
- Configure maturin
- Test `maturin develop`
- Create Python test suite

**Tests**:
- Import module in Python
- Load CoNLL-U file
- Execute query
- Iterate over results
- Access match fields

---

## Task 6: Documentation & Examples (1 day)

### Goal
Comprehensive docs for Phase 1 handoff.

### Deliverables

**6.1: Rustdoc**
- Document all public APIs
- Add examples to doc comments
- Run `cargo doc --open`

**6.2: Examples**
```rust
// examples/conllu_search.rs
// Load CoNLL-U file, run query, print results

// examples/python_usage.py
// Python example of query execution
```

**6.3: README Update**
- Installation instructions
- Quick start guide
- API overview

---

## Success Metrics

Phase 1 complete when:
- ✅ Can parse real CoNLL-U files
- ✅ TreeSearcher provides end-to-end search
- ✅ Leftmost semantics use token position
- ✅ Python bindings work for basic queries
- ✅ Can process single CoNLL-U file from Python
- ✅ Documentation updated
- ✅ All tests passing (target: 75+ tests)

---

## Next: Phase 2

After Phase 1, move to:
- Multi-file corpus processing
- Parallel processing with rayon
- Performance optimization
- Extended query features
