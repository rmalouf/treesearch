//! Python bindings for treesearch
//!
//! This module provides PyO3-based Python bindings for the Rust core.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;

use crate::iterators::{
    MatchIterator as RustMatchIterator, MultiFileMatchIterator as RustMultiFileMatchIterator,
    MultiFileTreeIterator as RustMultiFileTreeIterator,
};
use crate::pattern::Pattern as RustPattern;
use crate::query::parse_query;
use crate::searcher::search;
use crate::tree::{Tree as RustTree, Word as RustWord};

/// A dependency tree
#[pyclass(name = "Tree")]
#[derive(Clone)]
pub struct PyTree {
    pub(crate) inner: Arc<RustTree>,
}

#[pymethods]
impl PyTree {
    /// Get a word by ID
    fn get_word(&self, id: usize) -> Option<PyWord> {
        self.inner.words.get(id).map(|word| PyWord {
            inner: word.clone(),
            tree: Arc::clone(&self.inner),
        })
    }

    /// Get the number of words in the tree
    fn __len__(&self) -> usize {
        self.inner.words.len()
    }

    /// Get sentence text
    #[getter]
    fn sentence_text(&self) -> Option<String> {
        self.inner.sentence_text.clone()
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Tree({} words)", self.inner.words.len())
    }
}

/// A word in a dependency tree
#[pyclass(name = "Word")]
pub struct PyWord {
    inner: RustWord,
    tree: Arc<RustTree>,
}

#[pymethods]
impl PyWord {
    /// Word ID (0-based index)
    #[getter]
    fn id(&self) -> usize {
        self.inner.id
    }

    /// Token ID from CoNLL-U (1-based)
    #[getter]
    fn token_id(&self) -> usize {
        self.inner.token_id
    }

    /// Word form
    #[getter]
    fn form(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.form)).to_string()
    }

    /// Lemma
    #[getter]
    fn lemma(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.lemma)).to_string()
    }

    /// Universal POS tag
    #[getter]
    fn pos(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.upos)).to_string()
    }

    /// Language-specific POS tag
    #[getter]
    fn xpos(&self) -> Option<String> {
        self.inner
            .xpos
            .map(|sym| String::from_utf8_lossy(&self.tree.string_pool.resolve(sym)).to_string())
    }

    /// Dependency relation
    #[getter]
    fn deprel(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.deprel)).to_string()
    }

    /// Head word ID (parent)
    #[getter]
    fn head(&self) -> Option<usize> {
        self.inner.head
    }

    /// Get parent word
    fn parent(&self) -> Option<PyWord> {
        self.inner.parent(&self.tree).map(|word| PyWord {
            inner: word.clone(),
            tree: Arc::clone(&self.tree),
        })
    }

    /// Get child word IDs
    #[getter]
    fn children_ids(&self) -> Vec<usize> {
        self.inner.children.clone()
    }

    /// Get all children words
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

    /// Get children with a specific dependency relation
    ///
    /// Args:
    ///     deprel: The dependency relation name (e.g., "nsubj", "obj", "conj")
    ///
    /// Returns:
    ///     List of child words with the specified dependency relation
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

/// A compiled pattern for tree matching
#[pyclass(name = "Pattern")]
#[derive(Clone)]
pub struct PyPattern {
    pub(crate) inner: RustPattern,
}

#[pymethods]
impl PyPattern {
    /// Number of variables in the pattern
    #[getter]
    fn n_vars(&self) -> usize {
        self.inner.n_vars
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Pattern({} vars)", self.inner.n_vars)
    }
}

/// Parse a query string into a compiled pattern
///
/// Args:
///     query: Query string (e.g., "V [pos=\"VERB\"];")
///
/// Returns:
///     Compiled pattern
#[pyfunction(name = "parse_query")]
fn py_parse_query(query: &str) -> PyResult<PyPattern> {
    parse_query(query)
        .map(|inner| PyPattern { inner })
        .map_err(|e| PyValueError::new_err(format!("Query parse error: {}", e)))
}

/// Search a tree with a pattern
///
/// Args:
///     tree: The tree to search
///     pattern: The compiled pattern
///
/// Returns:
///     List of matches (each match is a list of word IDs)
#[pyfunction(name = "search")]
fn py_search(tree: &PyTree, pattern: &PyPattern) -> Vec<Vec<usize>> {
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

/// Read trees from a CoNLL-U file
///
/// Args:
///     path: Path to CoNLL-U file (supports .conllu and .conllu.gz)
///
/// Returns:
///     Iterator over trees
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
    inner: Option<RustMatchIterator>,
}

#[pymethods]
impl MatchIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(PyTree, Vec<usize>)> {
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

/// Search a single CoNLL-U file for pattern matches
///
/// Args:
///     path: Path to CoNLL-U file (supports .conllu and .conllu.gz)
///     pattern: Compiled pattern to search for
///
/// Returns:
///     Iterator over (tree, match) tuples, where match is a list of word IDs
#[pyfunction]
fn search_file(path: &str, pattern: &PyPattern) -> PyResult<MatchIterator> {
    RustMatchIterator::from_file(&PathBuf::from(path), pattern.inner.clone())
        .map(|inner| MatchIterator { inner: Some(inner) })
        .map_err(|e| PyValueError::new_err(format!("Failed to open file: {}", e)))
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

/// Read trees from multiple CoNLL-U files
///
/// Args:
///     glob_pattern: Glob pattern (e.g., "data/*.conllu")
///     parallel: Whether to process files in parallel (default: True)
///
/// Returns:
///     Iterator over trees
#[pyfunction]
#[pyo3(signature = (glob_pattern, parallel=true))]
fn read_trees_glob(glob_pattern: &str, parallel: bool) -> PyResult<MultiFileTreeIterator> {
    let iter = RustMultiFileTreeIterator::from_glob(glob_pattern)
        .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;

    if parallel {
        Ok(create_parallel_tree_iterator(iter))
    } else {
        Ok(create_sequential_tree_iterator(iter))
    }
}

fn create_parallel_tree_iterator(iter: RustMultiFileTreeIterator) -> MultiFileTreeIterator {
    use crate::conllu::TreeIterator;
    use rayon::prelude::*;

    let (sender, receiver) = channel();
    let file_paths = iter.file_paths;

    thread::spawn(move || {
        file_paths
            .into_par_iter()
            .flat_map_iter(|path| match TreeIterator::from_file(&path) {
                Ok(reader) => {
                    Box::new(reader.filter_map(Result::ok)) as Box<dyn Iterator<Item = _>>
                }
                Err(e) => {
                    eprintln!("Warning: Failed to open {:?}: {}", path, e);
                    Box::new(std::iter::empty())
                }
            })
            .for_each(|tree| {
                let py_tree = PyTree {
                    inner: Arc::new(tree),
                };
                let _ = sender.send(py_tree);
            });
    });

    MultiFileTreeIterator { receiver }
}

fn create_sequential_tree_iterator(iter: RustMultiFileTreeIterator) -> MultiFileTreeIterator {
    use crate::conllu::TreeIterator;

    let (sender, receiver) = channel();
    let file_paths = iter.file_paths;

    thread::spawn(move || {
        for path in file_paths {
            match TreeIterator::from_file(&path) {
                Ok(reader) => {
                    for tree in reader.filter_map(Result::ok) {
                        let py_tree = PyTree {
                            inner: Arc::new(tree),
                        };
                        if sender.send(py_tree).is_err() {
                            return;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to open {:?}: {}", path, e);
                }
            }
        }
    });

    MultiFileTreeIterator { receiver }
}

/// Iterator over matches from multiple files (with optional parallel processing)
#[pyclass(unsendable)]
struct MultiFileMatchIterator {
    receiver: Receiver<(PyTree, Vec<usize>)>,
}

#[pymethods]
impl MultiFileMatchIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(PyTree, Vec<usize>)> {
        self.receiver.recv().ok()
    }
}

/// Search multiple CoNLL-U files for pattern matches
///
/// Args:
///     glob_pattern: Glob pattern (e.g., "data/*.conllu")
///     pattern: Compiled pattern to search for
///     parallel: Whether to process files in parallel (default: True)
///
/// Returns:
///     Iterator over (tree, match) tuples, where match is a list of word IDs
#[pyfunction]
#[pyo3(signature = (glob_pattern, pattern, parallel=true))]
fn search_files(
    glob_pattern: &str,
    pattern: &PyPattern,
    parallel: bool,
) -> PyResult<MultiFileMatchIterator> {
    let iter = RustMultiFileMatchIterator::from_glob(glob_pattern, pattern.inner.clone())
        .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;

    if parallel {
        Ok(create_parallel_match_iterator(iter))
    } else {
        Ok(create_sequential_match_iterator(iter))
    }
}

fn create_parallel_match_iterator(iter: RustMultiFileMatchIterator) -> MultiFileMatchIterator {
    use rayon::prelude::*;

    let (sender, receiver) = channel();
    let file_paths = iter.file_paths;
    let pattern = iter.pattern;

    thread::spawn(move || {
        file_paths
            .into_par_iter()
            .flat_map_iter(
                move |path| match RustMatchIterator::from_file(&path, pattern.clone()) {
                    Ok(iter) => Box::new(iter) as Box<dyn Iterator<Item = _>>,
                    Err(e) => {
                        eprintln!("Warning: Failed to open {:?}: {}", path, e);
                        Box::new(std::iter::empty())
                    }
                },
            )
            .for_each(|(tree, m)| {
                let result = (
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

fn create_sequential_match_iterator(iter: RustMultiFileMatchIterator) -> MultiFileMatchIterator {
    let (sender, receiver) = channel();
    let file_paths = iter.file_paths;
    let pattern = iter.pattern;

    thread::spawn(move || {
        for path in file_paths {
            match RustMatchIterator::from_file(&path, pattern.clone()) {
                Ok(match_iter) => {
                    for (tree, m) in match_iter {
                        let result = (
                            PyTree {
                                inner: Arc::new(tree),
                            },
                            m,
                        );
                        if sender.send(result).is_err() {
                            return;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to open {:?}: {}", path, e);
                }
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
