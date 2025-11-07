# Dependency Tree Query Toolkit - Project Summary

## Project Overview

A high-performance toolkit for querying linguistic dependency parses at scale. Core implementation in Rust with Python bindings for ease of use in corpus linguistic research workflows.

**Primary Use Case**: Historical corpus linguistics research requiring structural pattern matching over large treebanks (500M+ tokens, 1000s of files).

## Core Requirements

### Functionality
- Read dependency parses from CoNLL-U format files
- Execute structural queries to find matching patterns in dependency trees
- Return matched tree structures to Python for custom analysis/annotation
- Save sentences containing matches to new CoNLL-U files (subcorpus extraction)
- Handle very large corpora (500M tokens, 1000s of files) efficiently
- Operate without requiring a precomputed index (though optional indexing may be added later)

### Performance Targets
- Process treebanks at scale (targeting 3+ billion nodes)
- Avoid pathological backtracking on complex wildcard patterns
- File-level parallelization for multi-file corpora
- Memory-efficient streaming where possible

### User Workflow
1. Write structural queries in a declarative pattern language
2. Execute queries to find matching subtrees
3. Use Python code to extract custom annotations from matched structures
4. Export results (e.g., to DataFrames for analysis)

## Architecture

### Core Components

**1. Parsing Layer (Rust)**
- CoNLL-U file readers
- Lazy loading for large corpora
- Sentence-by-sentence processing

**2. Data Structures (Rust)**
- Dependency tree representation
- Token attributes (form, lemma, POS, features, deprel)
- Efficient tree navigation (parent, children, siblings)

**3. Query Engine (Rust)**
- Two-phase matching: indexing + VM-based verification
- Virtual machine for pattern matching execution
- Leftmost, shortest-path match semantics (deterministic, avoids exponential search)
- Wildcard support with bounded search

**4. Python Bindings (PyO3)**
- Pythonic API for query execution
- Return navigable tree objects to Python
- Iterator-based result streaming

### Technology Stack
- **Language**: Rust (latest stable)
- **Python bindings**: PyO3 + maturin
- **Parallelization**: rayon
- **Query parsing**: nom or pest (TBD)

## Query Language Design

### Syntax Decisions

```rust
// Node declarations with constraints
Help [lemma="help"];
To [lemma="to"];
YHead [];

// Edge declarations (structural relations)
Help -[xcomp]-> To;
To -[obj]-> YHead;
```

**Key Features**:
- No `pattern` wrapper keyword (just declarations)
- Semicolons terminate statements
- `Head -[deprel]-> Dependent` syntax for relations
- Node IDs bind to match results
- Start with literal string matching, add regex soon after

**Philosophy**: Use query language to find relevant structures, then use custom Python/Rust code to extract annotations. Query language doesn't need to do everything.

## Pattern Matching Algorithm

### Core Approach
Virtual machine-based matcher with controlled backtracking:

1. **Index lookup**: Use inverted indices to find candidate anchor nodes
2. **VM verification**: Execute compiled pattern instructions to verify structural constraints
3. **Match semantics**: Leftmost, shortest-path to ensure deterministic results

### VM Instruction Set
- Tree navigation (parent, child, sibling, ancestor, descendant)
- Constraint checking (labels, features, positions)
- Wildcard expansion with BFS and early termination
- Minimal backtracking (only where necessary)

### Key Optimizations
- **Anchor selection**: Choose most selective node as starting point
- **Bidirectional verification**: Expand from anchor in both directions
- **Memoization**: Cache subpattern results
- **Pre-indexing**: Build indices for common constraints (lemma, POS, deprel)
- **Early termination**: Stop on first valid match

See `pattern_matching_vm_design.md` for detailed algorithm design.

## Current Status

### Design Phase
- ✅ High-level architecture defined
- ✅ Query language syntax decided
- ✅ Pattern matching algorithm designed
- ✅ Project structure set up (Rust + Python)

### Implementation Phase (Current)
- ✅ **Project Setup Complete**
  - Rust project initialized with core modules (tree, pattern, vm, index)
  - Python bindings configured (PyO3 + maturin)
  - Dependencies configured (rayon, hashbrown)
  - Basic module skeletons with passing tests
  - Documentation structure in place

- 🚧 **Phase 0: Matching Algorithm (CURRENT PRIORITY)**
  - [See PHASE_0_IMPLEMENTATION_PLAN.md for detailed breakdown]
  - Next: Complete core VM instructions
  - Estimated: 2-3 weeks

**Rationale for Algorithm-First Approach**: The matching algorithm is the core of the project and the hardest part. Its implementation will guide the design of tree representations and other data structures. By building and testing the VM-based matcher first, we ensure that all other components are optimized for the matching workflow.

### Implementation Phases

**Phase 0: Matching Algorithm (CURRENT PRIORITY)**
- ✅ Minimal tree data structure (just enough for testing matching)
- ✅ Pattern AST representation
- ⏳ VM instruction set implementation
- ⏳ VM executor with backtracking
- ⏳ Hand-coded test fixtures
- ⏳ Algorithm verification and optimization

**Phase 1: MVP Integration**
- CoNLL-U reader
- Full tree data structures (informed by algorithm needs)
- Simple query language parser (node + edge patterns)
- Integration of pattern matcher with parsed queries
- Basic Python bindings
- Single-file processing

**Phase 2: Scale & Performance**
- Multi-file corpus handling
- Parallel processing (file-level)
- Memory optimization
- Extended query features (precedence, siblings, more constraints)
- Subcorpus extraction (save matching sentences to CoNLL-U files)

**Phase 3: Result Handling**
- Navigable tree objects in Python
- Result export (DataFrame, JSON, etc.)
- Query result caching

**Phase 4: Advanced Features (Future)**
- Optional pre-computed index for frequently-used corpora
- Jupyter widget for result visualization
- TUI for interactive exploration
- Query optimization/rewriting

## Example Use Case

**Research Question**: Analyze "help X to Y" constructions

**Query**:
```
Help [lemma="help"];
To [lemma="to"];
YHead [];

Help -[xcomp]-> To;
To -[obj]-> YHead;
```

**Python Post-Processing**:
```python
for match in results:
    help_node = match['Help']
    to_node = match['To']
    y_head = match['YHead']
    
    annotations = {
        'help_form': help_node.form,
        'word_before_help': help_node.prev_sibling.form if help_node.prev_sibling else None,
        'y_head_lemma': y_head.lemma,
        'x_length': len(list(to_node.subtree)),
        # ... more custom extraction
    }
```

## Development Context

**Developer Profile**:
- Computational linguistics professor
- Research focus: historical corpus linguistics
- Experienced coder (Python, Rust)
- Uses JupyterLab/PyCharm for development
- Familiar with NLP tools (spaCy, treebank query tools)

**Previous Approach**: 
- Used spaCy + PhraseMatcher for coarse filtering
- Lots of manual tree traversal code
- Fragile and janky
- This toolkit aims to be a robust, performant replacement

## Key Design Principles

1. **Performance matters**: Historical corpora are large; Rust core for speed
2. **Simplicity over completeness**: Start minimal, iterate based on real usage
3. **Python-friendly**: Researchers work in Python; bindings must be ergonomic
4. **Deterministic results**: Leftmost, shortest-path semantics avoid surprises
5. **No pathological cases**: Careful algorithm design to avoid exponential blowup

## References

- **Pattern matching algorithm**: See `pattern_matching_vm_design.md`
- **Query language examples**: See design conversations
- **CoNLL-U format**: https://universaldependencies.org/format.html

## Next Steps (Algorithm-First Implementation)

1. **Set up Rust project structure** with basic module layout
2. **Define minimal tree data structures** for testing the matcher
   - Node representation with ID, label, parent/child pointers
   - Just enough to write test fixtures
3. **Implement pattern AST** representation
   - Node patterns (with constraints)
   - Edge patterns (structural relations)
4. **Build VM instruction set**
   - Navigation instructions (parent, child, ancestor, descendant)
   - Constraint checking instructions
   - Wildcard expansion with BFS
5. **Implement VM executor** with controlled backtracking
6. **Create hand-coded test fixtures** to verify matching behavior
7. **Test and optimize** the matching algorithm
8. **Iterate on tree representation** based on algorithm needs
9. Then proceed to Phase 1: integrate with CoNLL-U parsing and query language
