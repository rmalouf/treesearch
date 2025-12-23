//! Example demonstrating constrained vs unconstrained dependency relations
//!
//! Run with: cargo run --example unconstrained_deprel

//use treesearch::{Tree, Word, compile_query, search};

fn main() {}

/*
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

    let pattern1 = compile_query(query1).expect("Failed to parse query 1");
    println!(
        "Parsed: {} variables, {} edges, edge label: {:?}\n",
        pattern1.vars.len(),
        pattern1.edge_constraints.len(),
        pattern1.edge_constraints.first().unwrap().label
    );

    // Example 2: Unconstrained deprel (any relation)
    println!("2. UNCONSTRAINED DEPREL (any relation)");
    let query2 = r#"
        Verb [pos="VERB"];
        Noun [pos="NOUN"];
        Verb -> Noun;
    "#;
    println!("Query: {}", query2.trim());

    let pattern2 = compile_query(query2).expect("Failed to parse query 2");
    println!(
        "Parsed: {} variables, {} edges, edge label: {:?}\n",
        pattern2.vars.len(),
        pattern2.edge_constraints.len(),
        pattern2.edge_constraints.first().unwrap().label
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

    let pattern3 = compile_query(query3).expect("Failed to parse query 3");
    println!(
        "Parsed: {} variables, {} edges",
        pattern3.vars.len(),
        pattern3.edge_constraints.len()
    );
    println!("  Edge 1 label: {:?}", pattern3.edge_constraints[0].label);
    println!("  Edge 2 label: {:?}\n", pattern3.edge_constraints[1].label);

    // Test on a simple tree
    println!("4. TESTING ON SAMPLE TREE");
    let mut tree = Tree::new();
    tree.add_word(Word::new(0, "runs", "run", "VERB", "root"));
    tree.add_word(Word::new(1, "dog", "dog", "NOUN", "nsubj"));
    tree.set_parent(1, 0);

    println!("Tree: runs (VERB) -> dog (NOUN, nsubj)\n");

    // Try the unconstrained query
    println!("Match result for unconstrained query:");
    let matches: Vec<_> = search(&tree, pattern2).collect();
    if !matches.is_empty() {
        println!("  SUCCESS! Found {} match(es):", matches.len());
        for (i, m) in matches.iter().enumerate() {
            println!("  Match {}:", i + 1);
            // m is a Vec<WordId> ordered by variable declaration
            // Variable 0 = Verb, Variable 1 = Noun
            let verb = tree.get_word(m[0]).unwrap();
            let noun = tree.get_word(m[1]).unwrap();
            println!(
                "    Verb = {} (lemma: {}, pos: {})",
                verb.form, verb.lemma, verb.pos
            );
            println!(
                "    Noun = {} (lemma: {}, pos: {})",
                noun.form, noun.lemma, noun.pos
            );
        }
    } else {
        println!("  No match found");
    }
}
*/
