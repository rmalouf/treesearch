//! Example demonstrating query parsing and pattern matching on CoNLL-U files
//!
//! NOTE: This example is currently DISABLED because it uses the old API.
//! The CSP solver and Match type need to be completed first.
//!
//! This will be rewritten once the CSP solver is complete to demonstrate:
//! - Reading CoNLL-U files
//! - Executing queries over treebanks
//! - Accessing matched words from results
//!
//! Run with: cargo run --example latwp

fn main() {
    println!("=== LATWP Query Demo ===\n");
    println!(
        "This example is currently disabled and needs to be rewritten for the new CSP-based API."
    );
    println!("\nOnce complete, it will demonstrate:");
    println!("  1. Reading CoNLL-U treebank files");
    println!(
        "  2. Running queries like: Noun [pos=\"NOUN\"]; Adj [pos=\"ADJ\"]; Noun -[amod]-> Adj;"
    );
    println!("  3. Accessing matched words from the results");
    println!("\nPlease check back after the CSP solver is implemented!");
}
