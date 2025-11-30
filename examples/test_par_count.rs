//! Test parallel count
//!
//! Run with: cargo run --example test_par_count

use rayon::prelude::*;
use treesearch::{MatchSet, TreeSet, parse_query};

fn main() {
    let query = r#"V [pos="VERB"];"#;

    let pattern = parse_query(query).unwrap();
    let tree_set = TreeSet::from_glob("tests/data/*.conllu").unwrap();
    let count = MatchSet::new(&tree_set, &pattern).into_par_iter().count();
    println!("Count: {}", count);
}
