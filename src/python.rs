//! Python bindings for treesearch
//!
//! This module provides PyO3-based Python bindings for the Rust core.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;

use crate::iterators::{MatchSet, TreeSet};
use crate::pattern::Pattern as RustPattern;
use crate::query::parse_query;
use crate::searcher::search;
use crate::tree::{Tree as RustTree, Word as RustWord};

/// A dependency tree representing a parsed sentence.
///
/// Contains words and their dependency relationships from a CoNLL-U file.
/// Access words using `get_word(id)`, navigate the tree structure, and
/// retrieve sentence text and metadata.
///
/// See API.md for complete documentation.
#[pyclass(name = "Tree")]
#[derive(Clone)]
pub struct PyTree {
    pub(crate) inner: Arc<RustTree>,
}

#[pymethods]
impl PyTree {
    /// Get a word by its ID (0-based index).
    ///
    /// Args:
    ///     id: Word index (0-based, not CoNLL-U token ID)
    ///
    /// Returns:
    ///     Word object if ID is valid, None otherwise
    ///
    /// Example:
    ///     word = tree.get_word(3)
    fn get_word(&self, id: usize) -> Option<PyWord> {
        self.inner.words.get(id).map(|word| PyWord {
            inner: word.clone(),
            tree: Arc::clone(&self.inner),
        })
    }

    /// Get the number of words in the tree.
    ///
    /// Returns:
    ///     Number of words (length of tree)
    fn __len__(&self) -> usize {
        self.inner.words.len()
    }

    /// Reconstructed sentence text from word forms.
    ///
    /// Returns:
    ///     Sentence text if available from CoNLL-U metadata, None otherwise
    #[getter]
    fn sentence_text(&self) -> Option<String> {
        self.inner.sentence_text.clone()
    }

    /// CoNLL-U metadata from sentence comments.
    ///
    /// Returns:
    ///     Dictionary of metadata key-value pairs from # comments in CoNLL-U
    #[getter]
    fn metadata(&self) -> std::collections::HashMap<String, String> {
        self.inner.metadata.clone()
    }

    /// Find the dependency path between two words.
    ///
    /// Traces the path through parent-child relationships from word x to word y.
    ///
    /// Args:
    ///     x: Starting word
    ///     y: Target word
    ///
    /// Returns:
    ///     List of words forming the path from x to y, or None if no path exists
    ///
    /// Example:
    ///     path = tree.find_path(verb, noun)
    fn find_path(&self, x: &PyWord, y: &PyWord) -> Option<Vec<PyWord>> {
        self.inner.find_path(&x.inner, &y.inner).map(|words| {
            words
                .into_iter()
                .map(|word| PyWord {
                    inner: word.clone(),
                    tree: Arc::clone(&self.inner),
                })
                .collect()
        })
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Tree({} words)", self.inner.words.len())
    }
}

/// A word (node) in a dependency tree.
///
/// Represents a single token with its linguistic properties (form, lemma, POS)
/// and dependency relationships (parent, children). Access properties via
/// attributes (word.form, word.lemma) and navigate the tree via methods
/// (word.parent(), word.children()).
///
/// See API.md for complete documentation.
#[pyclass(name = "Word")]
pub struct PyWord {
    inner: RustWord,
    tree: Arc<RustTree>,
}

#[pymethods]
impl PyWord {
    /// Word ID (0-based index in tree).
    #[getter]
    fn id(&self) -> usize {
        self.inner.id
    }

    /// Token ID from CoNLL-U file (1-based).
    #[getter]
    fn token_id(&self) -> usize {
        self.inner.token_id
    }

    /// Surface form of the word.
    #[getter]
    fn form(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.form)).to_string()
    }

    /// Lemma (dictionary form) of the word.
    #[getter]
    fn lemma(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.lemma)).to_string()
    }

    /// Universal POS tag (UPOS field in CoNLL-U).
    ///
    /// Example values: "VERB", "NOUN", "ADJ", "PRON"
    #[getter]
    fn pos(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.upos)).to_string()
    }

    /// Language-specific POS tag (XPOS field in CoNLL-U).
    ///
    /// Returns None if XPOS is "_" (unspecified).
    #[getter]
    fn xpos(&self) -> Option<String> {
        let resolved = self.tree.string_pool.resolve(self.inner.xpos);
        if *resolved == *b"_" {
            None
        } else {
            Some(String::from_utf8_lossy(&resolved).to_string())
        }
    }

    /// Dependency relation to parent.
    ///
    /// Example values: "nsubj", "obj", "root", "xcomp"
    #[getter]
    fn deprel(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.deprel)).to_string()
    }

    /// Head word ID (0-based index of parent word).
    ///
    /// Returns None for root words (which have no parent).
    #[getter]
    fn head(&self) -> Option<usize> {
        self.inner.head
    }

    /// Get the parent word in the dependency tree.
    ///
    /// Returns:
    ///     Parent Word object, or None for root words
    fn parent(&self) -> Option<PyWord> {
        self.inner.parent(&self.tree).map(|word| PyWord {
            inner: word.clone(),
            tree: Arc::clone(&self.tree),
        })
    }

    /// List of child word IDs (0-based indices).
    #[getter]
    fn children_ids(&self) -> Vec<usize> {
        self.inner.children.clone()
    }

    /// Get all child words (dependents) of this word.
    ///
    /// Returns:
    ///     List of child Word objects
    fn children(&self) -> Vec<PyWord> {
        self.inner
            .children(&self.tree)
            .into_iter()
            .map(|word| PyWord {
                inner: word.clone(),
                tree: Arc::clone(&self.tree),
            })
            .collect()
    }

    /// Get children with a specific dependency relation.
    ///
    /// Filters this word's children to only those with the given deprel.
    ///
    /// Args:
    ///     deprel: Dependency relation name (e.g., "nsubj", "obj", "conj")
    ///
    /// Returns:
    ///     List of child Word objects with the specified dependency relation
    ///
    /// Example:
    ///     objects = verb.children_by_deprel("obj")
    fn children_by_deprel(&self, deprel: &str) -> Vec<PyWord> {
        self.inner
            .children_by_deprel(&self.tree, deprel)
            .into_iter()
            .map(|word| PyWord {
                inner: word.clone(),
                tree: Arc::clone(&self.tree),
            })
            .collect()
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "Word(id={}, form='{}', lemma='{}', pos='{}', deprel='{}')",
            self.inner.id,
            self.form(),
            self.lemma(),
            self.pos(),
            self.deprel()
        )
    }
}

/// A compiled query pattern for tree matching.
///
/// Created by parse_query() and used with search functions. Patterns are
/// reusable and should be compiled once then used across multiple searches
/// for best performance. Contains the parsed query variables and constraints.
///
/// See API.md for query language syntax.
#[pyclass(name = "Pattern")]
#[derive(Clone)]
pub struct PyPattern {
    pub(crate) inner: RustPattern,
}

#[pymethods]
impl PyPattern {
    /// Number of variables in the pattern.
    ///
    /// Each variable in the query (e.g., "V", "Noun") counts as one variable.
    #[getter]
    fn n_vars(&self) -> usize {
        self.inner.n_vars
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Pattern({} vars)", self.inner.n_vars)
    }
}

/// Parse a query string into a compiled pattern.
///
/// Compiles a query into a Pattern object that can be reused for multiple
/// searches. Parse once and search many times for best performance.
///
/// Args:
///     query: Query string with variable declarations and constraints.
///            Example: 'V [upos="VERB"]; N [upos="NOUN"]; V -[nsubj]-> N;'
///
/// Returns:
///     Compiled Pattern object
///
/// Raises:
///     ValueError: If query syntax is invalid
///
/// See API.md for complete query language reference.
#[pyfunction(name = "parse_query")]
fn py_parse_query(query: &str) -> PyResult<PyPattern> {
    parse_query(query)
        .map(|inner| PyPattern { inner })
        .map_err(|e| PyValueError::new_err(format!("Query parse error: {}", e)))
}

/// Search a single tree for pattern matches.
///
/// Returns all matches found in the tree. Each match is a dictionary mapping
/// variable names from the query to word IDs in the tree.
///
/// Args:
///     tree: Tree to search
///     pattern: Compiled pattern from parse_query()
///
/// Returns:
///     List of match dictionaries. Each dict maps variable names to word IDs.
///     Example: [{"Verb": 3, "Noun": 5}, {"Verb": 7, "Noun": 2}]
///
/// Example:
///     for match in treesearch.search(tree, pattern):
///         verb = tree.get_word(match["Verb"])
#[pyfunction(name = "search")]
fn py_search(tree: &PyTree, pattern: &PyPattern) -> Vec<std::collections::HashMap<String, usize>> {
    search(&tree.inner, &pattern.inner).collect()
}

/// Iterator over trees from a single file
#[pyclass(unsendable)]
struct TreeIterator {
    inner: Option<crate::conllu::TreeIterator<std::io::BufReader<Box<dyn std::io::Read>>>>,
}

#[pymethods]
impl TreeIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyTree> {
        self.inner.as_mut().and_then(|iter| {
            iter.find_map(|result| {
                result.ok().map(|tree| PyTree {
                    inner: Arc::new(tree),
                })
            })
        })
    }
}

/// Read trees from a CoNLL-U file.
///
/// Opens a CoNLL-U file and returns an iterator over the trees (sentences).
/// Automatically detects and handles gzip-compressed files (.conllu.gz).
///
/// Args:
///     path: Path to CoNLL-U file (supports .conllu and .conllu.gz)
///
/// Returns:
///     Iterator yielding Tree objects
///
/// Raises:
///     ValueError: If file cannot be opened
///
/// Example:
///     for tree in treesearch.read_trees("corpus.conllu"):
///         print(tree.sentence_text)
#[pyfunction]
fn read_trees(path: &str) -> PyResult<TreeIterator> {
    use crate::conllu::TreeIterator as ConlluTreeIterator;

    ConlluTreeIterator::from_file(&PathBuf::from(path))
        .map(|inner| TreeIterator { inner: Some(inner) })
        .map_err(|e| PyValueError::new_err(format!("Failed to open file: {}", e)))
}

/// Iterator over matches from a single file
#[pyclass(unsendable)]
struct MatchIterator {
    inner: Option<Box<dyn Iterator<Item = (crate::tree::Tree, crate::searcher::Match)>>>,
}

#[pymethods]
impl MatchIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(PyTree, std::collections::HashMap<String, usize>)> {
        self.inner.as_mut().and_then(|iter| {
            iter.next().map(|(tree, m)| {
                (
                    PyTree {
                        inner: Arc::new(tree),
                    },
                    m,
                )
            })
        })
    }
}

/// Search a single CoNLL-U file for pattern matches.
///
/// More efficient than manually reading trees and searching, as it streams
/// results without loading the entire file into memory.
///
/// Args:
///     path: Path to CoNLL-U file (supports .conllu and .conllu.gz)
///     pattern: Compiled pattern from parse_query()
///
/// Returns:
///     Iterator yielding (tree, match) tuples, where match is a dict
///     mapping variable names to word IDs
///
/// Raises:
///     ValueError: If file cannot be opened
///
/// Example:
///     for tree, match in treesearch.search_file("corpus.conllu", pattern):
///         verb = tree.get_word(match["Verb"])
#[pyfunction]
fn search_file(path: &str, pattern: &PyPattern) -> PyResult<MatchIterator> {
    let match_set = MatchSet::from_file(&PathBuf::from(path), pattern.inner.clone());
    Ok(MatchIterator {
        inner: Some(match_set.into_iter()),
    })
}

/// Iterator over trees from multiple files (with optional parallel processing)
#[pyclass(unsendable)]
struct MultiFileTreeIterator {
    receiver: Receiver<PyTree>,
}

#[pymethods]
impl MultiFileTreeIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyTree> {
        self.receiver.recv().ok()
    }
}

/// Read trees from multiple CoNLL-U files matching a glob pattern.
///
/// Processes multiple files, optionally in parallel for better performance
/// on large corpora.
///
/// Args:
///     glob_pattern: Glob pattern to match files (e.g., "data/*.conllu")
///     parallel: Process files in parallel (default: True)
///
/// Returns:
///     Iterator yielding Tree objects from all matching files
///
/// Raises:
///     ValueError: If glob pattern is invalid
///
/// Example:
///     for tree in treesearch.read_trees_glob("corpus/*.conllu"):
///         print(tree.sentence_text)
#[pyfunction]
#[pyo3(signature = (glob_pattern, parallel=true))]
fn read_trees_glob(glob_pattern: &str, parallel: bool) -> PyResult<MultiFileTreeIterator> {
    let tree_set = TreeSet::from_glob(glob_pattern)
        .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;

    if parallel {
        Ok(create_parallel_tree_iterator(tree_set))
    } else {
        Ok(create_sequential_tree_iterator(tree_set))
    }
}

fn create_parallel_tree_iterator(tree_set: TreeSet) -> MultiFileTreeIterator {
    use rayon::prelude::*;

    let (sender, receiver) = channel();

    thread::spawn(move || {
        tree_set.into_par_iter().for_each(|tree| {
            let py_tree = PyTree {
                inner: Arc::new(tree),
            };
            let _ = sender.send(py_tree);
        });
    });

    MultiFileTreeIterator { receiver }
}

fn create_sequential_tree_iterator(tree_set: TreeSet) -> MultiFileTreeIterator {
    let (sender, receiver) = channel();

    thread::spawn(move || {
        for tree in tree_set {
            let py_tree = PyTree {
                inner: Arc::new(tree),
            };
            if sender.send(py_tree).is_err() {
                return;
            }
        }
    });

    MultiFileTreeIterator { receiver }
}

/// Iterator over matches from multiple files (with optional parallel processing)
#[pyclass(unsendable)]
struct MultiFileMatchIterator {
    receiver: Receiver<(PyTree, std::collections::HashMap<String, usize>)>,
}

#[pymethods]
impl MultiFileMatchIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(PyTree, std::collections::HashMap<String, usize>)> {
        self.receiver.recv().ok()
    }
}

/// Search multiple CoNLL-U files for pattern matches.
///
/// The most efficient way to search large corpora. Uses parallel processing
/// by default to maximize performance across multiple files.
///
/// Args:
///     glob_pattern: Glob pattern to match files (e.g., "data/*.conllu")
///     pattern: Compiled pattern from parse_query()
///     parallel: Process files in parallel (default: True)
///
/// Returns:
///     Iterator yielding (tree, match) tuples, where match is a dict
///     mapping variable names to word IDs
///
/// Raises:
///     ValueError: If glob pattern is invalid
///
/// Example:
///     pattern = treesearch.parse_query('V [upos="VERB"];')
///     for tree, match in treesearch.search_files("corpus/*.conllu", pattern):
///         verb = tree.get_word(match["V"])
#[pyfunction]
#[pyo3(signature = (glob_pattern, pattern, parallel=true))]
fn search_files(
    glob_pattern: &str,
    pattern: &PyPattern,
    parallel: bool,
) -> PyResult<MultiFileMatchIterator> {
    let match_set = MatchSet::from_glob(glob_pattern, pattern.inner.clone())
        .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;

    if parallel {
        Ok(create_parallel_match_iterator(match_set))
    } else {
        Ok(create_sequential_match_iterator(match_set))
    }
}

fn create_parallel_match_iterator(match_set: MatchSet) -> MultiFileMatchIterator {
    use rayon::prelude::*;

    let (sender, receiver) = channel();

    thread::spawn(move || {
        match_set.into_par_iter().for_each(|(tree, m)| {
            let result: (PyTree, std::collections::HashMap<String, usize>) = (
                PyTree {
                    inner: Arc::new(tree),
                },
                m,
            );
            let _ = sender.send(result);
        });
    });

    MultiFileMatchIterator { receiver }
}

fn create_sequential_match_iterator(match_set: MatchSet) -> MultiFileMatchIterator {
    let (sender, receiver) = channel();

    thread::spawn(move || {
        for (tree, m) in match_set {
            let result: (PyTree, std::collections::HashMap<String, usize>) = (
                PyTree {
                    inner: Arc::new(tree),
                },
                m,
            );
            if sender.send(result).is_err() {
                return;
            }
        }
    });

    MultiFileMatchIterator { receiver }
}

/// Python module initialization
#[pymodule]
fn treesearch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PyTree>()?;
    m.add_class::<PyWord>()?;
    m.add_class::<PyPattern>()?;
    m.add_class::<TreeIterator>()?;
    m.add_class::<MatchIterator>()?;
    m.add_class::<MultiFileTreeIterator>()?;
    m.add_class::<MultiFileMatchIterator>()?;

    // Functions
    m.add_function(wrap_pyfunction!(py_parse_query, m)?)?;
    m.add_function(wrap_pyfunction!(py_search, m)?)?;
    m.add_function(wrap_pyfunction!(read_trees, m)?)?;
    m.add_function(wrap_pyfunction!(search_file, m)?)?;
    m.add_function(wrap_pyfunction!(read_trees_glob, m)?)?;
    m.add_function(wrap_pyfunction!(search_files, m)?)?;

    Ok(())
}
