//! Example demonstrating the iterator interfaces

use std::path::Path;
use treesearch::{
    TreeIterator, MatchIterator, MultiFileMatchIterator, MultiFileTreeIterator, parse_query,
};

fn main() {
    println!("=== CoNLLUReader Example ===\n");

    // Example 1: Iterate over trees from a string
    let conllu = r#"# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	_

# text = Cats sleep.
1	Cats	cat	NOUN	NNS	_	2	nsubj	_	_
2	sleep	sleep	VERB	VBP	_	0	root	_	_

# text = Birds fly quickly.
1	Birds	bird	NOUN	NNS	_	2	nsubj	_	_
2	fly	fly	VERB	VBP	_	0	root	_	_
3	quickly	quickly	ADV	RB	_	2	advmod	_	_

"#;

    println!("Iterating over trees:");
    for (idx, tree_result) in TreeIterator::from_string(conllu).enumerate() {
        let tree = tree_result.expect("Failed to parse tree");
        println!(
            "  Tree {}: {} words, text = {:?}",
            idx + 1,
            tree.words.len(),
            tree.sentence_text
        );
    }

    println!("\n=== MatchIterator Example ===\n");

    // Example 2: Search for verbs across all trees
    let pattern = parse_query("V [pos=\"VERB\"];").expect("Failed to parse query");
    println!("Searching for pattern: V [pos=\"VERB\"];");

    for (tree, match_) in MatchIterator::from_string(conllu, pattern.clone()) {
        let word_id = match_[0];
        let word = &tree.words[word_id];
        let form_bytes = tree.string_pool.resolve(word.form);
        let lemma_bytes = tree.string_pool.resolve(word.lemma);
        let form = std::str::from_utf8(&form_bytes).expect("Invalid UTF-8");
        let lemma = std::str::from_utf8(&lemma_bytes).expect("Invalid UTF-8");

        println!("  Found verb '{}' (lemma: {})", form, lemma);
    }

    // Example 3: More complex pattern - verbs with adverbial modifiers
    println!("\nSearching for pattern: V [pos=\"VERB\"]; A [deprel=\"advmod\"]; V -> A;");
    let pattern = parse_query("V [pos=\"VERB\"]; A [deprel=\"advmod\"]; V -> A;")
        .expect("Failed to parse query");

    for (tree, match_) in MatchIterator::from_string(conllu, pattern) {
        let verb_id = match_[0];
        let adv_id = match_[1];

        let verb = &tree.words[verb_id];
        let adv = &tree.words[adv_id];

        let verb_form_bytes = tree.string_pool.resolve(verb.form);
        let adv_form_bytes = tree.string_pool.resolve(adv.form);
        let verb_form = std::str::from_utf8(&verb_form_bytes).expect("Invalid UTF-8");
        let adv_form = std::str::from_utf8(&adv_form_bytes).expect("Invalid UTF-8");

        println!("  Found '{}' -> '{}'", verb_form, adv_form);
    }

    // Example 4: Iterate from a file (if it exists)
    println!("\n=== File Iterator Example ===\n");

    if let Some(path_str) = std::env::args().nth(1) {
        let path = Path::new(&path_str);
        match TreeIterator::from_file(path) {
            Ok(trees) => {
                let count = trees.filter_map(Result::ok).count();
                println!("Successfully read {} trees from {}", count, path_str);
            }
            Err(e) => {
                eprintln!("Error opening file: {}", e);
            }
        }
    } else {
        println!("To test file reading, run: cargo run --example iterators <file.conllu>");
    }

    // Example 5: Multi-file tree iterator with glob patterns
    println!("\n=== Multi-File Tree Iterator Example ===\n");

    // Check if a glob pattern is provided as second argument
    if let Some(glob_pattern) = std::env::args().nth(2) {
        println!("Searching for files matching: {}", glob_pattern);
        match MultiFileTreeIterator::from_glob(&glob_pattern) {
            Ok(trees) => {
                let tree_count = trees.filter_map(Result::ok).count();
                println!("Loaded {} trees from matching files", tree_count);
            }
            Err(e) => {
                eprintln!("Error with glob pattern: {}", e);
            }
        }
    } else {
        println!("To test multi-file iteration, run:");
        println!("  cargo run --example iterators <file> <glob-pattern>");
        println!("  Example: cargo run --example iterators '' 'examples/*.conll*'");
    }

    // Example 6: Multi-file match iterator
    println!("\n=== Multi-File Match Iterator Example ===\n");

    if let Some(glob_pattern) = std::env::args().nth(2) {
        let pattern = parse_query("V [pos=\"VERB\"];").expect("Failed to parse query");

        match MultiFileMatchIterator::from_glob(&glob_pattern, pattern) {
            Ok(matches) => {
                let match_count = matches.count();
                if match_count == 0 {
                    println!("No matches found across files");
                } else {
                    println!("Found {} verb matches across all files", match_count);
                }
            }
            Err(e) => {
                eprintln!("Error with glob pattern: {}", e);
            }
        }
    } else {
        println!("Provide a glob pattern to search for verbs across multiple files");
    }
}
