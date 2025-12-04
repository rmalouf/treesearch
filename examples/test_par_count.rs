//! Test parallel count
//!
//! Run with: cargo run --example test_par_count

use pariter::IteratorExt as _;
use treesearch::{MatchSet, Treebank, parse_query};

fn main() {
    let query = r#"V [pos="VERB"];"#;

    let pattern = parse_query(query).unwrap();
    let tree_set = Treebank::from_glob("tests/data/*.conllu").unwrap();
    let count = MatchSet::new(&tree_set, &pattern)
        .into_iter()
        .parallel_map(|m| m)
        .count();
    println!("Count: {}", count);
}
