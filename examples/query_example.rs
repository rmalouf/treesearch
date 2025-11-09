//! Example demonstrating query parsing and pattern matching
//!
//! Run with: cargo run --example query_example

use treesearch::compiler::compile_pattern;
use treesearch::vm::VM;
use treesearch::{parse_query, Node, Tree};

fn main() {
    // Create a test tree: "I help to write code"
    // Structure:
    //   help (VERB)
    //     ├─ I (PRON, nsubj)
    //     └─ to (PART, xcomp)
    //          └─ write (VERB, obj)
    //               └─ code (NOUN, obj)

    let mut tree = Tree::new();
    tree.add_node(Node::new(0, "help", "help", "VERB", "root"));
    tree.add_node(Node::new(1, "I", "I", "PRON", "nsubj"));
    tree.add_node(Node::new(2, "to", "to", "PART", "xcomp"));
    tree.add_node(Node::new(3, "write", "write", "VERB", "mark"));
    tree.add_node(Node::new(4, "code", "code", "NOUN", "obj"));

    tree.set_parent(1, 0).unwrap(); // I -> help
    tree.set_parent(2, 0).unwrap(); // to -> help
    tree.set_parent(3, 2).unwrap(); // write -> to
    tree.set_parent(4, 3).unwrap(); // code -> write

    // Parse a query instead of manually building a Pattern!
    let query = r#"
        Help [lemma="help"];
        To [lemma="to"];
        YHead [];

        Help -[xcomp]-> To;
        To -[mark]-> YHead;
    "#;

    println!("Query:");
    println!("{}", query);
    println!();

    // Parse the query into a Pattern
    let pattern = parse_query(query).expect("Failed to parse query");
    println!(
        "Parsed pattern with {} nodes and {} edges",
        pattern.elements.len(),
        pattern.edges.len()
    );

    // Compile the pattern to opcodes (also returns var_names)
    let (opcodes, anchor, var_names) = compile_pattern(pattern);
    println!(
        "Compiled to {} instructions, anchor at element {}",
        opcodes.len(),
        anchor
    );
    println!();

    // Execute the pattern on the tree
    let vm = VM::new(opcodes, var_names);
    let anchor_node = 0; // Start at "help"

    match vm.execute(&tree, anchor_node) {
        Some(result) => {
            println!("Match found!");
            // Use the new iter_named() method to get variable names automatically
            for (var_name, node_id) in result.iter_named() {
                let node = tree.get_node(node_id).unwrap();
                println!("  {} = {} (lemma: {})", var_name, node.form, node.lemma);
            }
        }
        None => {
            println!("No match found");
        }
    }
}
