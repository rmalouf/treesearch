//! Example demonstrating query parsing and pattern matching
//!
//! Run with: cargo run --example query_example

use treesearch::{CoNLLUReader, TreeSearcher};

fn main() {
    let reader =
        CoNLLUReader::from_file("./examples/lw970831.conll".as_ref()).expect("Can't open file");

    let query = r#"
        Verb [pos="VERB", lemma="help"];
        Xcomp [];
        Verb -[xcomp]-> Xcomp;
    "#;
    let searcher = TreeSearcher::new();

    for tree in reader {
        let tree = tree.expect("Reader error");
        let matches = searcher.search_query(&tree, query).expect("Search error");
        for result in matches {
            let verb_node = tree.get_node(result.get("Verb").unwrap()).unwrap();
            let xcomp_node = tree.get_node(result.get("Xcomp").unwrap()).unwrap();
            let to_node = xcomp_node.children_by_deprel(&tree, "aux");
            match to_node.first() {
                Some(to_node) => println!(
                    "{} {} {}",
                    verb_node.form, to_node.form, xcomp_node.form,
                ),
                None => println!(
                    "{} {}",
                    verb_node.form, xcomp_node.form
                )
            }
        }
    }
}
