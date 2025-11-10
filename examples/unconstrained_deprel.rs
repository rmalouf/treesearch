use treesearch::compiler::compile_pattern;
use treesearch::vm::VM;
use treesearch::{Node, Tree, parse_query};

fn main() {
    println!("=== Unconstrained Deprel Demo ===\n");

    // Example 1: Constrained deprel (specific relation)
    println!("1. CONSTRAINED DEPREL (specific relation)");
    let query1 = r#"
        Verb [pos="VERB"];
        Noun [pos="NOUN"];
        Verb -[nsubj]-> Noun;
    "#;
    println!("Query: {}", query1.trim());

    let pattern1 = parse_query(query1).expect("Failed to parse query 1");
    println!(
        "Parsed: {} nodes, {} edges, edge label: {:?}\n",
        pattern1.elements.len(),
        pattern1.edges.len(),
        pattern1.edges.first().unwrap().label
    );

    // Example 2: Unconstrained deprel (any relation)
    println!("2. UNCONSTRAINED DEPREL (any relation)");
    let query2 = r#"
        Verb [pos="VERB"];
        Noun [pos="NOUN"];
        Verb -> Noun;
    "#;
    println!("Query: {}", query2.trim());

    let pattern2 = parse_query(query2).expect("Failed to parse query 2");
    println!(
        "Parsed: {} nodes, {} edges, edge label: {:?}\n",
        pattern2.elements.len(),
        pattern2.edges.len(),
        pattern2.edges.first().unwrap().label
    );

    // Example 3: Mixed constrained and unconstrained
    println!("3. MIXED (both constrained and unconstrained)");
    let query3 = r#"
        Root [];
        Child1 [];
        Child2 [];
        Root -[nsubj]-> Child1;
        Root -> Child2;
    "#;
    println!("Query: {}", query3.trim());

    let pattern3 = parse_query(query3).expect("Failed to parse query 3");
    println!(
        "Parsed: {} nodes, {} edges",
        pattern3.elements.len(),
        pattern3.edges.len()
    );
    println!("  Edge 1 label: {:?}", pattern3.edges[0].label);
    println!("  Edge 2 label: {:?}\n", pattern3.edges[1].label);

    // Compile and show bytecode for unconstrained edge
    let (opcodes, anchor, var_names) = compile_pattern(pattern2.clone());
    println!("Bytecode for unconstrained deprel query:");
    println!("  Anchor: element {}", anchor);
    println!("  Variables: {:?}", var_names);
    println!("  Instructions:");
    for (i, instr) in opcodes.iter().enumerate() {
        println!("    {}: {:?}", i, instr);
    }
    println!("  Note: No CheckDepRel instruction since deprel is unconstrained!\n");

    // Test on a simple tree
    println!("4. TESTING ON SAMPLE TREE");
    let mut tree = Tree::new();
    tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
    tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
    tree.set_parent(1, 0);

    println!("Tree: runs (VERB) -> dog (NOUN, nsubj)\n");

    // Try the unconstrained query
    let vm = VM::new(opcodes, var_names);

    println!("Match result for unconstrained query:");
    match vm.execute(&tree, 0).next() {
        // Start at root (runs)
        Some(m) => {
            println!("  SUCCESS! Matched:");
            for (var_name, node_id) in m.iter_named() {
                let node = tree.get_node(node_id).unwrap();
                println!(
                    "    {} = {} (lemma: {}, pos: {})",
                    var_name, node.form, node.lemma, node.pos
                );
            }
        }
        None => {
            println!("  No match found");
        }
    }
}
