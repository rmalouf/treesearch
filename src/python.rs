//! Python bindings for treesearch
//!
//! This module provides PyO3-based Python bindings for the Rust core.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;

use crate::iterators::{
    MatchIterator as RustMatchIterator, MultiFileMatchIterator as RustMultiFileMatchIterator,
    MultiFileTreeIterator as RustMultiFileTreeIterator,
};
use crate::pattern::Pattern as RustPattern;
use crate::query::parse_query;
use crate::searcher::{search, Match as RustMatch};
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

/// A pattern match result (list of word IDs)
#[pyclass(name = "Match")]
pub struct PyMatch {
    inner: RustMatch,
}

#[pymethods]
impl PyMatch {
    /// Get the word IDs in the match
    fn word_ids(&self) -> Vec<usize> {
        self.inner.clone()
    }

    /// Get a specific word ID by index
    fn __getitem__(&self, idx: usize) -> PyResult<usize> {
        self.inner
            .get(idx)
            .copied()
            .ok_or_else(|| PyValueError::new_err(format!("Index {} out of bounds", idx)))
    }

    /// Number of words in the match
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Match({:?})", self.inner)
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
    /// Parse a query string into a pattern
    ///
    /// Args:
    ///     query: Query string (e.g., "V [pos=\"VERB\"];")
    ///
    /// Returns:
    ///     Compiled pattern
    #[staticmethod]
    fn from_query(query: &str) -> PyResult<Self> {
        parse_query(query)
            .map(|inner| PyPattern { inner })
            .map_err(|e| PyValueError::new_err(format!("Query parse error: {}", e)))
    }

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

/// Search a tree with a pattern
///
/// Args:
///     tree: The tree to search
///     pattern: The compiled pattern
///
/// Returns:
///     Iterator over (Tree, Match) tuples
#[pyfunction(name = "search")]
fn py_search(tree: &PyTree, pattern: &PyPattern) -> Vec<PyMatch> {
    search(&tree.inner, &pattern.inner)
        .map(|m| PyMatch { inner: m })
        .collect()
}

/// Iterator over matches across multiple trees
#[pyclass(name = "MatchIterator", unsendable)]
pub struct PyMatchIterator {
    inner: Option<RustMatchIterator>,
}

#[pymethods]
impl PyMatchIterator {
    /// Create from a file and pattern
    ///
    /// Args:
    ///     path: Path to CoNLL-U file
    ///     pattern: Compiled pattern to search for
    #[staticmethod]
    fn from_file(path: &str, pattern: &PyPattern) -> PyResult<Self> {
        RustMatchIterator::from_file(&PathBuf::from(path), pattern.inner.clone())
            .map(|inner| PyMatchIterator { inner: Some(inner) })
            .map_err(|e| PyValueError::new_err(format!("Failed to open file: {}", e)))
    }

    /// Create from a string and pattern
    ///
    /// Args:
    ///     text: CoNLL-U formatted text
    ///     pattern: Compiled pattern to search for
    #[staticmethod]
    fn from_string(text: &str, pattern: &PyPattern) -> Self {
        PyMatchIterator {
            inner: Some(RustMatchIterator::from_string(text, pattern.inner.clone())),
        }
    }

    /// Iterate over matches
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Get next match
    fn __next__(&mut self) -> Option<(PyTree, PyMatch)> {
        self.inner.as_mut().and_then(|iter| {
            iter.next().map(|(tree, m)| {
                (
                    PyTree {
                        inner: Arc::new(tree),
                    },
                    PyMatch { inner: m },
                )
            })
        })
    }
}

/// Iterator over trees from multiple CoNLL-U files (parallel processing)
#[pyclass(name = "MultiFileTreeIterator", unsendable)]
pub struct PyMultiFileTreeIterator {
    receiver: Receiver<PyTree>,
}

#[pymethods]
impl PyMultiFileTreeIterator {
    /// Create from a glob pattern
    ///
    /// Args:
    ///     pattern: Glob pattern (e.g., "data/*.conllu")
    #[staticmethod]
    fn from_glob(pattern: &str) -> PyResult<Self> {
        let iter = RustMultiFileTreeIterator::from_glob(pattern)
            .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;

        Ok(Self::new(iter))
    }

    /// Create from explicit file paths
    ///
    /// Args:
    ///     paths: List of file paths
    #[staticmethod]
    fn from_paths(paths: Vec<String>) -> Self {
        let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let iter = RustMultiFileTreeIterator::from_paths(path_bufs);
        Self::new(iter)
    }

    /// Iterate over trees (parallel processing under the hood)
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Get next tree
    fn __next__(&mut self) -> Option<PyTree> {
        self.receiver.recv().ok()
    }
}

impl PyMultiFileTreeIterator {
    fn new(iter: RustMultiFileTreeIterator) -> Self {
        use crate::conllu::TreeIterator;
        use rayon::prelude::*;

        let (sender, receiver) = channel();

        // Extract file paths which are Send
        let file_paths = iter.file_paths;

        // Spawn thread to process files in parallel
        thread::spawn(move || {
            file_paths
                .into_par_iter()
                .flat_map_iter(|path| match TreeIterator::from_file(&path) {
                    Ok(reader) => Box::new(reader.filter_map(Result::ok))
                        as Box<dyn Iterator<Item = _>>,
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

        PyMultiFileTreeIterator { receiver }
    }
}

/// Iterator over matches across multiple CoNLL-U files (parallel processing)
#[pyclass(name = "MultiFileMatchIterator", unsendable)]
pub struct PyMultiFileMatchIterator {
    receiver: Receiver<(PyTree, PyMatch)>,
}

#[pymethods]
impl PyMultiFileMatchIterator {
    /// Create from a glob pattern and pattern
    ///
    /// Args:
    ///     glob_pattern: Glob pattern for files (e.g., "data/*.conllu")
    ///     pattern: Compiled pattern to search for
    #[staticmethod]
    fn from_glob(glob_pattern: &str, pattern: &PyPattern) -> PyResult<Self> {
        let iter = RustMultiFileMatchIterator::from_glob(glob_pattern, pattern.inner.clone())
            .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;

        Ok(Self::new(iter))
    }

    /// Create from explicit file paths and pattern
    ///
    /// Args:
    ///     paths: List of file paths
    ///     pattern: Compiled pattern to search for
    #[staticmethod]
    fn from_paths(paths: Vec<String>, pattern: &PyPattern) -> Self {
        let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let iter = RustMultiFileMatchIterator::from_paths(path_bufs, pattern.inner.clone());
        Self::new(iter)
    }

    /// Iterate over matches (parallel processing under the hood)
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Get next match
    fn __next__(&mut self) -> Option<(PyTree, PyMatch)> {
        self.receiver.recv().ok()
    }
}

impl PyMultiFileMatchIterator {
    fn new(iter: RustMultiFileMatchIterator) -> Self {
        use rayon::prelude::*;

        let (sender, receiver) = channel();

        // Extract file paths and pattern which are Send
        let file_paths = iter.file_paths;
        let pattern = iter.pattern;

        // Spawn thread to process files in parallel
        thread::spawn(move || {
            file_paths
                .into_par_iter()
                .flat_map_iter(move |path| {
                    match RustMatchIterator::from_file(&path, pattern.clone()) {
                        Ok(iter) => Box::new(iter) as Box<dyn Iterator<Item = _>>,
                        Err(e) => {
                            eprintln!("Warning: Failed to open {:?}: {}", path, e);
                            Box::new(std::iter::empty())
                        }
                    }
                })
                .for_each(|(tree, m)| {
                    let result = (
                        PyTree {
                            inner: Arc::new(tree),
                        },
                        PyMatch { inner: m },
                    );
                    let _ = sender.send(result);
                });
        });

        PyMultiFileMatchIterator { receiver }
    }
}

/// Python module initialization
#[pymodule]
fn treesearch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTree>()?;
    m.add_class::<PyWord>()?;
    m.add_class::<PyMatch>()?;
    m.add_class::<PyPattern>()?;
    m.add_class::<PyMatchIterator>()?;
    m.add_class::<PyMultiFileTreeIterator>()?;
    m.add_class::<PyMultiFileMatchIterator>()?;
    m.add_function(wrap_pyfunction!(py_search, m)?)?;
    Ok(())
}
