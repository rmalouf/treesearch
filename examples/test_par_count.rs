//! Test parallel count
//! (parallel processing is now handled internally)
//!
//! Run with: cargo run --example test_par_count

use treesearch::{Treebank, parse_query};

fn main() {
    let query = r#"V [pos="VERB"];"#;

    let pattern = parse_query(query).unwrap();
    let treebank = Treebank::from_glob("tests/data/*.conllu").unwrap();
    // Note: parallel processing is now handled internally by match_iter()
    let count = treebank.match_iter(pattern).count();
    println!("Count: {}", count);
}
