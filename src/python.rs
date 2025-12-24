//! Python bindings for treesearch
//!
//! This module provides PyO3-based Python bindings for the Rust core.

use pyo3::exceptions::{PyIOError, PyIndexError, PyValueError};
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::iterators::{Treebank, TreebankError};
use crate::pattern::Pattern as RustPattern;
use crate::query::compile_query;
use crate::searcher::search_tree;
use crate::tree::{Tree as RustTree, Word as RustWord};

/// Convert TreebankError to Python exception
impl From<TreebankError> for PyErr {
    fn from(err: TreebankError) -> PyErr {
        match err {
            TreebankError::Io(e) => PyIOError::new_err(e.to_string()),
            TreebankError::Parse(e) => PyValueError::new_err(format!("Parse error: {}", e)),
            TreebankError::FileOpen { path, source } => PyIOError::new_err(format!(
                "Failed to open file {}: {}",
                path.display(),
                source
            )),
        }
    }
}

#[pyclass(name = "Tree")]
#[derive(Clone)]
pub struct PyTree {
    pub(crate) inner: Arc<RustTree>,
}

#[pymethods]
impl PyTree {
    fn word(&self, id: usize) -> PyResult<PyWord> {
        self.inner
            .words
            .get(id)
            .map(|word| PyWord {
                inner: word.clone(),
                tree: Arc::clone(&self.inner),
            })
            .ok_or_else(|| PyIndexError::new_err(format!("word index out of range: {}", id)))
    }

    fn __getitem__(&self, id: usize) -> PyResult<PyWord> {
        self.word(id)
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
        let n = self.inner.words.len();
        if n == 0 {
            return "<Tree (empty)>".to_string();
        }

        let num_to_show = n.min(3);
        let words: Vec<String> = self
            .inner
            .words
            .iter()
            .take(num_to_show)
            .map(|w| String::from_utf8_lossy(&self.inner.string_pool.resolve(w.form)).to_string())
            .collect();

        if n > 3 {
            format!("<Tree len={} words='{} ...'>", n, words.join(" "))
        } else {
            format!("<Tree len={} words='{}'>", n, words.join(" "))
        }
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
    fn upos(&self) -> String {
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

    #[getter]
    fn feats(&self) -> std::collections::HashMap<String, String> {
        self.inner
            .feats
            .iter()
            .map(|(k, v)| {
                (
                    String::from_utf8_lossy(&self.tree.string_pool.resolve(*k)).to_string(),
                    String::from_utf8_lossy(&self.tree.string_pool.resolve(*v)).to_string(),
                )
            })
            .collect()
    }

    #[getter]
    fn misc(&self) -> std::collections::HashMap<String, String> {
        self.inner
            .misc
            .iter()
            .map(|(k, v)| {
                (
                    String::from_utf8_lossy(&self.tree.string_pool.resolve(*k)).to_string(),
                    String::from_utf8_lossy(&self.tree.string_pool.resolve(*v)).to_string(),
                )
            })
            .collect()
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

    // TODO: add xpos and head to these (but they're optional)
    fn __repr__(&self) -> String {
        format!(
            "<Word id={} form='{}' lemma='{}' upos='{}' deprel='{}'>",
            self.inner.id,
            self.form(),
            self.lemma(),
            self.upos(),
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
    fn __repr__(&self) -> String {
        format!("Pattern({} vars)", self.inner.n_vars)
    }
}

/// A compiled query pattern for tree matching.
///
/// Created by parse_query() and used with search functions. Patterns are
/// reusable and should be compiled once then used across multiple searches
/// for best performance.
#[pyfunction(name = "compile_query")]
fn py_compile_query(query: &str) -> PyResult<PyPattern> {
    compile_query(query)
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
    fn from_file(_cls: &Bound<'_, pyo3::types::PyType>, file_path: &str) -> Self {
        PyTreebank {
            inner: Treebank::from_path(&PathBuf::from(file_path)),
        }
    }

    /// Create a Treebank from multiple file paths.
    ///
    /// Args:
    ///     paths: List of paths to CoNLL-U files
    ///
    /// Returns:
    ///     Treebank instance
    ///
    /// Example:
    ///     >>> tb = Treebank.from_paths(["file1.conllu", "file2.conllu"])
    ///     >>> for tree in tb.trees():
    ///     ...     print(tree)
    #[classmethod]
    fn from_files(_cls: &Bound<'_, pyo3::types::PyType>, file_paths: Vec<String>) -> Self {
        let path_bufs: Vec<PathBuf> = file_paths.iter().map(PathBuf::from).collect();
        PyTreebank {
            inner: Treebank::from_paths(path_bufs),
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
    // #[classmethod]
    // fn from_glob(_cls: &Bound<'_, pyo3::types::PyType>, pattern: &str) -> PyResult<Self> {
    //     Treebank::from_glob(pattern)
    //         .map(|inner| PyTreebank { inner })
    //         .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))
    // }

    /// Iterate over all trees in the treebank.
    ///
    /// Can be called multiple times. Uses automatic parallel processing
    /// for multi-file treebanks.
    ///
    /// Args:
    ///     ordered: If True (default), trees are returned in deterministic order.
    ///              If False, trees may arrive in any order for better performance.
    ///
    /// Returns:
    ///     Iterator over Tree objects
    ///
    /// Example:
    ///     >>> tb = Treebank.from_glob("data/*.conllu")
    ///     >>> for tree in tb.trees(ordered=True):  # deterministic
    ///     ...     print(tree)
    ///     >>> for tree in tb.trees(ordered=False):  # faster
    ///     ...     print(tree)
    #[pyo3(signature = (ordered=true))]
    fn trees(&self, ordered: bool) -> PyTreeIterator {
        PyTreeIterator {
            inner: Box::new(
                self.inner
                    .clone()
                    .tree_iter(ordered)
                    .map(|result| result.map(Arc::new)),
            ),
        }
    }

    /// Search for pattern matches across all trees.
    ///
    /// Can be called multiple times. Uses automatic parallel processing
    /// for multi-file treebanks.
    ///
    /// Args:
    ///     pattern: Compiled pattern from parse_query()
    ///     ordered: If True (default), matches are returned in deterministic order.
    ///              If False, matches may arrive in any order for better performance.
    ///
    /// Returns:
    ///     Iterator over (tree, match) tuples
    ///
    /// Example:
    ///     >>> tb = Treebank.from_glob("data/*.conllu")
    ///     >>> pattern = parse_query("MATCH { V [upos='VERB']; }")
    ///     >>> for tree, match in tb.matches(pattern, ordered=True):
    ///     ...     print(match)
    #[pyo3(signature = (pattern, ordered=true))]
    fn search(&self, pattern: &PyPattern, ordered: bool) -> PyMatchIterator {
        PyMatchIterator {
            inner: Box::new(
                self.inner
                    .clone()
                    .match_iter(pattern.inner.clone(), ordered)
                    .map(|result| result.map(|m| (m.tree, m.bindings))),
            ),
        }
    }

    // TODO: make this more interesting (number of files? start of string?)
    fn __repr__(&self) -> String {
        "<Treebank>".to_string()
    }
}

/// Iterator over trees from a treebank.
#[pyclass(name = "TreeIterator", unsendable)]
struct PyTreeIterator {
    inner: Box<dyn Iterator<Item = Result<Arc<RustTree>, TreebankError>> + Send>,
}

#[pymethods]
impl PyTreeIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<PyTree>> {
        match self.inner.next() {
            Some(Ok(tree)) => Ok(Some(PyTree { inner: tree })),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}

/// Iterator over (tree, match) tuples from a pattern search.
#[pyclass(name = "MatchIterator", unsendable)]
struct PyMatchIterator {
    inner: Box<
        dyn Iterator<
                Item = Result<
                    (Arc<RustTree>, std::collections::HashMap<String, usize>),
                    TreebankError,
                >,
            > + Send,
    >,
}

#[pymethods]
impl PyMatchIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<(PyTree, std::collections::HashMap<String, usize>)>> {
        match self.inner.next() {
            Some(Ok((tree, bindings))) => Ok(Some((PyTree { inner: tree }, bindings))),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}

/// Search a list of trees for pattern matches.
///
/// Returns an iterator over (tree, match) tuples for all matches found across
/// all trees. Each match is a dictionary mapping variable names from the query
/// to word IDs in the tree.
///
/// Args:
///     trees: List of trees to search
///     pattern: Compiled pattern from parse_query()
///
/// Returns:
///     Iterator over (tree, match) tuples
///
/// Example:
///     for tree, match in treesearch.search_trees([tree1, tree2], pattern):
///         print(match)
#[pyfunction]
fn py_search_trees(trees: Vec<PyTree>, pattern: &PyPattern) -> PyMatchIterator {
    let results: Vec<_> = trees
        .into_iter()
        .flat_map(|tree| {
            let tree_arc = tree.inner.clone();
            search_tree((*tree_arc).clone(), &pattern.inner)
                .into_iter()
                .map(move |m| Ok((tree_arc.clone(), m.bindings)))
        })
        .collect();

    PyMatchIterator {
        inner: Box::new(results.into_iter()),
    }
}

/*
/// Search a single CoNLL-U file for pattern matches.
///
/// Convenience function wrapping Treebank.from_file().matches(pattern).
///
/// Args:
///     path: Path to CoNLL-U file
///     pattern: Compiled pattern from parse_query()
///     ordered: If True (default), matches are returned in deterministic order.
///              If False, matches may arrive in any order for better performance.
///
/// Returns:
///     Iterator over (tree, match) tuples
#[pyfunction]
#[pyo3(signature = (path, pattern, ordered=true))]
fn search_file(path: &str, pattern: &PyPattern, ordered: bool) -> PyMatchIterator {
    let treebank = Treebank::from_path(&PathBuf::from(path));
    PyMatchIterator {
        inner: Box::new(
            treebank
                .match_iter(pattern.inner.clone(), ordered)
                .map(|result| result.map(|m| (m.tree, m.bindings))),
        ),
    }
}
*/

// /// Read trees from multiple CoNLL-U files matching a glob pattern.
// ///
// /// Convenience function wrapping Treebank.from_glob(pattern).trees().
// /// Uses automatic parallel processing.
// ///
// /// Args:
// ///     glob_pattern: Glob pattern (e.g., "data/*.conllu")
// ///     ordered: If True (default), trees are returned in deterministic order.
// ///              If False, trees may arrive in any order for better performance.
// ///
// /// Returns:
// ///     Iterator over Tree objects
// ///
// /// Raises:
// ///     ValueError: If glob pattern is invalid
// #[pyfunction]
// #[pyo3(signature = (glob_pattern, ordered=true))]
// fn read_trees_glob(glob_pattern: &str, ordered: bool) -> PyResult<PyTreeIterator> {
//     let treebank = Treebank::from_glob(glob_pattern)
//         .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;
//     Ok(PyTreeIterator {
//         inner: Box::new(treebank.tree_iter(ordered).map(|result| result.map(Arc::new))),
//     })
// }

// /// Search multiple CoNLL-U files for pattern matches.
// ///
// /// Convenience function wrapping Treebank.from_glob(pattern).matches(pattern).
// /// Uses automatic parallel processing.
// ///
// /// Args:
// ///     glob_pattern: Glob pattern (e.g., "data/*.conllu")
// ///     pattern: Compiled pattern from parse_query()
// ///     ordered: If True (default), matches are returned in deterministic order.
// ///              If False, matches may arrive in any order for better performance.
// ///
// /// Returns:
// ///     Iterator over (tree, match) tuples
// ///
// /// Raises:
// ///     ValueError: If glob pattern is invalid
// #[pyfunction]
// #[pyo3(signature = (glob_pattern, pattern, ordered=true))]
// fn search_files(
//     glob_pattern: &str,
//     pattern: &PyPattern,
//     ordered: bool,
// ) -> PyResult<PyMatchIterator> {
//     let treebank = Treebank::from_glob(glob_pattern)
//         .map_err(|e| PyValueError::new_err(format!("Glob pattern error: {}", e)))?;
//     Ok(PyMatchIterator {
//         inner: Box::new(
//             treebank
//                 .match_iter(pattern.inner.clone(), ordered)
//                 .map(|result| result.map(|m| (m.tree, m.bindings))),
//         ),
//     })
// }

#[pyfunction]
fn __version__() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn treesearch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTree>()?;
    m.add_class::<PyWord>()?;
    m.add_class::<PyPattern>()?;
    m.add_class::<PyTreebank>()?;
    m.add_class::<PyTreeIterator>()?;
    m.add_class::<PyMatchIterator>()?;

    m.add_function(wrap_pyfunction!(py_compile_query, m)?)?;
    m.add_function(wrap_pyfunction!(py_search_trees, m)?)?;
    //m.add_function(wrap_pyfunction!(search_file, m)?)?;
    //m.add_function(wrap_pyfunction!(read_trees_glob, m)?)?;
    //m.add_function(wrap_pyfunction!(search_files, m)?)?;

    Ok(())
}
