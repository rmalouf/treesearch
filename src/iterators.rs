//! Iterators for trees and matches
//!
//! Provides convenient collection interfaces for:
//! - Iterating over trees from a string, file, or glob pattern
//! - Searching patterns across trees from a string, file, or glob pattern
//! - Sequential and parallel iteration via standard traits

use crate::conllu::{ParseError, TreeIterator};
use crate::pattern::Pattern;
use crate::searcher::{Match, search_tree, tree_matches};
use crate::tree::Tree;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::sync_channel;
use std::thread;
use thiserror::Error;

/// Errors that can occur during treebank iteration
#[derive(Debug, Error)]
pub enum TreebankError {
    /// IO error when opening or reading files
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error when reading CoNLL-U content
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    /// Error opening file at specific path
    #[error("Failed to open file {path}: {source}")]
    FileOpen {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Batch size for sending matches through channels
const MATCH_BATCH_SIZE: usize = 500;

/// Channel buffer size (in batches)
const CHANNEL_BUFFER_SIZE: usize = 100;

/// Helper for accumulating items into batches
struct BatchAccumulator<T> {
    batch: Vec<T>,
    capacity: usize,
}

impl<T> BatchAccumulator<T> {
    /// Create a new batch accumulator with the given capacity
    fn new(capacity: usize) -> Self {
        Self {
            batch: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Push an item into the batch. Returns Some(batch) if the batch is full.
    fn push(&mut self, item: T) -> Option<Vec<T>> {
        self.batch.push(item);
        if self.batch.len() >= self.capacity {
            Some(std::mem::replace(
                &mut self.batch,
                Vec::with_capacity(self.capacity),
            ))
        } else {
            None
        }
    }

    /// Flush any remaining items in the batch
    fn flush(self) -> Option<Vec<T>> {
        if self.batch.is_empty() {
            None
        } else {
            Some(self.batch)
        }
    }
}

/// Process trees from a string source with batching (for match_iter and filter)
fn process_string_source_batched<T, F>(
    text: &str,
    tx: &crossbeam_channel::Sender<Vec<Result<T, TreebankError>>>,
    process_tree: F,
) where
    T: Send,
    F: Fn(Tree) -> Vec<Result<T, TreebankError>>,
{
    let mut batch = BatchAccumulator::new(MATCH_BATCH_SIZE);
    for result in TreeIterator::from_string(text) {
        let items = match result {
            Ok(tree) => process_tree(tree),
            Err(e) => vec![Err(TreebankError::from(e))],
        };
        for item in items {
            if let Some(full_batch) = batch.push(item) {
                if tx.send(full_batch).is_err() {
                    return;
                }
            }
        }
    }
    if let Some(final_batch) = batch.flush() {
        let _ = tx.send(final_batch);
    }
}

/// Process files in ordered mode with chunking (for match_iter and filter)
fn process_files_ordered_batched<T, F>(
    paths: Vec<PathBuf>,
    tx: &crossbeam_channel::Sender<Vec<Result<T, TreebankError>>>,
    process_tree: F,
    chunk_size: usize,
) where
    T: Send,
    F: Fn(Tree) -> Vec<Result<T, TreebankError>> + Send + Sync,
{
    for chunk in paths.chunks(chunk_size) {
        // Compute per-path results in parallel, keeping them grouped by path
        let per_path: Vec<Vec<Result<T, TreebankError>>> = chunk
            .par_iter()
            .map(|path| match TreeIterator::from_file(path) {
                Ok(it) => it
                    .flat_map(|result| match result {
                        Ok(tree) => process_tree(tree),
                        Err(e) => vec![Err(TreebankError::from(e))],
                    })
                    .collect(),
                Err(e) => vec![Err(TreebankError::FileOpen {
                    path: path.clone(),
                    source: e,
                })],
            })
            .collect();

        // Send batches in deterministic order: path order, then result order within each path
        for batch in per_path {
            if !batch.is_empty() && tx.send(batch).is_err() {
                return;
            }
        }
    }
}

/// Process files in unordered mode with full parallelism (for match_iter and filter)
fn process_files_unordered_batched<T, F>(
    paths: Vec<PathBuf>,
    tx: crossbeam_channel::Sender<Vec<Result<T, TreebankError>>>,
    process_tree: F,
) where
    T: Send,
    F: Fn(Tree) -> Vec<Result<T, TreebankError>> + Send + Sync,
{
    paths.par_iter().for_each(|path| {
        let tx = tx.clone();
        match TreeIterator::from_file(path) {
            Ok(reader) => {
                let mut batch = BatchAccumulator::new(MATCH_BATCH_SIZE);
                for result in reader {
                    let items = match result {
                        Ok(tree) => process_tree(tree),
                        Err(e) => vec![Err(TreebankError::from(e))],
                    };
                    for item in items {
                        if let Some(full_batch) = batch.push(item) {
                            if tx.send(full_batch).is_err() {
                                return;
                            }
                        }
                    }
                }
                if let Some(final_batch) = batch.flush() {
                    let _ = tx.send(final_batch);
                }
            }
            Err(e) => {
                let _ = tx.send(vec![Err(TreebankError::FileOpen {
                    path: path.clone(),
                    source: e,
                })]);
            }
        }
    });
}

/// Build a parallel iterator with batching (for match_iter and filter)
fn build_parallel_iter_batched<T, F>(
    source: TreeSource,
    ordered: bool,
    chunk_size: usize,
    process_tree: F,
) -> impl Iterator<Item = Result<T, TreebankError>>
where
    T: Send + 'static,
    F: Fn(Tree) -> Vec<Result<T, TreebankError>> + Send + Sync + Clone + 'static,
{
    let (tx, rx) = crossbeam_channel::bounded(CHANNEL_BUFFER_SIZE);

    thread::spawn(move || match source {
        TreeSource::String(text) => {
            process_string_source_batched(&text, &tx, process_tree);
        }
        TreeSource::Files(paths) => {
            if ordered {
                process_files_ordered_batched(paths, &tx, process_tree, chunk_size);
            } else {
                process_files_unordered_batched(paths, tx, process_tree);
            }
        }
    });

    rx.into_iter().flatten()
}

/// Source of trees for a collection
#[derive(Debug, Clone)]
enum TreeSource {
    /// In-memory CoNLL-U text
    String(String),
    /// Multiple file paths (from glob or explicit path(s))
    Files(Vec<PathBuf>),
}

///
/// Provides iterator-based access to trees with parallel processing.
/// Errors (file open, parse errors) are returned in the iterator for proper handling.
///
/// # Examples
///
/// ```no_run
/// use treesearch::Treebank;
///
/// // Iterate over trees from a file
/// let trees = Treebank::from_path("data.conllu");
/// for result in trees.tree_iter(true) {
///     match result {
///         Ok(tree) => println!("Tree with {} words", tree.words.len()),
///         Err(e) => eprintln!("Error: {}", e),
///     }
/// }
///
/// // Count trees from multiple files (parallel processing handled internally)
/// let count = Treebank::from_glob("data/*.conllu")
///     .unwrap()
///     .tree_iter(true)
///     .filter_map(Result::ok)
///     .count();
/// ```
#[derive(Clone)]
pub struct Treebank {
    source: TreeSource,
}

impl Treebank {
    /// Create from an in-memory CoNLL-U string
    pub fn from_string(text: &str) -> Self {
        Self {
            source: TreeSource::String(text.to_string()),
        }
    }

    /// Create from a single file path
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path_vec = vec![path.as_ref().to_path_buf()];
        Self::from_paths(path_vec)
    }

    /// Create from explicit file paths
    pub fn from_paths(file_paths: Vec<PathBuf>) -> Self {
        Self {
            source: TreeSource::Files(file_paths),
        }
    }

    /// Create from a glob pattern
    ///
    /// Files are processed in sorted order for deterministic results.
    pub fn from_glob(pattern: &str) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> = glob::glob(pattern)?.filter_map(Result::ok).collect();
        file_paths.sort();
        Ok(Self::from_paths(file_paths))
    }

    /// Iterate over trees with optional ordering.
    ///
    /// Returns an iterator over `Result<Tree, TreebankError>`. Errors from file I/O
    /// or parsing are returned in the iterator rather than being silently logged.
    ///
    /// # Arguments
    /// * `ordered` - If true (default), maintains file and tree order for deterministic results.
    ///   If false, trees may arrive in any order for better performance.
    ///
    /// # Examples
    /// ```no_run
    /// use treesearch::Treebank;
    ///
    /// let treebank = Treebank::from_glob("data/*.conllu").unwrap();
    ///
    /// // Ordered iteration (deterministic)
    /// for result in treebank.clone().tree_iter(true) {
    ///     match result {
    ///         Ok(tree) => println!("Tree: {}", tree.words.len()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    ///
    /// // Unordered iteration (faster for large corpora), ignoring errors
    /// for tree in treebank.tree_iter(false).filter_map(Result::ok) {
    ///     println!("Tree: {}", tree.words.len());
    /// }
    /// ```
    pub fn tree_iter(self, ordered: bool) -> impl Iterator<Item = Result<Tree, TreebankError>> {
        if ordered {
            // Ordered mode: maintain deterministic ordering via chunking
            // Smaller chunks (2 files) improve load balancing for heterogeneous file sizes
            let (tx, rx) = sync_channel(64); // larger buffer for better pipelining

            thread::spawn(move || match self.source {
                TreeSource::String(text) => {
                    for result in TreeIterator::from_string(&text) {
                        let result = result.map_err(TreebankError::from);
                        if tx.send(result).is_err() {
                            return;
                        }
                    }
                }
                TreeSource::Files(paths) => {
                    for chunk in paths.chunks(2) {
                        let results: Vec<_> = chunk
                            .par_iter()
                            .flat_map_iter(|path| {
                                let file_results: Vec<Result<Tree, TreebankError>> =
                                    match TreeIterator::from_file(path) {
                                        Ok(iter) => {
                                            iter.map(|r| r.map_err(TreebankError::from)).collect()
                                        }
                                        Err(e) => vec![Err(TreebankError::FileOpen {
                                            path: path.clone(),
                                            source: e,
                                        })],
                                    };
                                file_results.into_iter()
                            })
                            .collect();
                        for result in results {
                            if tx.send(result).is_err() {
                                return;
                            }
                        }
                    }
                }
            });
            rx.into_iter()
        } else {
            // Unordered mode: maximum concurrency by removing synchronization barriers
            let (tx, rx) = sync_channel(5000); // larger buffer for higher throughput

            thread::spawn(move || match self.source {
                TreeSource::String(text) => {
                    for result in TreeIterator::from_string(&text) {
                        let result = result.map_err(TreebankError::from);
                        if tx.send(result).is_err() {
                            return;
                        }
                    }
                }
                TreeSource::Files(paths) => {
                    paths.par_iter().for_each(|path| {
                        let tx = tx.clone(); // Clone sender for each parallel thread
                        match TreeIterator::from_file(path) {
                            Ok(reader) => {
                                for result in reader {
                                    let result = result.map_err(TreebankError::from);
                                    if tx.send(result).is_err() {
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(Err(TreebankError::FileOpen {
                                    path: path.clone(),
                                    source: e,
                                }));
                            }
                        }
                    });
                }
            });
            rx.into_iter()
        }
    }

    /// Search for pattern matches with optional ordering.
    ///
    /// Returns an iterator over `Result<Match, TreebankError>`. Errors from file I/O
    /// or parsing are returned in the iterator rather than being silently logged.
    ///
    /// # Arguments
    /// * `pattern` - The pattern to search for
    /// * `ordered` - If true (default), maintains file and tree order for deterministic results.
    ///   If false, matches may arrive in any order for better performance.
    ///
    /// # Examples
    /// ```no_run
    /// use treesearch::{Treebank, compile_query};
    ///
    /// let treebank = Treebank::from_glob("data/*.conllu").unwrap();
    /// let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
    ///
    /// // Ordered iteration (deterministic)
    /// for result in treebank.clone().match_iter(pattern.clone(), true) {
    ///     match result {
    ///         Ok(m) => println!("Match found"),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    ///
    /// // Unordered iteration (faster for large corpora), ignoring errors
    /// for m in treebank.match_iter(pattern, false).filter_map(Result::ok) {
    ///     println!("Match found");
    /// }
    /// ```
    pub fn match_iter(
        self,
        pattern: Pattern,
        ordered: bool,
    ) -> impl Iterator<Item = Result<Match, TreebankError>> {
        build_parallel_iter_batched(
            self.source,
            ordered,
            4, // chunk_size for ordered mode
            move |tree| search_tree(tree, &pattern).into_iter().map(Ok).collect(),
        )
    }

    /// Filter trees that match a pattern.
    ///
    /// Returns an iterator over trees that have at least one match for the pattern.
    /// Uses early termination for efficiency - stops searching each tree after finding
    /// the first match.
    ///
    /// # Arguments
    /// * `pattern` - The pattern to match against
    /// * `ordered` - If true, maintains file and tree order. If false, may be faster.
    pub fn filter(
        self,
        pattern: Pattern,
        ordered: bool,
    ) -> impl Iterator<Item = Result<Tree, TreebankError>> {
        build_parallel_iter_batched(
            self.source,
            ordered,
            4, // chunk_size for ordered mode
            move |tree| {
                if tree_matches(&tree, &pattern) {
                    vec![Ok(tree)]
                } else {
                    vec![]
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_query;

    const TWO_TREE_CONLLU: &str = r#"# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	_

# text = Cats sleep.
1	Cats	cat	NOUN	NNS	_	2	nsubj	_	_
2	sleep	sleep	VERB	VBP	_	0	root	_	_

"#;

    const THREE_VERB_CONLLU: &str = r#"1	helped	help	VERB	VBD	_	0	root	_	_
2	us	we	PRON	PRP	_	1	obj	_	_

1	ran	run	VERB	VBD	_	0	root	_	_
2	quickly	quickly	ADV	RB	_	1	advmod	_	_

1	sleeps	sleep	VERB	VBZ	_	0	root	_	_

"#;

    #[test]
    fn test_treebank_from_string() {
        let trees: Vec<_> = Treebank::from_string(TWO_TREE_CONLLU)
            .tree_iter(true)
            .filter_map(Result::ok)
            .collect();

        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].words.len(), 3);
        assert_eq!(trees[1].words.len(), 2);
    }

    #[test]
    fn test_match_set_from_string() {
        let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let tree_set = Treebank::from_string(THREE_VERB_CONLLU);
        let matches: Vec<_> = tree_set
            .match_iter(pattern, true)
            .filter_map(Result::ok)
            .collect();

        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_match_set_multiple_matches_per_tree() {
        let conllu = "1\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                      2\tJohn\tJohn\tPROPN\tNNP\t_\t1\tobj\t_\t_\n\
                      3\trunning\trun\tVERB\tVBG\t_\t1\txcomp\t_\t_\n";

        let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let tree_set = Treebank::from_string(conllu);
        let matches: Vec<_> = tree_set
            .match_iter(pattern, true)
            .filter_map(Result::ok)
            .collect();

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_match_set_no_matches() {
        let conllu = "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n\
                      2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n";

        let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let tree_set = Treebank::from_string(conllu);
        let matches: Vec<_> = tree_set
            .match_iter(pattern, true)
            .filter_map(Result::ok)
            .collect();

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_match_set_with_constraints() {
        let conllu = "1\thelped\thelp\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                      2\tus\twe\tPRON\tPRP\t_\t1\tobj\t_\t_\n\
                      3\tto\tto\tPART\tTO\t_\t4\tmark\t_\t_\n\
                      4\twin\twin\tVERB\tVB\t_\t1\txcomp\t_\t_\n";

        let pattern =
            compile_query("MATCH { V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; V1 -> V2; }").unwrap();
        let tree_set = Treebank::from_string(conllu);
        let matches: Vec<_> = tree_set
            .match_iter(pattern, true)
            .filter_map(Result::ok)
            .collect();

        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_filter() {
        // THREE_VERB_CONLLU has 3 trees, each with one verb
        let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();

        // All 3 trees match the pattern
        let trees: Vec<_> = Treebank::from_string(THREE_VERB_CONLLU)
            .filter(pattern.clone(), true)
            .filter_map(Result::ok)
            .collect();
        assert_eq!(trees.len(), 3);

        // Pattern that matches only some trees (lemma="help")
        let pattern = compile_query("MATCH { V [lemma=\"help\"]; }").unwrap();
        let trees: Vec<_> = Treebank::from_string(THREE_VERB_CONLLU)
            .filter(pattern, true)
            .filter_map(Result::ok)
            .collect();
        assert_eq!(trees.len(), 1);

        // Pattern that matches no trees
        let pattern = compile_query("MATCH { N [upos=\"NOUN\"]; }").unwrap();
        let trees: Vec<_> = Treebank::from_string(THREE_VERB_CONLLU)
            .filter(pattern, true)
            .filter_map(Result::ok)
            .collect();
        assert_eq!(trees.len(), 0);
    }

    #[cfg(test)]
    mod multi_file {
        use super::*;
        use std::fs;
        use std::io::Write;
        use std::path::PathBuf;
        use tempfile::{TempDir, tempdir};

        /// Helper to create test files with given content
        fn create_test_files(contents: &[(&str, &str)]) -> (TempDir, Vec<PathBuf>) {
            let dir = tempdir().unwrap();
            let mut paths = Vec::new();

            for (filename, content) in contents {
                let path = dir.path().join(filename);
                let mut file = fs::File::create(&path).unwrap();
                write!(file, "{}", content).unwrap();
                paths.push(path);
            }

            (dir, paths)
        }

        #[test]
        fn test_treebank_from_paths() {
            let (_dir, paths) = create_test_files(&[
                (
                    "file1.conllu",
                    "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n",
                ),
                (
                    "file2.conllu",
                    "1\tCats\tcat\tNOUN\tNNS\t_\t2\tnsubj\t_\t_\n2\tsleep\tsleep\tVERB\tVBP\t_\t0\troot\t_\t_\n",
                ),
            ]);

            let results: Vec<_> = Treebank::from_paths(paths)
                .tree_iter(true)
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 2);
            assert_eq!(results[0].words.len(), 2);
            assert_eq!(results[1].words.len(), 2);
        }

        #[test]
        fn test_treebank_from_glob() {
            let (dir, _paths) = create_test_files(&[
                (
                    "test1.conllu",
                    "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n",
                ),
                (
                    "test2.conllu",
                    "1\tCats\tcat\tNOUN\tNNS\t_\t2\tnsubj\t_\t_\n2\tsleep\tsleep\tVERB\tVBP\t_\t0\troot\t_\t_\n",
                ),
                ("other.txt", "ignored"),
            ]);

            let pattern = format!("{}/*.conllu", dir.path().display());
            let results: Vec<_> = Treebank::from_glob(&pattern)
                .unwrap()
                .tree_iter(true)
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 2);
        }

        #[test]
        fn test_match_set_from_paths() {
            let (_dir, paths) = create_test_files(&[
                (
                    "file1.conllu",
                    "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
                (
                    "file2.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
            ]);

            let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
            let tree_set = Treebank::from_paths(paths);
            let results: Vec<_> = tree_set
                .match_iter(pattern, true)
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 2);
        }

        #[test]
        fn test_match_set_from_glob() {
            let (dir, _paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
            ]);

            let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
            let glob_pattern = format!("{}/*.conllu", dir.path().display());
            let tree_set = Treebank::from_glob(&glob_pattern).unwrap();
            let results: Vec<_> = tree_set
                .match_iter(pattern, true)
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 2);
        }

        #[test]
        fn test_reports_bad_files() {
            let (dir, mut paths) = create_test_files(&[(
                "good.conllu",
                "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
            )]);

            let good_file = paths[0].clone();
            let bad_file = dir.path().join("nonexistent.conllu");
            paths = vec![good_file.clone(), bad_file, good_file];

            let results: Vec<_> = Treebank::from_paths(paths).tree_iter(true).collect();

            // Should get 2 Ok results and 1 Err result
            assert_eq!(results.len(), 3);
            assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 2);
            assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
        }

        #[test]
        fn test_ordered_iteration_deterministic() {
            let (_dir, paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
                ("c.conllu", "1\twalks\twalk\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
            ]);

            // Multiple iterations should produce same order
            let treebank = Treebank::from_paths(paths.clone());
            let run1: Vec<_> = treebank
                .clone()
                .tree_iter(true)
                .filter_map(Result::ok)
                .collect();
            let run2: Vec<_> = treebank
                .clone()
                .tree_iter(true)
                .filter_map(Result::ok)
                .collect();

            assert_eq!(run1.len(), 3);
            assert_eq!(run2.len(), 3);

            // Verify same order by comparing lemmas
            for (t1, t2) in run1.iter().zip(run2.iter()) {
                assert_eq!(
                    t1.string_pool.resolve(t1.words[0].lemma),
                    t2.string_pool.resolve(t2.words[0].lemma)
                );
            }
        }

        #[test]
        fn test_unordered_iteration_completeness() {
            let (_dir, paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
                ("c.conllu", "1\twalks\twalk\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
            ]);

            let treebank = Treebank::from_paths(paths);
            let results: Vec<_> = treebank.tree_iter(false).filter_map(Result::ok).collect();

            // Should still get all trees, just possibly in different order
            assert_eq!(results.len(), 3);

            // Verify we got all the expected lemmas
            let mut lemmas: Vec<Vec<u8>> = results
                .iter()
                .map(|t| t.string_pool.resolve(t.words[0].lemma).to_vec())
                .collect();
            lemmas.sort();

            let expected: Vec<Vec<u8>> = vec![b"run".to_vec(), b"sleep".to_vec(), b"walk".to_vec()];
            assert_eq!(lemmas, expected);
        }

        #[test]
        fn test_match_iter_ordered() {
            let (_dir, paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
            ]);

            let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
            let treebank = Treebank::from_paths(paths);
            let results: Vec<_> = treebank
                .match_iter(pattern, true)
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 2);
        }

        #[test]
        fn test_match_iter_unordered() {
            let (_dir, paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
            ]);

            let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
            let treebank = Treebank::from_paths(paths);
            let results: Vec<_> = treebank
                .match_iter(pattern, false)
                .filter_map(Result::ok)
                .collect();

            // Should get all matches, order doesn't matter
            assert_eq!(results.len(), 2);
        }
    }
}
