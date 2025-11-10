//! Complete end-to-end example: CoNLL-U → Query → Results
//!
//! This example demonstrates the full pipeline:
//! 1. Parse CoNLL-U text into a Tree
//! 2. Execute a query on the tree
//! 3. Extract and display results
//!
//! Run with: cargo run --example end_to_end

use treesearch::{CoNLLUReader, search_query};

fn main() {
    println!("=== Treesearch: End-to-End Example ===\n");

    // Sample CoNLL-U data: "The big dog runs quickly."
    let conllu_text = r#"# sent_id = example-001
# text = The big dog runs quickly.
1	The	the	DET	DT	Definite=Def|PronType=Art	3	det	_	_
2	big	big	ADJ	JJ	Degree=Pos	3	amod	_	_
3	dog	dog	NOUN	NN	Number=Sing	4	nsubj	_	_
4	runs	run	VERB	VBZ	Mood=Ind|Number=Sing|Person=3|Tense=Pres|VerbForm=Fin	0	root	_	_
5	quickly	quickly	ADV	RB	_	4	advmod	_	SpaceAfter=No
6	.	.	PUNCT	.	_	4	punct	_	_

"#;

    println!("📄 Input CoNLL-U:");
    println!("{}", conllu_text);

    // Step 1: Parse CoNLL-U into Tree
    println!("🔧 Step 1: Parsing CoNLL-U...");
    let mut reader = CoNLLUReader::from_string(conllu_text);
    let tree = match reader.next() {
        Some(Ok(tree)) => tree,
        Some(Err(e)) => {
            eprintln!("❌ Parse error: {}", e);
            return;
        }
        None => {
            eprintln!("❌ No sentences found");
            return;
        }
    };

    println!("✅ Parsed tree with {} nodes", tree.len());
    if let Some(text) = &tree.sentence_text {
        println!("   Text: {}", text);
    }
    if let Some(sent_id) = tree.metadata.get("sent_id") {
        println!("   ID: {}", sent_id);
    }
    println!();

    // Display tree structure
    println!("🌳 Tree structure:");
    for node in tree.nodes() {
        let parent_info = if let Ok(Some(parent_id)) = tree.parent_id(node.id) {
            format!("→ {} ({})", tree.nodes()[parent_id].form, node.deprel)
        } else {
            format!("({})", node.deprel)
        };
        println!(
            "   {}: {} [{}] {}",
            node.id, node.form, node.pos, parent_info
        );
    }
    println!();

    // Step 2: Ready to search
    println!("🔍 Step 2: Ready to search...\n");

    // Example queries to demonstrate different features
    let queries = vec![
        ("Query 1: Find all VERB nodes", r#"V [pos="VERB"];"#),
        (
            "Query 2: Find VERB with NOUN subject",
            r#"
                V [pos="VERB"];
                N [pos="NOUN"];
                V -[nsubj]-> N;
            "#,
        ),
        (
            "Query 3: Find NOUN with ADJ modifier",
            r#"
                N [pos="NOUN"];
                A [pos="ADJ"];
                N -[amod]-> A;
            "#,
        ),
        (
            "Query 4: Find the specific word 'dog'",
            r#"D [lemma="dog"];"#,
        ),
        (
            "Query 5: Find VERB with ADV modifier",
            r#"
                V [pos="VERB"];
                Adv [pos="ADV"];
                V -[advmod]-> Adv;
            "#,
        ),
    ];

    // Execute each query
    for (description, query) in &queries {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("{}", description);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Query:");
        println!("{}", query.trim());
        println!();

        // Step 3: Execute query
        match search_query(&tree, query) {
            Ok(matches) => {
                let matches: Vec<_> = matches.collect();
                println!("✅ Found {} match(es)", matches.len());

                if matches.is_empty() {
                    println!("   (no matches)");
                } else {
                    for (match_idx, match_result) in matches.iter().enumerate() {
                        println!("\n   Match #{}:", match_idx + 1);

                        // Use the new iter_named() method to display bindings with names
                        for (var_name, node_id) in match_result.iter_named() {
                            let node = &tree.nodes()[node_id];
                            println!(
                                "     {}: {} (lemma: {}, pos: {})",
                                var_name, node.form, node.lemma, node.pos
                            );
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Query error: {}", e);
            }
        }

        println!();
    }

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Successfully demonstrated:");
    println!("   • CoNLL-U parsing");
    println!("   • Tree construction with {} nodes", tree.len());
    println!("   • Multiple query patterns");
    println!("   • Index-based candidate filtering");
    println!("   • VM-based pattern matching");
    println!("\n🎉 Complete pipeline working!\n");

    // Show some internals
    println!("🔍 Pipeline Details:");
    println!("   Components:");
    println!("   1. Parser     → Converts CoNLL-U to Tree");
    println!("   2. Query      → Parses query string to Pattern");
    println!("   3. Compiler   → Compiles Pattern to VM opcodes");
    println!("   4. Index      → Finds candidate nodes quickly");
    println!("   5. VM         → Verifies pattern matches");
    println!("   6. Results    → Returns Match objects");
    println!();
}
