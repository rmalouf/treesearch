# CoNLL-U Parsing Optimization Plan

## Context

Current profiling shows `parse_tree` consumes most execution time with minimal I/O wait. The bottleneck is CPU-bound processing, primarily in the parsing hot path. This plan focuses on optimizations that work with parallel execution (rayon).

## Critical Bottlenecks Identified

### 1. **Per-Line Vec Allocation** (25-30% speedup potential)

**Current Code** (`src/conllu.rs:274`):
```rust
loop {
    let mut buffer: Vec<u8> = Vec::with_capacity(100);  // ⚠️ ALLOCATES EVERY LINE
    match self.reader.read_until(b'\n', &mut buffer) {
```

**Problem**: Allocates heap memory for every line in every file.

**Fix**: Reuse single buffer per iterator:
```rust
let mut buffer = Vec::with_capacity(100);
loop {
    buffer.clear();  // Reuse allocation
    match self.reader.read_until(b'\n', &mut buffer) {
```

**Parallel safety**: ✅ Each iterator owns its buffer - no contention.

---

### 2. **Redundant Newline Scanning** (5-10% speedup potential)

**Current Code** (`src/conllu.rs:285`):
```rust
let line = bs_trim(&buffer);  // Scans entire buffer for '\n'
```

**Problem**: `read_until(b'\n', ...)` already includes `\n` at end. `bs_trim` then scans the entire buffer byte-by-byte to find it again.

**Fix**: O(1) suffix check instead of O(n) scan:
```rust
let line = buffer.strip_suffix(&[b'\n']).unwrap_or(&buffer);
```

**Parallel safety**: ✅ No shared state.

---

### 3. **Double Iteration: Store Then Parse** (20-30% speedup potential)

**Current Code** (`src/conllu.rs:301, 313`):
```rust
// First pass: collect all lines
tree_lines.push((self.line_num, line.to_owned()));  // Clone + allocate

// Second pass: parse collected lines
Some(self.parse_tree(&tree_lines, sentence_text, metadata))
```

**Problem**:
- Allocates `Vec<(usize, Vec<u8>)>` for tree_lines
- Clones every line's bytes
- Iterates over data twice
- Poor cache locality

**Fix**: Parse immediately during iteration:
```rust
if !line.is_empty() && line[0] != b'#' {
    self.parse_line(&mut tree, line, word_id)?;  // Parse immediately
    word_id += 1;
}
```

**Benefits**:
- Single pass over data
- No intermediate storage
- Better cache locality
- Eliminates cloning

**Trade-off**: Error reporting slightly more complex (can't store all line content for errors). Solution: Keep last buffer for error context.

**Parallel safety**: ✅ Each iterator parses independently.

---

### 4. **String Pool Mutex Overhead** (10-20% speedup potential)

**Current Code** (`src/bytes.rs:27-29`):
```rust
pub fn get_or_intern(&mut self, bytes: &[u8]) -> Sym {
    self.0.lock().unwrap().get_or_intern(bytes)  // ⚠️ MUTEX LOCK
}
```

Called **4-6 times per word**: form, lemma, upos, xpos, deprel, plus 2× per feature.

**Problem for Parallelism**:
- With parallel iterators, this becomes a major contention point
- All threads compete for the same mutex
- Cache line bouncing between cores

**Solutions**:

#### Option A: Thread-Local String Pools (RECOMMENDED)
Each iterator gets its own pool, merged at end if needed:
```rust
pub struct TreeIterator<R: BufRead> {
    reader: R,
    line_num: usize,
    string_pool: BytestringPool,  // Thread-local, no mutex needed
}
```

**Pros**:
- Zero contention
- No synchronization overhead
- Symbols are tree-local (fine for most use cases)

**Cons**:
- Can't compare symbols across trees directly
- Slightly higher memory usage (duplicate strings across pools)

**For most use cases**: Queries run on one tree at a time, so cross-tree symbol comparison isn't needed.

#### Option B: Sharded String Pool
Partition pool by hash to reduce contention:
```rust
const SHARD_COUNT: usize = 16;
pub struct ShardedPool {
    shards: [Mutex<ByteInterner>; SHARD_COUNT],
}
```

**Pros**:
- Global symbol space
- Reduced contention (16× fewer conflicts)

**Cons**:
- Still has synchronization overhead
- More complex implementation

#### Option C: Lock-Free String Interning
Use atomic operations and lock-free data structures:
```rust
use dashmap::DashMap;  // Lock-free concurrent HashMap
```

**Pros**:
- No lock contention
- Global symbol space

**Cons**:
- Adds dependency
- Still has atomic CAS overhead

**Recommendation**: Option A (thread-local pools) for initial implementation. Most queries don't need cross-tree symbol comparison.

---

### 5. **Field Splitting Iterator Overhead** (5-15% speedup potential)

**Current Code** (`src/conllu.rs:98`):
```rust
let mut fields = line.split(|b| *b == b'\t');  // Creates iterator
```

**Problem**:
- Split iterator has overhead (state machine, checks)
- Closure called for every byte: `|b| *b == b'\t'`
- Can't leverage SIMD

**Fix Option A**: Manual indexing with memchr (SIMD-accelerated):
```rust
use memchr::memchr_iter;

let mut tabs: Vec<usize> = memchr_iter(b'\t', line).collect();
tabs.push(line.len());  // Sentinel

let token_id = parse_id(&line[0..tabs[0]])?;
let form = &line[tabs[0]+1..tabs[1]];
let lemma = &line[tabs[1]+1..tabs[2]];
// ... etc
```

**Fix Option B**: Manual scan (simpler, no dependency):
```rust
let mut fields = [&b""[..]; 10];
let mut field_idx = 0;
let mut start = 0;

for i in 0..line.len() {
    if line[i] == b'\t' {
        fields[field_idx] = &line[start..i];
        field_idx += 1;
        start = i + 1;
    }
}
fields[field_idx] = &line[start..];  // Last field

// Now fields[0] = token_id, fields[1] = form, etc.
```

**Recommendation**: Try Option A first (memchr), fallback to Option B if dependency is unwanted.

**Parallel safety**: ✅ No shared state.

---

## Medium Impact Optimizations

### 6. **Missing Inline Hints** (5-10% speedup)

Add `#[inline]` to hot path functions:
- `parse_id` (`conllu.rs:340`)
- `parse_head` (`conllu.rs:373`)
- `parse_features` (`conllu.rs:156`)
- `bs_split_once` (`bytes.rs:96`)
- `bs_atoi` (`bytes.rs:113`)
- `bs_trim` (`bytes.rs:104`)

**Why**: These are called in tight loops but may not be inlined automatically due to size/complexity heuristics.

---

### 7. **Features Vec Pre-allocation** (1-3% speedup)

**Current Code** (`conllu.rs:161`):
```rust
let mut feats = Features::new();  // Capacity = 0
```

**Problem**: Most words have 0-5 features. Vec starts at 0, reallocates on first push.

**Fix**:
```rust
let mut feats = Features::with_capacity(4);  // Typical feature count
```

---

### 8. **Hash Function Optimization for Small Strings** (2-5% speedup)

**Current Code** (`bytes.rs:68-70`):
```rust
let mut h = FxHasher::default();
bytes.hash(&mut h);
let hash = h.finish();
```

**Observation**: Many linguistic strings are short (2-8 bytes): "the", "NOUN", "nsubj", etc.

**Optimization**: Fast path for small strings:
```rust
let hash = if bytes.len() <= 8 {
    // For strings ≤8 bytes, interpret bytes directly as u64 (padded)
    let mut arr = [0u8; 8];
    arr[..bytes.len()].copy_from_slice(bytes);
    u64::from_ne_bytes(arr)
} else {
    let mut h = FxHasher::default();
    bytes.hash(&mut h);
    h.finish()
};
```

**Trade-off**: Adds branch, but eliminates hashing for common case.

---

## Low Impact / Future Optimizations

### 9. **SIMD Newline/Tab Scanning**
Use `memchr` crate for SIMD-accelerated byte finding. Already suggested in #5.

**Dependency**: `memchr = "2.7"`

---

### 10. **Reduce compile_tree Overhead**
**Current**: Builds parent-child relationships after parsing (`tree.rs:192-200`).

**Alternative**: Build relationships during parsing:
```rust
// In parse_line:
if let Some(head) = head {
    tree.words[head].children.push(word_id);  // Add child immediately
}
```

**Trade-off**: Requires words vector to be pre-sized or use unsafe indexing.

---

### 11. **Arena Allocation for Strings**
Instead of `Arc<[u8]>` per string, use bump allocator:
```rust
use bumpalo::Bump;

pub struct ByteInterner {
    arena: Bump,
    map: HashMap<&[u8], Sym>,  // Borrows from arena
}
```

**Benefits**: Better cache locality, fewer allocations.

**Complexity**: Higher implementation complexity.

---

## Implementation Strategy

### Phase 1: Critical Path (Expected 2-3× speedup)
1. ✅ Reuse line buffer (#1)
2. ✅ Fix bs_trim redundancy (#2)
3. ✅ Parse immediately instead of store-then-parse (#3)
4. ✅ Add inline hints (#6)

**Goal**: Reduce parsing overhead by 50-66%.

### Phase 2: Parallelism Preparation (Expected 1.5-2× additional speedup)
5. ✅ Thread-local string pools (#4 Option A)
6. ✅ Efficient field splitting with memchr (#5)
7. ✅ Pre-allocate Features vec (#7)

**Goal**: Enable efficient parallel processing without contention.

### Phase 3: Fine-tuning (Expected 10-20% additional speedup)
8. ⏳ Hash optimization for small strings (#8)
9. ⏳ Inline compile_tree into parsing (#10)

**Goal**: Squeeze out remaining performance.

---

## Benchmarking Strategy

Create benchmarks for:
1. **Single file parsing**: Measure raw parsing speed
2. **Multi-file sequential**: Baseline for parallel comparison
3. **Multi-file parallel**: Measure scaling with rayon

**Files to use**:
- Small: 1k sentences
- Medium: 100k sentences
- Large: 1M+ sentences

**Metrics**:
- Sentences/second
- Memory usage (peak RSS)
- CPU efficiency (user time / wall time with parallel)

**Baseline before optimization**:
```bash
cargo bench --bench conllu -- --save-baseline before
```

**After each phase**:
```bash
cargo bench --bench conllu -- --baseline before
```

---

## Parallel Execution Considerations

### String Pool Strategy
With thread-local pools (Option A from #4):
- Each rayon thread gets its own iterator with own pool
- No contention between threads
- Trees are independent (fine for most queries)
- If cross-tree queries needed, implement global symbol resolution post-processing

### Memory Usage
- Each thread allocates its own string pool (~5000 string capacity)
- For 8 threads: ~8× string storage for duplicates
- Trade-off: Speed vs memory (acceptable for large corpus use case)

### Work Distribution
Rayon's `par_bridge()` or manual chunking:
```rust
use rayon::prelude::*;

file_paths.par_iter()
    .flat_map(|path| TreeIterator::from_file(path).ok())
    .filter_map(Result::ok)  // Each iterator on separate thread
    .for_each(|tree| { /* process */ });
```

---

## Success Criteria

- [ ] Parsing throughput: >100k sentences/sec single-threaded (Medium corpus)
- [ ] Parallel scaling: >70% efficiency on 8 cores
- [ ] Memory usage: <2GB for 1M sentence corpus
- [ ] Zero regression in correctness (all tests pass)

---

## Testing Plan

1. **Correctness**: Ensure all existing tests pass after each optimization
2. **Performance**: Benchmark before/after for each phase
3. **Stress test**: Parse entire large corpus (COHA, Wikipedia dumps)
4. **Profiling**: Use `perf`, `flamegraph`, or `cargo-flamegraph` to validate optimizations

---

## References

- Profiling discussion: Performance analysis identified parse_tree as bottleneck
- CoNLL-U format: https://universaldependencies.org/format.html
- memchr crate: https://docs.rs/memchr/
- Rayon parallelism: https://docs.rs/rayon/
