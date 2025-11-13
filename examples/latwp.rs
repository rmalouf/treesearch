//! Example demonstrating query parsing and pattern matching on CoNLL-U files
//!
//! Run with: cargo run --example latwp

use treesearch::{CoNLLUReader, search_query};

fn main() {

    let mut count = 0;

    let query = r#"
            N1 [pos="NOUN"];
            Of [form="of"];
            N2 [pos="NOUN"];
            N1 -> Of;
            Of -> N2;
        "#;

    for _ in 0..100 {
        let reader =
            CoNLLUReader::from_file("./examples/lw970831.conll".as_ref()).expect("Can't open file");

        for tree in reader {
            let tree = tree.expect("Reader error");
            let matches = search_query(&tree, query).expect("Search error");
            for _result in matches {
                // result is a Vec<WordId> ordered by variable declaration
                // Variable 0 = Noun, Variable 1 = Adj
                //let noun = tree.get_word(result[0]).unwrap();
                //let adj = tree.get_word(result[2]).unwrap();
                count += 1;
                //println!("{} of {}", noun.form, adj.form)
            }
        }
    }
    println!("{}", count);

}
