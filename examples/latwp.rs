//! Example demonstrating query parsing and pattern matching
//!
//! Run with: cargo run --example query_example

use treesearch::{CoNLLUReader, TreeSearcher};

fn main() {
    let reader =
        CoNLLUReader::from_file("./examples/lw970831.conll".as_ref()).expect("Can't open file");

    let query = r#"
        Verb [pos="VERB"];
        Xcomp [pos="VERB"];
        Verb -[xcomp]-> Xcomp;
    "#;
    let searcher = TreeSearcher::new();

    for tree in reader {
        let tree = tree.expect("Reader error");
        let matches = searcher.search_query(&tree, query).expect("Search error");
        for result in matches {
            // Use the new iter_named() method to get variable names automatically
            for (var_name, node_id) in result.iter_named() {
                let node = tree.get_node(node_id).unwrap();
                println!("  {} = {} (lemma: {})", var_name, node.form, node.lemma);
            }
            println!();
        }
    }

    ()
}
