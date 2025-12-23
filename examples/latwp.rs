//! Example demonstrating query parsing and pattern matching on CoNLL-U files
//!
//! Run with: cargo run --example latwp --release

use treesearch::{Treebank, compile_query};

fn main() {
    let query = r#"
            N1 [pos="NOUN"];
            Of [form="of"];
            N2 [pos="NOUN"];
            N1 -> Of;
            Of -> N2;
        "#;

    let path = "/Volumes/Corpora/COHA/conll/*.conllu.gz";
    let pattern = compile_query(query).unwrap();
    let treebank = Treebank::from_glob(path).unwrap();
    let count = treebank.match_iter(pattern, true).count();

    println!("{}", count);
}
