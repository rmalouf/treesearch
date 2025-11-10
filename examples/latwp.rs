//! Example demonstrating query parsing and pattern matching
//!
//! Run with: cargo run --example query_example

use treesearch::{CoNLLUReader, search_query};

fn main() {
    let reader =
        CoNLLUReader::from_file("./examples/lw970831.conll".as_ref()).expect("Can't open file");

    let query = r#"
        Noun [pos="NOUN"];
        Adj [pos="ADJ"];
        Noun -[amod]-> Adj;
    "#;

    for tree in reader {
        let tree = tree.expect("Reader error");
        let matches = search_query(&tree, query).expect("Search error");
        for result in matches {
            let verb_node = tree.get_node(result.get("Noun").unwrap()).unwrap();
            let xcomp_node = tree.get_node(result.get("Adj").unwrap()).unwrap();
            println!("{} -> {}", verb_node.form, xcomp_node.form)
        }
    }
}
