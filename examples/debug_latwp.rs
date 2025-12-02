//! Debug version to see what's happening
//!
//! Run with: cargo run --example debug_latwp

use treesearch::{MatchSet, Treebank, parse_query};

fn main() {
    let path = "/Volumes/Corpora/Corpora/parsed/NA_NEWS/latwp/1994/**/*.conll.gz";

    println!("Testing glob pattern...");
    let tree_count = Treebank::from_glob(path)
        .unwrap()
        .into_iter()
        .take(5) // Just take first 5 to see if it works
        .count();
    println!(
        "Successfully read {} trees from first few files",
        tree_count
    );

    let query = r#"N1 [pos="NOUN"]; Of [form="of"]; N2 [pos="NOUN"]; N1 -> Of; Of -> N2;"#;
    let pattern = parse_query(query).unwrap();

    println!("\nCounting all matches (sequential)...");
    let tree_set = Treebank::from_glob(path).unwrap();
    let count = MatchSet::new(&tree_set, &pattern).into_iter().count();
    println!("Total matches: {}", count);
}
