//! Example demonstrating query parsing and pattern matching on CoNLL-U files
//!
//! Run with: cargo run --example latwp

use treesearch::{search_query, CoNLLUReader};

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
            // result is a Vec<WordId> ordered by variable declaration
            // Variable 0 = Noun, Variable 1 = Adj
            let noun = tree.get_word(result[0]).unwrap();
            let adj = tree.get_word(result[1]).unwrap();
            println!("{} -> {}", noun.form, adj.form)
        }
    }
}
