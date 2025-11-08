# Dependency Tree Pattern Matching VM Design

## Overview

A virtual machine-based pattern matcher for linguistic queries over dependency treebanks. Designed to handle complex patterns (including unbounded wildcards) efficiently at scale (3+ billion nodes) while avoiding pathological backtracking.

## Core Design Decisions

### Match Semantics
- **Leftmost**: When multiple matches are possible, select the leftmost node in linear order
- **Shortest path**: For wildcard patterns, select the match with minimum path distance
- **First match**: Return immediately upon finding a valid match (no exhaustive search)

These semantics allow aggressive pruning and early termination, avoiding exponential search spaces.

## Architecture

### Two-Phase Search Strategy

1. **Index Phase**: Use inverted indices to identify candidate nodes
2. **Verification Phase**: Run VM only on viable candidates

```rust
// Pseudocode structure
struct TreebankSearcher {
    index: TreebankIndex,
    vm: PatternVM,
}

impl TreebankSearcher {
    fn search(&self, pattern: &Pattern) -> impl Iterator<Match> {
        // Compile pattern to bytecode
        let (bytecode, anchor_pos) = compile_pattern(pattern);
        
        // Get candidates using most selective constraint
        let candidates = self.index.get_candidates(pattern, anchor_pos);
        
        // Verify each candidate
        candidates
            .filter_map(|node| self.vm.execute(bytecode, node))
            .take(1)  // First match only
    }
}
```

### Indexing Strategy

Build inverted indices at treebank load time:

```rust
struct TreebankIndex {
    by_lemma: HashMap<String, Vec<NodeRef>>,
    by_pos: HashMap<String, Vec<NodeRef>>,
    by_deprel: HashMap<String, Vec<NodeRef>>,
    
    // Pre-index common patterns
    verb_with_rel_descendant: HashSet<NodeRef>,
    noun_with_det: HashSet<NodeRef>,
    // ... other common patterns
}
```

## Virtual Machine Design

### Instruction Set

```rust
enum Instruction {
    // Constraint checking
    CheckPOS(String),
    CheckLemma(String),
    CheckDeprel(String),
    
    // Navigation
    MoveToParent,
    MoveToChild(Option<Constraint>),
    MoveLeft,
    MoveRight,
    
    // Wildcard search (with early termination)
    ScanDescendants(Constraint),  // BFS for shortest path
    ScanAncestors(Constraint),
    ScanRightward(Constraint),
    
    // Control flow
    Bind(usize),        // Bind current node to pattern position
    Choice,             // Create backtrack point
    Commit,             // Discard backtrack points (cut)
    Match,              // Success
    Fail,               // Trigger backtracking
    
    // State management  
    PushState,          // For interleaved search
    RestoreState,
    Jump(isize),
}
```

### VM State

```rust
struct VMState {
    current: NodeRef,
    bindings: HashMap<usize, NodeRef>,
    ip: usize,  // Instruction pointer
    backtrack_stack: Vec<ChoicePoint>,
    
    // For interleaved bidirectional search
    cursors: HashMap<usize, NodeRef>,
    
    // Memoization to avoid redundant work
    memo: HashMap<(NodeId, usize, Direction), Option<Bindings>>,
}

struct ChoicePoint {
    ip: usize,
    node: NodeRef,
    bindings: HashMap<usize, NodeRef>,
    alternatives: Vec<NodeRef>,  // Remaining options
}
```

## Pattern Compilation Strategy

### Anchor Selection

Choose the most selective pattern element as the anchor point to minimize candidates:

```rust
fn select_anchor(pattern: &Pattern, stats: &CorpusStats) -> usize {
    pattern
        .elements
        .iter()
        .enumerate()
        .min_by_key(|(_, elem)| {
            // Estimate selectivity (lower is better)
            match elem {
                Elem::Lemma(l) => stats.lemma_freq(l),
                Elem::POS(p) => stats.pos_freq(p),
                Elem::Wildcard => usize::MAX,  // Never anchor on wildcard
            }
        })
        .map(|(idx, _)| idx)
        .unwrap()
}
```

### Interleaved Verification

Compile patterns to verify constraints by alternating between backward and forward directions from the anchor. This enables early failure detection:

```rust
fn compile_interleaved(pattern: &Pattern, anchor: usize) -> Vec<Instruction> {
    let mut instructions = vec![];
    
    // Verify anchor
    instructions.extend(compile_constraints(&pattern[anchor]));
    instructions.push(Instruction::Bind(anchor));
    
    let mut back_idx = anchor.saturating_sub(1);
    let mut forward_idx = anchor + 1;
    
    loop {
        let mut made_progress = false;
        
        // One step backward
        if back_idx < anchor {
            instructions.push(Instruction::PushState);
            instructions.extend(compile_backward_step(pattern, back_idx));
            instructions.push(Instruction::Bind(back_idx));
            back_idx = back_idx.saturating_sub(1);
            made_progress = true;
        }
        
        // One step forward
        if forward_idx < pattern.len() {
            instructions.push(Instruction::RestoreState);
            instructions.extend(compile_forward_step(pattern, forward_idx));
            instructions.push(Instruction::Bind(forward_idx));
            forward_idx += 1;
            made_progress = true;
        }
        
        if !made_progress { break; }
    }
    
    instructions.push(Instruction::Match);
    instructions
}
```

## Wildcard Handling

### Bounded Search with BFS

For patterns with wildcards (`...`), use breadth-first search to guarantee shortest path:

```rust
fn scan_descendants(node: NodeRef, constraint: &Constraint) -> Option<NodeRef> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    
    queue.push_back((node, 0));
    visited.insert(node.id());
    
    while let Some((current, depth)) = queue.pop_front() {
        // Check children first (BFS order = shortest path)
        for child in current.children() {
            if !visited.insert(child.id()) { continue; }
            
            if constraint.matches(child) {
                return Some(child);  // First match = shortest path
            }
            
            queue.push_back((child, depth + 1));
        }
    }
    
    None
}
```

### Controlled Backtracking

Only create choice points where necessary (multiple valid paths), ordered by preference:

```rust
impl VMExecutor {
    fn handle_scan_descendants(&mut self, constraint: Constraint) -> Result<(), VMError> {
        // Find ALL matches (for potential backtracking)
        let matches = find_all_descendants(self.state.current, &constraint);
        
        if matches.is_empty() {
            return self.backtrack();
        }
        
        // Sort by preference (leftmost, then shortest)
        let mut matches = matches;
        matches.sort_by_key(|n| (n.position(), n.depth()));
        
        // Take first, save rest as alternatives if needed
        let (first, rest) = matches.split_first().unwrap();
        
        if !rest.is_empty() {
            self.state.backtrack_stack.push(ChoicePoint {
                ip: self.state.ip + 1,
                node: self.state.current,
                bindings: self.state.bindings.clone(),
                alternatives: rest.to_vec(),
            });
        }
        
        self.state.current = *first;
        Ok(())
    }
}
```

## Optimization Techniques

### 1. Memoization
Cache pattern matching results to avoid redundant verification:
```rust
type MemoKey = (NodeId, PatternPosition, SearchDirection);
type MemoTable = HashMap<MemoKey, Option<Bindings>>;
```

### 2. Common Pattern Pre-indexing
Pre-compute frequently used patterns during index building:
- Verbs with relative clause descendants
- Nouns preceded by determiners  
- Auxiliary + past participle sequences

### 3. Instruction Ordering
Order constraint checks by selectivity (most likely to fail first)

### 4. Early Termination
With "first match" semantics, stop as soon as any valid match is found

### 5. Depth Limits for Wildcards
Set configurable maximum depth for wildcard searches (e.g., 5-7 edges)

## Example: Complex Pattern Compilation

Pattern: `VERB > NOUN ... REL` (verb with noun child that dominates a relative)

```rust
// Assuming REL is rarest, anchor there
let bytecode = vec![
    // Verify REL
    Instruction::CheckPOS("REL".into()),
    Instruction::Bind(2),
    
    // Find NOUN ancestor (shortest path)
    Instruction::ScanAncestors(Constraint::POS("NOUN")),
    Instruction::Bind(1),
    
    // Verify NOUN has VERB parent
    Instruction::MoveToParent,
    Instruction::CheckPOS("VERB".into()),
    Instruction::Bind(0),
    
    Instruction::Match,
];
```

## Performance Characteristics

- **Index lookup**: O(1) average case
- **Single pattern verification**: O(n) where n = sentence length (typically < 50)
- **Wildcard search**: O(n) with BFS (bounded by sentence size)
- **Full treebank search**: O(k × n) where k = number of candidates from index
- **Memory**: O(nodes × features) for indices, constant per pattern execution

## Implementation Priorities

1. **Core VM executor** with basic instructions
2. **Pattern compiler** with anchor selection
3. **Inverted index** builder and query interface
4. **Wildcard handling** with BFS and depth limits
5. **Backtracking** for complex patterns
6. **Optimization** passes (memoization, instruction reordering)
7. **Pattern syntax parser** (can prototype with s-expressions initially)

## Open Questions / Future Work

1. **JIT compilation** for frequently used patterns?
2. **SIMD instructions** for parallel constraint checking?
3. **Incremental indexing** for dynamic treebanks?
4. **Query plan optimization** based on corpus statistics?
5. **Pattern caching** strategies for common subpatterns?
6. **Parallel execution** across treebank shards?

## References

- CQP (Corpus Query Processor) for corpus indexing strategies
- Tgrep2/SETS for tree pattern matching approaches  
- Stanford Semgrex for dependency pattern compilation
- PML-TQ for complex linguistic query optimization
