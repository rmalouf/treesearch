//! Example demonstrating query parsing and pattern matching on CoNLL-U files
//!
//! Run with: cargo run --example latwp --release

use treesearch::{MatchSet, Treebank, parse_query};

fn main() {
    let query = r#"
            N1 [pos="NOUN"];
            Of [form="of"];
            N2 [pos="NOUN"];
            N1 -> Of;
            Of -> N2;
        "#;

    let path = "/Volumes/Corpora/COHA/conll/*.conllu.gz";
    let pattern = parse_query(query).unwrap();
    let tree_set = Treebank::from_glob(path).unwrap();
    let count = MatchSet::new(&tree_set, &pattern).into_iter().count();

    println!("{}", count);
}
