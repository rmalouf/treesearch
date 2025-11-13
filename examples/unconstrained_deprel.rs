//! Example demonstrating unconstrained deprel matching
//!
//! NOTE: This example is currently DISABLED because it was based on the old
//! VM-based matching algorithm which has been removed in favor of the CSP approach.
//!
//! This will be rewritten once the CSP solver is complete to demonstrate:
//! - Constrained vs unconstrained edge labels
//! - How the CSP solver handles optional deprel constraints
//!
//! Run with: cargo run --example unconstrained_deprel

fn main() {
    println!("=== Unconstrained Deprel Demo ===\n");
    println!(
        "This example is currently disabled and needs to be rewritten for the new CSP-based API."
    );
    println!("\nThe new approach will demonstrate:");
    println!("  1. Constrained edge labels: Verb -[nsubj]-> Noun");
    println!("  2. Unconstrained edges: Verb -> Noun (any deprel)");
    println!("  3. How the CSP solver handles these constraints");
    println!("\nPlease check back after the CSP solver is implemented!");
}
