//! Example demonstrating query parsing and pattern matching on CoNLL-U files
//!
//! Run with: cargo run --example latwp

use rayon::prelude::*;
use treesearch::{MultiFileMatchIterator, parse_query};

fn main() {
    let query = r#"
            N1 [pos="NOUN"];
            Of [form="of"];
            N2 [pos="NOUN"];
            N1 -> Of;
            Of -> N2;
        "#;

    let path = "/Volumes/Corpora/Corpora/parsed/COCA/*.conll.gz";
    let pattern = parse_query(query).unwrap();
    let count = MultiFileMatchIterator::from_glob(path, pattern)
        .unwrap()
        .par_iter()
        .count();

    println!("{}", count);
}
