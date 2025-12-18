//! Example demonstrating query parsing and pattern matching on CoNLL-U files
//! (parallel processing is now handled internally)
//!
//! Run with: cargo run --example latwp_par --release

use treesearch::{Treebank, parse_query};

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
    let treebank = Treebank::from_glob(path).unwrap();
    // Note: unordered mode (false) enables maximum parallelism for best performance
    let count = treebank.match_iter(pattern, false).count();

    println!("{}", count);
}
