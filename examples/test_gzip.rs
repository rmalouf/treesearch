use std::path::Path;
use treesearch::TreeIterator;

fn main() {
    // Test reading plain file
    println!("Testing plain CoNLL-U file...");
    let plain_path = Path::new("examples/lw970831.conll");
    let plain_reader = TreeIterator::from_file(plain_path).expect("Failed to open plain file");
    let plain_count = plain_reader.count();
    println!("Plain file: {} sentences", plain_count);

    // Test reading gzipped file
    println!("\nTesting gzipped CoNLL-U file...");
    let gz_path = Path::new("examples/lw970831.conll.gz");
    let gz_reader = TreeIterator::from_file(gz_path).expect("Failed to open gzipped file");
    let gz_count = gz_reader.count();
    println!("Gzipped file: {} sentences", gz_count);

    // Verify they have the same number of sentences
    if plain_count == gz_count {
        println!("\n✓ Success! Both files have the same number of sentences.");
    } else {
        panic!(
            "Mismatch! Plain: {} sentences, Gzipped: {} sentences",
            plain_count, gz_count
        );
    }
}
