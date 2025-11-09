//! Example demonstrating the children_by_deprel API
//!
//! Run with: cargo run --example dependent_lookup

use treesearch::{Node, Tree};

fn main() {
    println!("=== Dependent Lookup API Demo ===\n");

    // Create a sample sentence tree: "The dog quickly chased the cat and the mouse"
    // Structure:
    //   chased (VERB, root)
    //     ├─ dog (NOUN, nsubj)
    //     │   └─ The (DET, det)
    //     ├─ quickly (ADV, advmod)
    //     ├─ cat (NOUN, obj)
    //     │   └─ the (DET, det)
    //     └─ and (CCONJ, cc)
    //          └─ mouse (NOUN, conj)
    //              └─ the (DET, det)

    let mut tree = Tree::new();

    // Add nodes
    tree.add_node(Node::new(0, "chased", "chase", "VERB", "root"));
    tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
    tree.add_node(Node::new(2, "The", "the", "DET", "det"));
    tree.add_node(Node::new(3, "quickly", "quickly", "ADV", "advmod"));
    tree.add_node(Node::new(4, "cat", "cat", "NOUN", "obj"));
    tree.add_node(Node::new(5, "the", "the", "DET", "det"));
    tree.add_node(Node::new(6, "and", "and", "CCONJ", "cc"));
    tree.add_node(Node::new(7, "mouse", "mouse", "NOUN", "conj"));
    tree.add_node(Node::new(8, "the", "the", "DET", "det"));

    // Set up tree structure
    tree.set_parent(1, 0); // dog -> chased
    tree.set_parent(2, 1); // The -> dog
    tree.set_parent(3, 0); // quickly -> chased
    tree.set_parent(4, 0); // cat -> chased
    tree.set_parent(5, 4); // the -> cat
    tree.set_parent(6, 0); // and -> chased
    tree.set_parent(7, 4); // mouse -> cat (coordination)
    tree.set_parent(8, 7); // the -> mouse

    println!("Sentence: The dog quickly chased the cat and the mouse\n");

    // Example 1: Get a single dependent by deprel (using first element)
    println!("1. SINGLE DEPENDENT LOOKUP (using .first())");
    let verb = tree.get_node(0).unwrap();

    if let Some(subject) = verb.children_by_deprel(&tree, "nsubj").first() {
        println!("   Subject of '{}': {}", verb.form, subject.form);
    }

    if let Some(object) = verb.children_by_deprel(&tree, "obj").first() {
        println!("   Object of '{}': {}", verb.form, object.form);
    }

    if let Some(adverb) = verb.children_by_deprel(&tree, "advmod").first() {
        println!("   Adverb modifying '{}': {}", verb.form, adverb.form);
    }

    // Non-existent dependent
    if verb.children_by_deprel(&tree, "ccomp").is_empty() {
        println!("   No clausal complement found");
    }
    println!();

    // Example 2: Get all dependents with a specific relation
    println!("2. MULTIPLE DEPENDENTS (children_by_deprel)");
    let cat = tree.get_node(4).unwrap();

    println!("   Determiners of '{}':", cat.form);
    let dets = cat.children_by_deprel(&tree, "det");
    for det in &dets {
        println!("     - {}", det.form);
    }

    println!("   Conjuncts coordinated with '{}':", cat.form);
    let conjs = cat.children_by_deprel(&tree, "conj");
    for conj in &conjs {
        println!("     - {}", conj.form);
    }
    println!();

    // Example 3: Chaining lookups
    println!("3. CHAINED LOOKUPS");
    if let Some(subj) = verb.children_by_deprel(&tree, "nsubj").first() {
        if let Some(det) = subj.children_by_deprel(&tree, "det").first() {
            println!(
                "   The determiner of the subject of '{}' is '{}'",
                verb.form, det.form
            );
        }
    }
    println!();

    // Example 4: Comparing old vs new API
    println!("4. OLD vs NEW API");
    println!("   Old API (manual filtering):");
    let children = verb.children(&tree);
    if let Some(obj) = children.iter().find(|c| c.deprel == "obj") {
        println!("     Object found: {}", obj.form);
    }

    println!("   New API (using children_by_deprel):");
    if let Some(obj) = verb.children_by_deprel(&tree, "obj").first() {
        println!("     Object found: {}", obj.form);
    }
}
