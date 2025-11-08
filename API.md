# Treesearch API Reference

**Quick reference for querying dependency treebanks**

## Overview

Treesearch provides a simple API for searching linguistic dependency trees using a pattern-matching query language. The typical workflow is:

1. Load CoNLL-U data with `CoNLLUReader`
2. Create a `TreeSearcher`
3. Search trees with query strings
4. Access matched nodes by variable name

## Basic Usage

```rust
use treesearch::{CoNLLUReader, TreeSearcher};

// 1. Load your treebank
let reader = CoNLLUReader::from_file("corpus.conll")?;

// 2. Create a searcher (reusable across trees)
let searcher = TreeSearcher::new();

// 3. Define your query
let query = r#"
    Verb [pos="VERB"];
    Noun [pos="NOUN"];
    Verb -[nsubj]-> Noun;
"#;

// 4. Search each tree
for tree in reader {
    let tree = tree?;
    let matches = searcher.search_query(&tree, query)?;

    // 5. Access results by variable name
    for result in matches {
        let verb_id = result.get("Verb").unwrap();
        let noun_id = result.get("Noun").unwrap();

        let verb = tree.get_node(verb_id).unwrap();
        let noun = tree.get_node(noun_id).unwrap();

        println!("{} ← {}", verb.lemma, noun.lemma);
    }
}
```

## Query Language

### Pattern Elements

Define nodes with constraints:

```
VariableName [constraint];
```

**Available constraints:**
- `pos="VERB"` - Part-of-speech tag
- `lemma="run"` - Lemma
- `form="running"` - Word form
- `deprel="nsubj"` - Dependency relation (to parent)

**Multiple constraints** (AND):
```
V [pos="VERB", lemma="be"];
```

**Empty constraint** (matches any node):
```
AnyNode [];
```

### Pattern Edges

Define relationships between nodes:

```
Parent -[deprel]-> Child;
```

**Dependency types:**
- `-[nsubj]->` - Specific dependency relation
- `->` - Any child (no relation specified)

**Example patterns:**

```
// VERB with nominal subject
V [pos="VERB"];
N [pos="NOUN"];
V -[nsubj]-> N;

// Verb with xcomp (control verb)
Main [pos="VERB"];
Comp [pos="VERB"];
Main -[xcomp]-> Comp;

// Complex: VERB → NOUN → ADJ
V [pos="VERB"];
N [pos="NOUN"];
A [pos="ADJ"];
V -[obj]-> N;
N -[amod]-> A;
```

## API Reference

### CoNLLUReader

Parse CoNLL-U formatted treebanks.

```rust
// From file
let reader = CoNLLUReader::from_file("path/to/corpus.conll")?;

// From string
let reader = CoNLLUReader::from_string(conllu_text);

// Iterate over trees
for tree in reader {
    let tree = tree?; // Result<Tree, ParseError>
    // ...
}
```

### TreeSearcher

Execute queries on dependency trees.

```rust
let searcher = TreeSearcher::new();

// Search with query string
let matches = searcher.search_query(&tree, query_string)?;

// Iterate over all matches
for result in matches {
    // Access bindings...
}
```

### Match

Results of pattern matching with variable bindings.

```rust
// Access by variable name
let node_id = result.get("VariableName")?;

// Iterate over all bindings with names
for (var_name, node_id) in result.iter_named() {
    println!("{} = node {}", var_name, node_id);
}

// Direct access to bindings (position-based)
let node_id = result.bindings[&0]; // First variable
```

### Tree

Dependency tree with CoNLL-U annotations.

```rust
// Get node by ID
let node = tree.get_node(node_id)?;

// Access node properties
println!("Form: {}", node.form);        // Word form
println!("Lemma: {}", node.lemma);      // Lemma
println!("POS: {}", node.pos);          // UPOS tag
println!("DepRel: {}", node.deprel);    // Dependency relation

// Optional fields
if let Some(xpos) = &node.xpos {
    println!("XPOS: {}", xpos);
}

// Features (morphological)
if let Some(case) = node.feats.get("Case") {
    println!("Case: {}", case);
}

// Tree metadata
if let Some(text) = &tree.sentence_text {
    println!("Sentence: {}", text);
}
```

## Complete Example

```rust
use treesearch::{CoNLLUReader, TreeSearcher};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load corpus
    let reader = CoNLLUReader::from_file("./corpus.conll")?;
    let searcher = TreeSearcher::new();

    // Find all control verbs (VERB with VERB xcomp)
    let query = r#"
        Main [pos="VERB"];
        Comp [pos="VERB"];
        Main -[xcomp]-> Comp;
    "#;

    // Search and display results
    for tree in reader {
        let tree = tree?;
        let matches = searcher.search_query(&tree, query)?;

        for result in matches {
            // Use iter_named() for clean iteration
            for (var_name, node_id) in result.iter_named() {
                let node = tree.get_node(node_id).unwrap();
                println!("  {} = {} (lemma: {})",
                    var_name, node.form, node.lemma);
            }
            println!();
        }
    }

    Ok(())
}
```

## Error Handling

All user-facing operations return `Result` types:

```rust
// Parse errors
match CoNLLUReader::from_file("corpus.conll") {
    Ok(reader) => { /* ... */ }
    Err(e) => eprintln!("Failed to open file: {}", e),
}

// Query syntax errors
match searcher.search_query(&tree, query) {
    Ok(matches) => { /* ... */ }
    Err(e) => eprintln!("Invalid query: {}", e),
}

// Tree iteration errors
for tree_result in reader {
    match tree_result {
        Ok(tree) => { /* ... */ }
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
```

## Performance Notes

- **TreeSearcher is reusable**: Create once, use for all trees
- **Index-based filtering**: Candidates are pre-filtered before VM execution
- **Leftmost matching**: Returns first match in left-to-right word order
- **Parallel processing**: Use `rayon` to process multiple trees concurrently

```rust
use rayon::prelude::*;

let trees: Vec<Tree> = reader.collect::<Result<_, _>>()?;
let searcher = TreeSearcher::new();

// Process trees in parallel
trees.par_iter().for_each(|tree| {
    if let Ok(matches) = searcher.search_query(tree, query) {
        // Process matches...
    }
});
```
