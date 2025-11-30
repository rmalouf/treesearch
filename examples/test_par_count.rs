//! Test parallel count
//!
//! Run with: cargo run --example test_par_count

use rayon::prelude::*;
use treesearch::{MatchSet, parse_query};

fn main() {
    let query = r#"V [pos="VERB"];"#;

    let pattern = parse_query(query).unwrap();
    let count = MatchSet::from_glob("tests/data/*.conllu", pattern)
        .unwrap()
        .into_par_iter()
        .count();
    println!("Count: {}", count);
}
