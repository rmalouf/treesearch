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
            let verb_node = tree.get_node(result.get("Verb").unwrap()).unwrap();
            let xcomp_node = tree.get_node(result.get("Xcomp").unwrap()).unwrap();
            println!(
                "{} ({}) -> {} ({})",
                verb_node.form, verb_node.position, xcomp_node.form, xcomp_node.position
            );
        }
    }
    ()
}
