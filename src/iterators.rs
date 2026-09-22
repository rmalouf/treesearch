//! Iterator interfaces for trees and pattern matches.
//!
//! The main entry points are:
//!
//! - [`load`] — load a [`Treebank`] from a glob pattern (convenience wrapper)
//! - [`Treebank`] — collection of trees with [`trees`](Treebank::trees),
//!   [`search`](Treebank::search), and [`filter`](Treebank::filter) iterators
//! - [`IntoPattern`] — conversion trait that lets `search`/`filter` accept
//!   either a pre-compiled [`Pattern`] or a raw `&str`/`String`
//!
//! # Quick start
//!
//! ```no_run
//! use treesearch::{load, compile_query};
//!
//! let tb = load("data/*.conllu")?;
//!
//! // Iterate every tree
//! for tree in tb.clone().trees(true).filter_map(Result::ok) {
//!     println!("{}", tree.words.len());
//! }
//!
//! // Search with an inline query string — no separate compile step needed
//! for m in tb.clone().search("MATCH { V [upos=\"VERB\"]; }", true)? {
//!     let m = m?;
//! }
//!
//! // Or pre-compile when reusing the same pattern
//! let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }")?;
//! for m in tb.search(pattern, true)? { }
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```

use crate::conllu::{ParseError, TreeIterator};
use crate::pattern::Pattern;
use crate::query::{QueryError, compile_query};
use crate::searcher::{Match, search_tree, tree_matches};
use crate::tree::Tree;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::thread;
use thiserror::Error;

/// Conversion trait for search patterns.
///
/// Allows [`Treebank::search`] and [`Treebank::filter`] to accept either a
/// pre-compiled [`Pattern`] or a raw query string (`&str` / `String`), following
/// the same idiom as `AsRef<Path>` for file paths.
///
/// Pre-compiling is worth it when the same pattern is used across many calls;
/// passing a string is more ergonomic for one-off searches.
pub trait IntoPattern {
    /// Convert `self` into a [`Pattern`], compiling from a string if necessary.
    ///
    /// Returns `Err(QueryError)` if `self` is a string that fails to parse.
    /// The conversion is infallible when `self` is already a `Pattern`.
    fn into_pattern(self) -> Result<Pattern, QueryError>;
}

impl IntoPattern for Pattern {
    fn into_pattern(self) -> Result<Pattern, QueryError> {
        Ok(self)
    }
}

impl IntoPattern for &str {
    fn into_pattern(self) -> Result<Pattern, QueryError> {
        compile_query(self)
    }
}

impl IntoPattern for String {
    fn into_pattern(self) -> Result<Pattern, QueryError> {
        compile_query(&self)
    }
}

#[derive(Debug, Error)]
pub enum TreebankError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("Failed to open file {path}: {source}")]
    FileOpen {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Shared progress counters and cancellation flag for a running search.
///
/// Create one, pass a clone of the `Arc` to [`Treebank::search_with`], and poll
/// the counters from another thread (e.g. a UI). Calling [`cancel`](Self::cancel)
/// makes the workers stop at the next tree boundary.
#[derive(Debug, Default)]
pub struct Progress {
    /// Number of files to process (set when the search starts)
    pub files_total: AtomicUsize,
    /// Number of files fully processed
    pub files_done: AtomicUsize,
    /// Number of trees processed so far
    pub trees: AtomicUsize,
    cancelled: AtomicBool,
}

impl Progress {
    /// Request that the search stop as soon as possible.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Whether [`cancel`](Self::cancel) has been called.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

// Batch handling

const MATCH_BATCH_SIZE: usize = 500;
const CHANNEL_BUFFER_SIZE: usize = 100;

struct BatchAccumulator<T> {
    batch: Vec<T>,
    capacity: usize,
}

impl<T> BatchAccumulator<T> {
    fn new(capacity: usize) -> Self {
        Self {
            batch: Vec::with_capacity(capacity),
            capacity,
        }
    }

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

    fn flush(self) -> Option<Vec<T>> {
        if self.batch.is_empty() {
            None
        } else {
            Some(self.batch)
        }
    }
}

fn process_string_source_batched<T, F>(
    text: &str,
    tx: &crossbeam_channel::Sender<Vec<Result<T, TreebankError>>>,
    process_tree: F,
    progress: &Progress,
) where
    T: Send,
    F: Fn(Tree) -> Vec<Result<T, TreebankError>>,
{
    let mut batch = BatchAccumulator::new(MATCH_BATCH_SIZE);
    for result in TreeIterator::from_string(text) {
        if progress.is_cancelled() {
            return;
        }
        let items = match result {
            Ok(tree) => {
                progress.trees.fetch_add(1, Ordering::Relaxed);
                process_tree(tree)
            }
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

fn process_files_ordered_batched<T, F>(
    paths: Vec<PathBuf>,
    tx: &crossbeam_channel::Sender<Vec<Result<T, TreebankError>>>,
    process_tree: F,
    chunk_size: usize,
    progress: &Progress,
) where
    T: Send,
    F: Fn(Tree) -> Vec<Result<T, TreebankError>> + Send + Sync,
{
    for chunk in paths.chunks(chunk_size) {
        if progress.is_cancelled() {
            return;
        }
        // Compute per-path results in parallel, keeping them grouped by path
        let per_path: Vec<Vec<Result<T, TreebankError>>> = chunk
            .par_iter()
            .map(|path| {
                let results = match TreeIterator::from_file(path) {
                    Ok(it) => it
                        .take_while(|_| !progress.is_cancelled())
                        .flat_map(|result| match result {
                            Ok(tree) => {
                                progress.trees.fetch_add(1, Ordering::Relaxed);
                                process_tree(tree)
                            }
                            Err(e) => vec![Err(TreebankError::from(e))],
                        })
                        .collect(),
                    Err(e) => vec![Err(TreebankError::FileOpen {
                        path: path.clone(),
                        source: e,
                    })],
                };
                progress.files_done.fetch_add(1, Ordering::Relaxed);
                results
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

fn process_files_unordered_batched<T, F>(
    paths: Vec<PathBuf>,
    tx: crossbeam_channel::Sender<Vec<Result<T, TreebankError>>>,
    process_tree: F,
    progress: &Progress,
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
                    if progress.is_cancelled() {
                        return;
                    }
                    let items = match result {
                        Ok(tree) => {
                            progress.trees.fetch_add(1, Ordering::Relaxed);
                            process_tree(tree)
                        }
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
        progress.files_done.fetch_add(1, Ordering::Relaxed);
    });
}

fn build_parallel_iter_batched<T, F>(
    source: TreeSource,
    ordered: bool,
    chunk_size: usize,
    progress: Arc<Progress>,
    process_tree: F,
) -> impl Iterator<Item = Result<T, TreebankError>>
where
    T: Send + 'static,
    F: Fn(Tree) -> Vec<Result<T, TreebankError>> + Send + Sync + Clone + 'static,
{
    let (tx, rx) = crossbeam_channel::bounded(CHANNEL_BUFFER_SIZE);

    let files_total = match &source {
        TreeSource::String(_) => 1,
        TreeSource::Files(paths) => paths.len(),
    };
    progress.files_total.store(files_total, Ordering::Relaxed);

    thread::spawn(move || match source {
        TreeSource::String(text) => {
            process_string_source_batched(&text, &tx, process_tree, &progress);
            progress.files_done.fetch_add(1, Ordering::Relaxed);
        }
        TreeSource::Files(paths) => {
            if ordered {
                process_files_ordered_batched(paths, &tx, process_tree, chunk_size, &progress);
            } else {
                process_files_unordered_batched(paths, tx, process_tree, &progress);
            }
        }
    });

    rx.into_iter().flatten()
}

#[derive(Debug, Clone)]
enum TreeSource {
    String(String),
    Files(Vec<PathBuf>),
}

/// A collection of dependency trees with parallel iteration support.
///
/// A `Treebank` is a lazy handle over one or more CoNLL-U sources (an in-memory
/// string, a single file, a list of files, or a glob pattern).  No I/O happens
/// at construction time; work begins when you call [`trees`](Self::trees),
/// [`search`](Self::search), or [`filter`](Self::filter).
///
/// File-level parallelism is handled automatically via rayon.  I/O and parse
/// errors are surfaced as `Err` items in the iterator rather than panicking, so
/// callers decide how to handle them.
///
/// # Constructors
///
/// | Method | Source |
/// |--------|--------|
/// | [`from_string`](Self::from_string) | In-memory CoNLL-U text |
/// | [`from_path`](Self::from_path)     | Single file |
/// | [`from_paths`](Self::from_paths)   | Explicit list of files |
/// | [`from_glob`](Self::from_glob)     | Glob pattern (e.g. `"data/*.conllu"`) |
///
/// The free function [`load`] is a short alias for [`from_glob`](Self::from_glob).
///
/// # Example
///
/// ```no_run
/// use treesearch::Treebank;
///
/// let tb = Treebank::from_glob("data/*.conllu").unwrap();
///
/// let count = tb.trees(true).filter_map(Result::ok).count();
/// println!("{count} trees");
/// ```
#[derive(Clone)]
pub struct Treebank {
    source: TreeSource,
}

impl Treebank {
    /// Create a treebank from an in-memory CoNLL-U string.
    ///
    /// Useful for testing or when corpus data is already loaded into memory.
    /// The string is cloned once; iteration is single-threaded (no files to parallelize).
    pub fn from_string(text: &str) -> Self {
        Self {
            source: TreeSource::String(text.to_string()),
        }
    }

    /// Create a treebank from a single file path.
    ///
    /// Accepts anything that implements `AsRef<Path>` (`&str`, `PathBuf`, etc.).
    /// For multiple files use [`from_paths`](Self::from_paths) or [`from_glob`](Self::from_glob).
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path_vec = vec![path.as_ref().to_path_buf()];
        Self::from_paths(path_vec)
    }

    /// Create a treebank from an explicit list of file paths.
    ///
    /// Files are processed in the order given. Use [`from_glob`](Self::from_glob)
    /// when a filename pattern is more convenient.
    pub fn from_paths(file_paths: Vec<PathBuf>) -> Self {
        Self {
            source: TreeSource::Files(file_paths),
        }
    }

    /// Create a treebank from a glob pattern (e.g. `"data/**/*.conllu.gz"`).
    ///
    /// Matching files are sorted before processing so results are deterministic
    /// across runs. Returns `Err` if the glob pattern itself is malformed.
    ///
    /// The free function [`load`] is a short alias for this method.
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
    /// for result in treebank.clone().trees(true) {
    ///     match result {
    ///         Ok(tree) => println!("Tree: {}", tree.words.len()),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    ///
    /// // Unordered iteration (faster for large corpora), ignoring errors
    /// for tree in treebank.trees(false).filter_map(Result::ok) {
    ///     println!("Tree: {}", tree.words.len());
    /// }
    /// ```
    pub fn trees(self, ordered: bool) -> impl Iterator<Item = Result<Tree, TreebankError>> {
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

    /// Search for all pattern matches across the treebank.
    ///
    /// Returns every [`Match`] found; a single tree can produce multiple matches if the
    /// pattern can be satisfied by different variable assignments within that tree.
    ///
    /// Accepts a pre-compiled [`Pattern`] or a raw query `&str`/`String` via [`IntoPattern`].
    ///
    /// # Arguments
    /// * `query` - Pattern or query string
    /// * `ordered` - If true (default), maintains file and tree order for deterministic results.
    ///   If false, matches may arrive in any order for better performance.
    ///
    /// # Errors
    ///
    /// Returns `Err(QueryError)` if `query` is a string that fails to compile.
    /// Infallible when `query` is a pre-compiled `Pattern`.
    ///
    /// # Examples
    /// ```no_run
    /// use treesearch::{Treebank, compile_query};
    ///
    /// let treebank = Treebank::from_glob("data/*.conllu").unwrap();
    ///
    /// // With pre-compiled pattern
    /// let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
    /// for result in treebank.clone().search(pattern, true).unwrap() { }
    ///
    /// // With inline string
    /// for result in treebank.search("MATCH { V [upos=\"VERB\"]; }", true).unwrap() { }
    /// ```
    pub fn search<Q: IntoPattern>(
        self,
        query: Q,
        ordered: bool,
    ) -> Result<impl Iterator<Item = Result<Match, TreebankError>>, QueryError> {
        self.search_with(query, ordered, Arc::default())
    }

    /// Like [`search`](Self::search), but reports progress to (and can be
    /// cancelled through) a shared [`Progress`].
    pub fn search_with<Q: IntoPattern>(
        self,
        query: Q,
        ordered: bool,
        progress: Arc<Progress>,
    ) -> Result<impl Iterator<Item = Result<Match, TreebankError>>, QueryError> {
        let pattern = query.into_pattern()?;
        Ok(build_parallel_iter_batched(
            self.source,
            ordered,
            4, // chunk_size for ordered mode
            progress,
            move |tree| search_tree(tree, &pattern).into_iter().map(Ok).collect(),
        ))
    }

    /// Filter to only trees that contain at least one match for a pattern.
    ///
    /// More efficient than [`search`](Self::search) when you need the matching trees
    /// but not the specific bindings — each tree is checked with early termination
    /// so the solver stops after the first match.
    ///
    /// Accepts a pre-compiled [`Pattern`] or a raw query `&str`/`String` via [`IntoPattern`].
    ///
    /// # Arguments
    /// * `query` - Pattern or query string
    /// * `ordered` - If true, maintains file and tree order. If false, may be faster.
    ///
    /// # Errors
    ///
    /// Returns `Err(QueryError)` if `query` is a string that fails to compile.
    /// Infallible when `query` is a pre-compiled `Pattern`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use treesearch::Treebank;
    ///
    /// let tb = Treebank::from_glob("data/*.conllu").unwrap();
    ///
    /// // Count sentences containing a passive construction
    /// let count = tb
    ///     .filter("MATCH { V []; V -[aux:pass]-> _; }", true)
    ///     .unwrap()
    ///     .filter_map(Result::ok)
    ///     .count();
    /// ```
    pub fn filter<Q: IntoPattern>(
        self,
        query: Q,
        ordered: bool,
    ) -> Result<impl Iterator<Item = Result<Tree, TreebankError>>, QueryError> {
        let pattern = query.into_pattern()?;
        Ok(build_parallel_iter_batched(
            self.source,
            ordered,
            4, // chunk_size for ordered mode
            Arc::default(),
            move |tree| {
                if tree_matches(&tree, &pattern) {
                    vec![Ok(tree)]
                } else {
                    vec![]
                }
            },
        ))
    }
}

/// Load a treebank from a glob pattern.
///
/// Convenience entry point equivalent to `Treebank::from_glob`.
///
/// # Example
/// ```no_run
/// use treesearch::load;
/// let tb = load("data/*.conllu")?;
/// # Ok::<_, glob::PatternError>(())
/// ```
pub fn load(glob_pattern: &str) -> Result<Treebank, glob::PatternError> {
    Treebank::from_glob(glob_pattern)
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
            .trees(true)
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
            .search(pattern, true)
            .unwrap()
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
            .search(pattern, true)
            .unwrap()
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
            .search(pattern, true)
            .unwrap()
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
            .search(pattern, true)
            .unwrap()
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
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(trees.len(), 3);

        // Pattern that matches only some trees (lemma="help")
        let pattern = compile_query("MATCH { V [lemma=\"help\"]; }").unwrap();
        let trees: Vec<_> = Treebank::from_string(THREE_VERB_CONLLU)
            .filter(pattern, true)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(trees.len(), 1);

        // Pattern that matches no trees
        let pattern = compile_query("MATCH { N [upos=\"NOUN\"]; }").unwrap();
        let trees: Vec<_> = Treebank::from_string(THREE_VERB_CONLLU)
            .filter(pattern, true)
            .unwrap()
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
                .trees(true)
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
                .trees(true)
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
                .search(pattern, true)
                .unwrap()
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
                .search(pattern, true)
                .unwrap()
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

            let results: Vec<_> = Treebank::from_paths(paths).trees(true).collect();

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
                .trees(true)
                .filter_map(Result::ok)
                .collect();
            let run2: Vec<_> = treebank
                .clone()
                .trees(true)
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
            let results: Vec<_> = treebank.trees(false).filter_map(Result::ok).collect();

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
                .search(pattern, true)
                .unwrap()
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
                .search(pattern, false)
                .unwrap()
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 2);
        }
    }

    #[test]
    fn test_progress_and_cancel() {
        let progress = Arc::new(Progress::default());
        let n = Treebank::from_string(TWO_TREE_CONLLU)
            .search_with("MATCH { V [upos=\"VERB\"]; }", true, progress.clone())
            .unwrap()
            .count();
        assert_eq!(n, 2);
        assert_eq!(progress.files_total.load(Ordering::Relaxed), 1);
        assert_eq!(progress.files_done.load(Ordering::Relaxed), 1);
        assert_eq!(progress.trees.load(Ordering::Relaxed), 2);

        let progress = Arc::new(Progress::default());
        progress.cancel();
        let n = Treebank::from_string(TWO_TREE_CONLLU)
            .search_with("MATCH { V [upos=\"VERB\"]; }", true, progress.clone())
            .unwrap()
            .count();
        assert_eq!(n, 0);
        assert_eq!(progress.trees.load(Ordering::Relaxed), 0);
    }
}
