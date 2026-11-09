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
    /// Invalid CoNLL-U; `path` is `None` when parsing from a string
    #[error("{}{source}", .path.as_ref().map(|p| format!("{}: ", p.display())).unwrap_or_default())]
    Parse {
        path: Option<PathBuf>,
        source: ParseError,
    },

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

const BATCH_SIZE: usize = 500;
const CHANNEL_BUFFER_SIZE: usize = 100;

// Files processed in parallel per chunk in ordered mode. Smaller chunks improve
// load balancing for heterogeneous file sizes and bound how much is buffered.
const TREES_CHUNK_SIZE: usize = 2;
const SEARCH_CHUNK_SIZE: usize = 4;

type Batch<T> = Vec<Result<T, TreebankError>>;

/// Run every tree from `trees` through `process_tree`, handing each result to `emit`.
///
/// Stops early if the search is cancelled or `emit` returns `false`.
fn process_trees<T, R>(
    trees: impl Iterator<Item = Result<Tree, ParseError>>,
    path: Option<&Path>,
    process_tree: &impl Fn(Tree) -> R,
    progress: &Progress,
    mut emit: impl FnMut(Result<T, TreebankError>) -> bool,
) where
    R: IntoIterator<Item = T>,
{
    for result in trees {
        if progress.is_cancelled() {
            return;
        }
        match result {
            Ok(tree) => {
                progress.trees.fetch_add(1, Ordering::Relaxed);
                for item in process_tree(tree) {
                    if !emit(Ok(item)) {
                        return;
                    }
                }
            }
            Err(source) => {
                let path = path.map(Path::to_path_buf);
                if !emit(Err(TreebankError::Parse { path, source })) {
                    return;
                }
            }
        }
    }
}

/// Like [`process_trees`], sending the results over `tx` in batches of [`BATCH_SIZE`].
fn send_trees_batched<T, R>(
    trees: impl Iterator<Item = Result<Tree, ParseError>>,
    path: Option<&Path>,
    tx: &crossbeam_channel::Sender<Batch<T>>,
    process_tree: &impl Fn(Tree) -> R,
    progress: &Progress,
) where
    R: IntoIterator<Item = T>,
{
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    process_trees(trees, path, process_tree, progress, |item| {
        batch.push(item);
        batch.len() < BATCH_SIZE
            || tx
                .send(std::mem::replace(
                    &mut batch,
                    Vec::with_capacity(BATCH_SIZE),
                ))
                .is_ok()
    });
    if !batch.is_empty() {
        let _ = tx.send(batch);
    }
}

fn file_open_error(path: &Path, source: std::io::Error) -> TreebankError {
    TreebankError::FileOpen {
        path: path.to_path_buf(),
        source,
    }
}

fn process_files_ordered<T, R, F>(
    paths: &[PathBuf],
    tx: &crossbeam_channel::Sender<Batch<T>>,
    process_tree: &F,
    chunk_size: usize,
    progress: &Progress,
) where
    T: Send,
    R: IntoIterator<Item = T>,
    F: Fn(Tree) -> R + Sync,
{
    for chunk in paths.chunks(chunk_size) {
        if progress.is_cancelled() {
            return;
        }
        // Compute per-path results in parallel, keeping them grouped by path
        let per_path: Vec<Batch<T>> = chunk
            .par_iter()
            .map(|path| {
                let mut results = Vec::new();
                match TreeIterator::from_file(path) {
                    Ok(trees) => process_trees(trees, Some(path), process_tree, progress, |item| {
                        results.push(item);
                        true
                    }),
                    Err(e) => results.push(Err(file_open_error(path, e))),
                }
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

fn process_files_unordered<T, R, F>(
    paths: &[PathBuf],
    tx: &crossbeam_channel::Sender<Batch<T>>,
    process_tree: &F,
    progress: &Progress,
) where
    T: Send,
    R: IntoIterator<Item = T>,
    F: Fn(Tree) -> R + Sync,
{
    paths.par_iter().for_each(|path| {
        if progress.is_cancelled() {
            return;
        }
        match TreeIterator::from_file(path) {
            Ok(trees) => send_trees_batched(trees, Some(path), tx, process_tree, progress),
            Err(e) => {
                let _ = tx.send(vec![Err(file_open_error(path, e))]);
            }
        }
        progress.files_done.fetch_add(1, Ordering::Relaxed);
    });
}

/// Iterator over the results produced by the worker threads.
///
/// Dropping it before it is exhausted cancels the workers, so abandoning an
/// iteration early does not leave the rest of the treebank being processed.
struct ResultIter<T> {
    results: std::iter::Flatten<crossbeam_channel::IntoIter<Batch<T>>>,
    progress: Arc<Progress>,
    exhausted: bool,
}

impl<T> Iterator for ResultIter<T> {
    type Item = Result<T, TreebankError>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.results.next();
        self.exhausted = item.is_none();
        item
    }
}

impl<T> Drop for ResultIter<T> {
    fn drop(&mut self) {
        if !self.exhausted {
            self.progress.cancel();
        }
    }
}

fn build_parallel_iter<T, R, F>(
    source: TreeSource,
    ordered: bool,
    chunk_size: usize,
    progress: Arc<Progress>,
    process_tree: F,
) -> impl Iterator<Item = Result<T, TreebankError>>
where
    T: Send + 'static,
    R: IntoIterator<Item = T>,
    F: Fn(Tree) -> R + Send + Sync + 'static,
{
    let (tx, rx) = crossbeam_channel::bounded(CHANNEL_BUFFER_SIZE);

    let files_total = match &source {
        TreeSource::String(_) => 1,
        TreeSource::Files(paths) => paths.len(),
    };
    progress.files_total.store(files_total, Ordering::Relaxed);

    let worker_progress = Arc::clone(&progress);
    thread::spawn(move || {
        let progress = &*worker_progress;
        match source {
            TreeSource::String(text) => {
                let trees = TreeIterator::from_string(&text);
                send_trees_batched(trees, None, &tx, &process_tree, progress);
                progress.files_done.fetch_add(1, Ordering::Relaxed);
            }
            TreeSource::Files(paths) if ordered => {
                process_files_ordered(&paths, &tx, &process_tree, chunk_size, progress);
            }
            TreeSource::Files(paths) => {
                process_files_unordered(&paths, &tx, &process_tree, progress);
            }
        }
    });

    ResultIter {
        results: rx.into_iter().flatten(),
        progress,
        exhausted: false,
    }
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
    /// * `ordered` - If true, maintains file and tree order for deterministic results.
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
        build_parallel_iter(self.source, ordered, TREES_CHUNK_SIZE, Arc::default(), Some)
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
    /// * `ordered` - If true, maintains file and tree order for deterministic results.
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
    /// canceled through) a shared [`Progress`].
    pub fn search_with<Q: IntoPattern>(
        self,
        query: Q,
        ordered: bool,
        progress: Arc<Progress>,
    ) -> Result<impl Iterator<Item = Result<Match, TreebankError>>, QueryError> {
        let pattern = query.into_pattern()?;
        Ok(build_parallel_iter(
            self.source,
            ordered,
            SEARCH_CHUNK_SIZE,
            progress,
            move |tree| search_tree(tree, &pattern),
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
        Ok(build_parallel_iter(
            self.source,
            ordered,
            SEARCH_CHUNK_SIZE,
            Arc::default(),
            move |tree| tree_matches(&tree, &pattern).then_some(tree),
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
    use std::fs;
    use tempfile::{TempDir, tempdir};

    const VERB_QUERY: &str = r#"MATCH { V [upos="VERB"]; }"#;

    /// Two trees: "The dog runs." (one verb) and "John saw Mary leave" (two verbs)
    const TWO_TREES: &str = "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n\
                             2\tdog\tdog\tNOUN\tNN\t_\t3\tnsubj\t_\t_\n\
                             3\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n\
                             \n\
                             1\tJohn\tJohn\tPROPN\tNNP\t_\t2\tnsubj\t_\t_\n\
                             2\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                             3\tMary\tMary\tPROPN\tNNP\t_\t4\tnsubj\t_\t_\n\
                             4\tleave\tleave\tVERB\tVB\t_\t2\tccomp\t_\t_\n";

    fn one_word(lemma: &str) -> String {
        format!("1\t{lemma}s\t{lemma}\tVERB\tVBZ\t_\t0\troot\t_\t_\n")
    }

    /// Write single-tree files a.conllu, b.conllu, ... (plus a non-matching
    /// other.txt), one per lemma
    fn write_files(lemmas: &[&str]) -> (TempDir, Vec<PathBuf>) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("other.txt"), "ignored").unwrap();
        let paths = lemmas
            .iter()
            .zip('a'..)
            .map(|(lemma, name)| {
                let path = dir.path().join(format!("{name}.conllu"));
                fs::write(&path, one_word(lemma)).unwrap();
                path
            })
            .collect();
        (dir, paths)
    }

    fn first_lemma(tree: &Tree) -> String {
        String::from_utf8(tree.string_pool.resolve(tree.words[0].lemma).to_vec()).unwrap()
    }

    #[test]
    fn test_from_string() {
        let trees: Vec<_> = Treebank::from_string(TWO_TREES)
            .trees(true)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            trees.iter().map(|t| t.words.len()).collect::<Vec<_>>(),
            [3, 4]
        );

        // A tree can produce several matches
        let count = |query| {
            Treebank::from_string(TWO_TREES)
                .search(query, true)
                .unwrap()
                .map(Result::unwrap)
                .count()
        };
        assert_eq!(count(VERB_QUERY), 3);
        assert_eq!(
            count(r#"MATCH { V [lemma="see"]; S []; V -[nsubj]-> S; }"#),
            1
        );
        assert_eq!(count(r#"MATCH { N [upos="ADJ"]; }"#), 0);

        // filter yields each matching tree once
        let filter = |query| {
            Treebank::from_string(TWO_TREES)
                .filter(query, true)
                .unwrap()
                .map(|t| t.unwrap().words.len())
                .collect::<Vec<_>>()
        };
        assert_eq!(filter(VERB_QUERY), [3, 4]);
        assert_eq!(filter(r#"MATCH { N [upos="PROPN"]; }"#), [4]);
        assert!(filter(r#"MATCH { N [upos="ADJ"]; }"#).is_empty());

        // Queries can be strings or compiled patterns
        let pattern = compile_query(VERB_QUERY).unwrap();
        let n = Treebank::from_string(TWO_TREES)
            .search(pattern, true)
            .unwrap()
            .count();
        assert_eq!(n, 3);
        assert!(
            Treebank::from_string(TWO_TREES)
                .search("MATCH {", true)
                .is_err()
        );
        assert!(
            Treebank::from_string(TWO_TREES)
                .filter("MATCH {", true)
                .is_err()
        );
    }

    #[test]
    fn test_files() {
        let lemmas = ["run", "sleep", "walk"];
        let (dir, paths) = write_files(&lemmas);
        let glob = format!("{}/*.conllu", dir.path().display());

        for ordered in [true, false] {
            for tb in [Treebank::from_paths(paths.clone()), load(&glob).unwrap()] {
                let mut found: Vec<_> = tb
                    .clone()
                    .trees(ordered)
                    .map(|t| first_lemma(&t.unwrap()))
                    .collect();
                // Unordered iteration is complete, but in no particular order
                if !ordered {
                    found.sort();
                }
                assert_eq!(found, lemmas);

                let mut found: Vec<_> = tb
                    .search(VERB_QUERY, ordered)
                    .unwrap()
                    .map(|m| first_lemma(&m.unwrap().tree))
                    .collect();
                if !ordered {
                    found.sort();
                }
                assert_eq!(found, lemmas);
            }
        }

        assert_eq!(Treebank::from_path(&paths[1]).trees(true).count(), 1);
    }

    #[test]
    fn test_file_errors() {
        let (dir, mut paths) = write_files(&["run"]);
        let bad = dir.path().join("bad.conllu");
        fs::write(&bad, "1\truns\trun\tVERB\tVBZ\t_\tx\troot\t_\t_\n").unwrap();
        paths.extend([dir.path().join("missing.conllu"), bad, paths[0].clone()]);

        for ordered in [true, false] {
            let results: Vec<_> = Treebank::from_paths(paths.clone()).trees(ordered).collect();
            assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 2);

            let mut errors: Vec<_> = results
                .into_iter()
                .filter_map(|r| r.err().map(|e| e.to_string()))
                .collect();
            errors.sort();
            assert_eq!(errors.len(), 2);
            assert!(
                errors[0].contains("bad.conllu: Parse error at line 1"),
                "{errors:?}"
            );
            assert!(errors[1].contains("missing.conllu"), "{errors:?}");
        }
    }

    #[test]
    fn test_progress_and_cancel() {
        let search = |progress: &Arc<Progress>| {
            Treebank::from_string(TWO_TREES)
                .search_with(VERB_QUERY, true, progress.clone())
                .unwrap()
        };

        let progress = Arc::new(Progress::default());
        assert_eq!(search(&progress).count(), 3);
        assert_eq!(progress.files_total.load(Ordering::Relaxed), 1);
        assert_eq!(progress.files_done.load(Ordering::Relaxed), 1);
        assert_eq!(progress.trees.load(Ordering::Relaxed), 2);
        assert!(!progress.is_cancelled());

        // Dropping the iterator before it is exhausted cancels the workers
        let progress = Arc::new(Progress::default());
        let mut matches = search(&progress);
        assert!(matches.next().is_some());
        drop(matches);
        assert!(progress.is_cancelled());

        // A cancelled search yields nothing
        let progress = Arc::new(Progress::default());
        progress.cancel();
        assert_eq!(search(&progress).count(), 0);
        assert_eq!(progress.trees.load(Ordering::Relaxed), 0);
    }
}
