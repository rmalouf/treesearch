# Architecture

How treesearch works under the hood.

## Overview

Treesearch uses a **constraint satisfaction** approach to pattern matching, treating queries as CSP (Constraint Satisfaction Problem) instances.

## Key Components

### 1. Query Parser

**File**: `src/query.rs`

Uses Pest to parse query strings into an AST:

```
Query String → Parser → Pattern AST
```

The parser handles:
- Variable declarations with constraints
- Edge relationships (dependencies)
- Precedence operators
- Comments and whitespace

### 2. Pattern Representation

**File**: `src/pattern.rs`

The Pattern AST contains:

- **Variables**: Names and their constraints (lemma, pos, etc.)
- **Edge constraints**: Parent-child relationships
- **Precedence constraints**: Linear word order

Constraints are represented as:
- Node attributes (equality checks)
- Binary constraints (edges between variables)

### 3. CSP Solver

**File**: `src/searcher.rs`

The core matching algorithm uses:

**Search Strategy**: Depth-first search with backtracking

**Optimization Techniques**:

1. **Forward Checking**: Prune domains before assignment
2. **MRV Heuristic**: Choose most constrained variable first
3. **Arc Consistency**: Check constraints before committing
4. **AllDifferent**: Global constraint (no two variables bind to same word)

**Algorithm**:

```
1. Order variables by constraint degree (MRV)
2. For each variable:
   a. Get candidate words (satisfy node constraints)
   b. Check arc consistency (satisfy edge constraints)
   c. Recursively assign remaining variables
   d. Backtrack if no valid assignment
3. Yield all valid complete assignments
```

**Constraint Checking**:

The `satisfies_var_constraint()` function evaluates node constraints efficiently:

- **Attribute constraints** (lemma, upos, form, deprel): Direct string pool comparison
- **Feature constraints**: Iterate through word features to find key-value matches
- **HasIncomingEdge**: Check if word has parent (via `word.head`), optionally matching deprel
- **HasOutgoingEdge**: Check if word has children matching optional deprel (uses `children_by_deprel()` helper)
- **Negation**: Recursively negate inner constraint
- **Conjunction**: All sub-constraints must be satisfied

Anonymous variables (`_`) in queries create these `HasIncomingEdge`/`HasOutgoingEdge` constraints rather than creating actual variable bindings.

### 4. Tree Representation

**File**: `src/tree.rs`

Trees use efficient data structures:

- **String Interning**: lasso with FxHash reduces memory
- **Word IDs**: 0-based indexing for fast lookup
- **Adjacency Lists**: Children stored as vectors

Each Word contains:
- Indices into string pool (form, lemma, pos, etc.)
- Parent ID and children IDs
- CoNLL-U metadata

### 5. CoNLL-U Parsing

**File**: `src/conllu.rs`

Features:

- **Streaming Parser**: Processes one sentence at a time
- **Gzip Detection**: Automatic via magic bytes
- **Error Reporting**: Line numbers for malformed input
- **Metadata Extraction**: Preserves # comments

### 6. Parallelization

**File**: `src/iterators.rs`

Uses rayon for file-level parallelism:

```
Files → rayon::par_iter → Process in parallel → Channel → Python iterator
```

- Each file processed by separate thread
- Results sent through channel to Python
- No guaranteed ordering in parallel mode

### 7. Python Bindings

**File**: `src/python.rs`

PyO3 provides:

- **Zero-copy**: Arc for shared tree ownership
- **Iterator Protocol**: Python-friendly iteration
- **Error Translation**: Rust errors → Python exceptions
- **GIL Management**: Minimal holding of Global Interpreter Lock

## Search Flow

```
┌─────────────────┐
│  Query String   │
└────────┬────────┘
         │ parse_query()
         ▼
┌─────────────────┐
│  Pattern AST    │
└────────┬────────┘
         │
         │ search()
         ▼
┌─────────────────┐
│   CSP Solver    │  ← Tree
└────────┬────────┘
         │
         │ DFS + Forward Checking
         ▼
┌─────────────────┐
│  Match Dicts    │
└─────────────────┘
```

## Performance Characteristics

### Time Complexity

- **Parse Query**: O(query length)
- **Search Tree**: O(n^k) where:
  - n = number of words in tree
  - k = number of variables in pattern
  - Optimized by forward checking and MRV

### Space Complexity

- **Tree Storage**: O(words) with string interning
- **Pattern**: O(variables + constraints)
- **Search State**: O(k) for current assignment

### Optimization Impact

Forward checking typically reduces search space by 90%+:

```
Without optimization: Try all n^k combinations
With forward checking: Try ~n candidates per variable
```

## Design Decisions

### Why Constraint Satisfaction?

Alternatives considered:

1. **Graph Isomorphism**: Too general, slower
2. **Regex on Linearized Trees**: Misses structural constraints
3. **SQL-style Queries**: Complex translation to tree matching

CSP provides:

- Natural representation of structural constraints
- Well-studied optimization techniques
- Exhaustive search guarantee

### Why Exhaustive Search?

- Corpus linguistics requires **all** matches
- No arbitrary filtering or pruning
- Researchers control filtering in post-processing

### Why Rust Core?

- Performance: 10-100x faster than pure Python
- Memory: String interning reduces overhead
- Parallelism: rayon provides easy parallelization
- Safety: Type system prevents common bugs

### Why Functional API?

- Simpler mental model
- Easier to compose operations
- More Pythonic than OO pattern
- Clearer data flow

## Implementation Details

### String Interning

All strings (lemmas, forms, POS tags) are interned:

```rust
// Instead of storing strings directly
form: String,  // 24 bytes + string length

// Store indices into pool
form: lasso::Spur,  // 4 bytes
```

Benefits:
- Reduced memory (especially with repetition)
- Fast equality checks (compare indices)
- Cache-friendly

### Variable Ordering

MRV (Minimum Remaining Values) heuristic:

```
Order variables by:
1. Number of constraints (more = higher priority)
2. Domain size after constraints (smaller = higher)
```

This dramatically reduces search space by failing fast on over-constrained variables.

## Future Optimizations

Potential improvements:

1. **Indexing**: Pre-build indices on lemma/POS for faster filtering
2. **Query Optimization**: Rewrite queries for better performance
3. **Caching**: Cache compiled patterns across sessions
4. **SIMD**: Vectorize constraint checking
5. **GPU**: Parallelize within-tree search

## Source Code

For implementation details, see:

- `src/query.rs` - Query parser (Pest grammar)
- `src/pattern.rs` - Pattern AST and constraints
- `src/searcher.rs` - CSP solver implementation
- `src/tree.rs` - Tree and word data structures
- `src/iterators.rs` - Iterator interfaces
- `src/conllu.rs` - CoNLL-U parsing
- `src/python.rs` - Python bindings (PyO3)

## Next Steps

- [Performance Tips](performance.md) - Optimize your queries
- [Query Language](../guide/query-language.md) - Write better queries
