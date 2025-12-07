//! Python bindings for treesearch
//!
//! This module provides PyO3-based Python bindings for the Rust core.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::iterators::Treebank;
use crate::pattern::Pattern as RustPattern;
use crate::query::parse_query;
use crate::searcher::search;
use crate::tree::{Tree as RustTree, Word as RustWord};

#[pyclass(name = "Tree")]
#[derive(Clone)]
pub struct PyTree {
    pub(crate) inner: Arc<RustTree>,
}

#[pymethods]
impl PyTree {
    fn get_word(&self, id: usize) -> Option<PyWord> {
        self.inner.words.get(id).map(|word| PyWord {
            inner: word.clone(),
            tree: Arc::clone(&self.inner),
        })
    }

    fn __len__(&self) -> usize {
        self.inner.words.len()
    }

    #[getter]
    fn sentence_text(&self) -> Option<String> {
        self.inner.sentence_text.clone()
    }

    #[getter]
    fn metadata(&self) -> std::collections::HashMap<String, String> {
        self.inner.metadata.clone()
    }

    fn __repr__(&self) -> String {
        format!("Tree({} words)", self.inner.words.len())
    }
}

#[pyclass(name = "Word")]
pub struct PyWord {
    inner: RustWord,
    tree: Arc<RustTree>,
}

#[pymethods]
impl PyWord {
    #[getter]
    fn id(&self) -> usize {
        self.inner.id
    }

    #[getter]
    fn token_id(&self) -> usize {
        self.inner.token_id
    }

    #[getter]
    fn form(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.form)).to_string()
    }

    #[getter]
    fn lemma(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.lemma)).to_string()
    }

    #[getter]
    fn pos(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.upos)).to_string()
    }

    #[getter]
    fn xpos(&self) -> Option<String> {
        let resolved = self.tree.string_pool.resolve(self.inner.xpos);
        if *resolved == *b"_" {
            None
        } else {
            Some(String::from_utf8_lossy(&resolved).to_string())
        }
    }

    #[getter]
    fn deprel(&self) -> String {
        String::from_utf8_lossy(&self.tree.string_pool.resolve(self.inner.deprel)).to_string()
    }

    #[getter]
    fn head(&self) -> Option<usize> {
        self.inner.head
    }

    fn parent(&self) -> Option<PyWord> {
        self.inner.parent(&self.tree).map(|word| PyWord {
            inner: word.clone(),
            tree: Arc::clone(&self.tree),
        })
    }

    #[getter]
    fn children_ids(&self) -> Vec<usize> {
        self.inner.children.clone()
    }

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

#[pyclass(name = "Pattern")]
#[derive(Clone)]
pub struct PyPattern {
    pub(crate) inner: RustPattern,
}

#[pymethods]
impl PyPattern {
    #[getter]
    fn n_vars(&self) -> usize {
        self.inner.n_vars
    }

    fn __repr__(&self) -> String {
        format!("Pattern({} vars)", self.inner.n_vars)
    }
}

/// A compiled query pattern for tree matching.
///
/// Created by parse_query() and used with search functions. Patterns are
/// reusable and should be compiled once then used across multiple searches
/// for best performance.
#[pyfunction(name = "parse_query")]
fn py_parse_query(query: &str) -> PyResult<PyPattern> {
    parse_query(query)
        .map(|inner| PyPattern { inner })
        .map_err(|e| PyValueError::new_err(format!("Query parse error: {}", e)))
}

/// A collection of dependency trees from files or strings.
///
/// Provides methods for iterating over trees and searching for patterns.
/// Supports multiple iterations by cloning internally.
#[pyclass(name = "Treebank")]
#[derive(Clone)]
pub struct PyTreebank {
    inner: Treebank,
}

#[pymethods]
impl PyTreebank {
    /// Create a Treebank from a CoNLL-U string.
    ///
    /// Args:
    ///     text: CoNLL-U formatted text
    ///
    /// Returns:
    ///     Treebank instance
    #[classmethod]
    fn from_string(_cls: &Bound<'_, pyo3::types::PyType>, text: &str) -> Self {
        PyTreebank {
            inner: Treebank::from_string(text),
        }
    }

    /// Create a Treebank from a CoNLL-U file.
    ///
    /// Automatically detects and handles gzip-compressed files (.conllu.gz).
    ///
    /// Args:
    ///     path: Path to CoNLL-U file
    ///
    /// Returns:
    ///     Treebank instance
    #[classmethod]
    fn from_file(_cls: &Bound<'_, pyo3::types::PyType>, path: &str) -> Self {
        PyTreebank {
            inner: Treebank::from_file(&PathBuf::from(path)),
        }
    }

    /// Create a Treebank from multiple files matching a glob pattern.
    ///
    /// Files are processed in sorted order for deterministic results.
    ///
    /// Args:
    ///     pattern: Glob pattern (e.g., "data/*.conllu")
    ///
    /// Returns:
    ///     Treebank instance
    ///
    /// Raises:
    ///     ValueError: If glob pattern is invalid
    #[classmethod]
    fn from_glob(_cls: &Bound<'_, pyo3::types::PyType>, pattern: &str) -> PyResult<Self> {
        Treebank::from_glob(pattern)
            .map(|inner| PyTreebank { inner })
            .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))
    }

    /// Iterate over all trees in the treebank.
    ///
    /// Can be called multiple times. Uses automatic parallel processing
    /// for multi-file treebanks.
    ///
    /// Returns:
    ///     Iterator over Tree objects
    fn trees(&self) -> PyTreeIterator {
        PyTreeIterator {
            inner: Box::new(self.inner.clone().tree_iter().map(Arc::new)),
        }
    }

    /// Search for pattern matches across all trees.
    ///
    /// Can be called multiple times. Uses automatic parallel processing
    /// for multi-file treebanks.
    ///
    /// Args:
    ///     pattern: Compiled pattern from parse_query()
    ///
    /// Returns:
    ///     Iterator over (tree, match) tuples
    fn matches(&self, pattern: &PyPattern) -> PyMatchIterator {
        PyMatchIterator {
            inner: Box::new(
                self.inner
                    .clone()
                    .match_iter(pattern.inner.clone())
                    .map(|m| (m.tree, m.bindings)),
            ),
        }
    }

    fn __repr__(&self) -> String {
        "Treebank()".to_string()
    }
}

/// Iterator over trees from a treebank.
#[pyclass(name = "TreeIterator", unsendable)]
struct PyTreeIterator {
    inner: Box<dyn Iterator<Item = Arc<RustTree>> + Send>,
}

#[pymethods]
impl PyTreeIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyTree> {
        self.inner.next().map(|tree| PyTree { inner: tree })
    }
}

/// Iterator over (tree, match) tuples from a pattern search.
#[pyclass(name = "MatchIterator", unsendable)]
struct PyMatchIterator {
    inner:
        Box<dyn Iterator<Item = (Arc<RustTree>, std::collections::HashMap<String, usize>)> + Send>,
}

#[pymethods]
impl PyMatchIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(PyTree, std::collections::HashMap<String, usize>)> {
        self.inner
            .next()
            .map(|(tree, bindings)| (PyTree { inner: tree }, bindings))
    }
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
///     List of match dictionaries
///
/// Example:
///     for match in treesearch.search(tree, pattern):
///         verb = tree.get_word(match["Verb"])
#[pyfunction(name = "search")]
fn py_search(tree: &PyTree, pattern: &PyPattern) -> Vec<std::collections::HashMap<String, usize>> {
    search((*tree.inner).clone(), &pattern.inner)
        .into_iter()
        .map(|m| m.bindings)
        .collect()
}

/// Read trees from a CoNLL-U file.
///
/// Convenience function wrapping Treebank.from_file().trees().
///
/// Args:
///     path: Path to CoNLL-U file
///
/// Returns:
///     Iterator over Tree objects
#[pyfunction]
fn read_trees(path: &str) -> PyTreeIterator {
    let treebank = Treebank::from_file(&PathBuf::from(path));
    PyTreeIterator {
        inner: Box::new(treebank.tree_iter().map(Arc::new)),
    }
}

/// Search a single CoNLL-U file for pattern matches.
///
/// Convenience function wrapping Treebank.from_file().matches(pattern).
///
/// Args:
///     path: Path to CoNLL-U file
///     pattern: Compiled pattern from parse_query()
///
/// Returns:
///     Iterator over (tree, match) tuples
#[pyfunction]
fn search_file(path: &str, pattern: &PyPattern) -> PyMatchIterator {
    let treebank = Treebank::from_file(&PathBuf::from(path));
    PyMatchIterator {
        inner: Box::new(
            treebank
                .match_iter(pattern.inner.clone())
                .map(|m| (m.tree, m.bindings)),
        ),
    }
}

/// Read trees from multiple CoNLL-U files matching a glob pattern.
///
/// Convenience function wrapping Treebank.from_glob(pattern).trees().
/// Uses automatic parallel processing.
///
/// Args:
///     glob_pattern: Glob pattern (e.g., "data/*.conllu")
///
/// Returns:
///     Iterator over Tree objects
///
/// Raises:
///     ValueError: If glob pattern is invalid
#[pyfunction]
fn read_trees_glob(glob_pattern: &str) -> PyResult<PyTreeIterator> {
    let treebank = Treebank::from_glob(glob_pattern)
        .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;
    Ok(PyTreeIterator {
        inner: Box::new(treebank.tree_iter().map(Arc::new)),
    })
}

/// Search multiple CoNLL-U files for pattern matches.
///
/// Convenience function wrapping Treebank.from_glob(pattern).matches(pattern).
/// Uses automatic parallel processing.
///
/// Args:
///     glob_pattern: Glob pattern (e.g., "data/*.conllu")
///     pattern: Compiled pattern from parse_query()
///
/// Returns:
///     Iterator over (tree, match) tuples
///
/// Raises:
///     ValueError: If glob pattern is invalid
#[pyfunction]
fn search_files(glob_pattern: &str, pattern: &PyPattern) -> PyResult<PyMatchIterator> {
    let treebank = Treebank::from_glob(glob_pattern)
        .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;
    Ok(PyMatchIterator {
        inner: Box::new(
            treebank
                .match_iter(pattern.inner.clone())
                .map(|m| (m.tree, m.bindings)),
        ),
    })
}

/// Python module initialization
#[pymodule]
fn treesearch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PyTree>()?;
    m.add_class::<PyWord>()?;
    m.add_class::<PyPattern>()?;
    m.add_class::<PyTreebank>()?;
    m.add_class::<PyTreeIterator>()?;
    m.add_class::<PyMatchIterator>()?;

    // Functions
    m.add_function(wrap_pyfunction!(py_parse_query, m)?)?;
    m.add_function(wrap_pyfunction!(py_search, m)?)?;
    m.add_function(wrap_pyfunction!(read_trees, m)?)?;
    m.add_function(wrap_pyfunction!(search_file, m)?)?;
    m.add_function(wrap_pyfunction!(read_trees_glob, m)?)?;
    m.add_function(wrap_pyfunction!(search_files, m)?)?;

    Ok(())
}
