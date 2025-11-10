//! Python bindings for treesearch
//!
//! This module provides PyO3-based Python bindings for the Rust core.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

use crate::conllu::CoNLLUReader as RustCoNLLUReader;
use crate::searcher::search_query;
use crate::tree::{Node as RustNode, Tree as RustTree};
use crate::vm::Match as RustMatch;

/// A dependency tree
#[pyclass(name = "Tree")]
pub struct PyTree {
    inner: Arc<RustTree>,
}

#[pymethods]
impl PyTree {
    /// Create a new empty tree
    #[new]
    fn new() -> Self {
        PyTree {
            inner: Arc::new(RustTree::new()),
        }
    }

    /// Get a node by ID
    fn get_node(&self, id: usize) -> Option<PyNode> {
        self.inner.get_node(id).ok().map(|node| PyNode {
            inner: node.clone(),
            tree: Arc::clone(&self.inner),
        })
    }

    /// Get the number of nodes in the tree
    fn __len__(&self) -> usize {
        self.inner.nodes().len()
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Tree({} nodes)", self.inner.nodes().len())
    }
}

/// A node in a dependency tree
#[pyclass(name = "Node")]
pub struct PyNode {
    inner: RustNode,
    tree: Arc<RustTree>,
}

#[pymethods]
impl PyNode {
    /// Node ID
    #[getter]
    fn id(&self) -> usize {
        self.inner.id
    }

    /// Word form
    #[getter]
    fn form(&self) -> &str {
        &self.inner.form
    }

    /// Lemma
    #[getter]
    fn lemma(&self) -> &str {
        &self.inner.lemma
    }

    /// Universal POS tag
    #[getter]
    fn pos(&self) -> &str {
        &self.inner.pos
    }

    /// Language-specific POS tag
    #[getter]
    fn xpos(&self) -> Option<&str> {
        self.inner.xpos.as_deref()
    }

    /// Dependency relation
    #[getter]
    fn deprel(&self) -> &str {
        &self.inner.deprel
    }

    /// Linear position in sentence
    #[getter]
    fn position(&self) -> usize {
        self.inner.position
    }

    /// Get parent node ID
    fn parent_id(&self) -> PyResult<Option<usize>> {
        let Ok(parent_id) = self.tree.parent_id(self.inner.id) else {
            return Err(PyValueError::new_err(format!(
                "Failed to get parent of node {}",
                self.inner.id
            )));
        };
        Ok(parent_id)
    }

    /// Get parent node
    fn parent(&self) -> Option<PyNode> {
        self.inner.parent(&self.tree).map(|node| PyNode {
            inner: node.clone(),
            tree: Arc::clone(&self.tree),
        })
    }

    /// Get child node IDs
    fn children_ids(&self) -> PyResult<Vec<usize>> {
        let Ok(children_ids) = self.tree.children_ids(self.inner.id) else {
            return Err(PyValueError::new_err(format!(
                "Failed to get children of node {}",
                self.inner.id
            )));
        };
        Ok(children_ids)
    }

    /// Get all children nodes
    fn children(&self) -> Vec<PyNode> {
        self.inner
            .children(&self.tree)
            .iter()
            .map(|&node| PyNode {
                inner: node.clone(),
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
    ///     List of child nodes with the specified dependency relation
    fn children_by_deprel(&self, deprel: &str) -> Vec<PyNode> {
        self.inner
            .children_by_deprel(&self.tree, deprel)
            .into_iter()
            .map(|node| PyNode {
                inner: node.clone(),
                tree: Arc::clone(&self.tree),
            })
            .collect()
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "Node(id={}, form='{}', lemma='{}', pos='{}', deprel='{}')",
            self.inner.id, self.inner.form, self.inner.lemma, self.inner.pos, self.inner.deprel
        )
    }
}

/// A pattern match result
#[pyclass(name = "Match")]
pub struct PyMatch {
    inner: RustMatch,
    tree: Arc<RustTree>,
}

#[pymethods]
impl PyMatch {
    /// Get the node ID bound to a variable name
    fn get(&self, name: &str) -> Option<usize> {
        self.inner.get(name)
    }

    /// Get the node bound to a variable name
    fn get_node(&self, name: &str) -> Option<PyNode> {
        self.inner.get(name).and_then(|id| {
            self.tree.get_node(id).ok().map(|node| PyNode {
                inner: node.clone(),
                tree: Arc::clone(&self.tree),
            })
        })
    }

    /// Get all variable bindings as a dictionary
    fn bindings(&self) -> HashMap<String, usize> {
        self.inner
            .iter_named()
            .map(|(name, id)| (name.to_string(), id))
            .collect()
    }

    /// Get all variable bindings with nodes as a dictionary
    fn nodes(&self) -> HashMap<String, PyNode> {
        self.inner
            .iter_named()
            .filter_map(|(name, id)| {
                self.tree.get_node(id).ok().map(|node| {
                    (
                        name.to_string(),
                        PyNode {
                            inner: node.clone(),
                            tree: Arc::clone(&self.tree),
                        },
                    )
                })
            })
            .collect()
    }

    /// String representation
    fn __repr__(&self) -> String {
        let bindings: Vec<String> = self
            .inner
            .iter_named()
            .map(|(name, id)| format!("{}={}", name, id))
            .collect();
        format!("Match({})", bindings.join(", "))
    }
}

/// Search a tree with a query string
///
/// Args:
///     tree: The tree to search
///     query: The query string (e.g., "Verb [pos='VERB']; Noun [pos='NOUN']; Verb -[nsubj]-> Noun;")
///
/// Returns:
///     List of match results
#[pyfunction(name = "search_query")]
fn py_search_query(tree: &PyTree, query: &str) -> PyResult<Vec<PyMatch>> {
    let Ok(matches) = search_query(&tree.inner, query) else {
        return Err(PyValueError::new_err(format!("Search error for query: {}", query)));
    };

    Ok(matches
        .map(|m| PyMatch {
            inner: m,
            tree: Arc::clone(&tree.inner),
        })
        .collect())
}

/// CoNLL-U file reader
#[pyclass(name = "CoNLLUReader")]
pub struct PyCoNLLUReader {
    inner: RustCoNLLUReader<BufReader<File>>,
}

#[pymethods]
impl PyCoNLLUReader {
    /// Create a reader from a file path
    ///
    /// Args:
    ///     path: Path to the CoNLL-U file
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let Ok(reader) = RustCoNLLUReader::from_file(&PathBuf::from(path)) else {
            return Err(PyValueError::new_err(format!("Failed to open file: {}", path)));
        };
        Ok(PyCoNLLUReader { inner: reader })
    }

    /// Iterate over trees in the file
    fn __iter__(slf: PyRef<Self>) -> PyResult<PyCoNLLUReaderIterator> {
        // We can't clone the reader, so we need to return self
        // The iterator will consume from the same reader
        Ok(PyCoNLLUReaderIterator {
            reader: slf.into(),
        })
    }
}

/// Iterator for CoNLLU reader
#[pyclass]
struct PyCoNLLUReaderIterator {
    reader: Py<PyCoNLLUReader>,
}

#[pymethods]
impl PyCoNLLUReaderIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self, py: Python) -> PyResult<Option<PyTree>> {
        let mut reader = self.reader.borrow_mut(py);
        match reader.inner.next() {
            Some(Ok(tree)) => Ok(Some(PyTree { inner: Arc::new(tree) })),
            Some(Err(e)) => Err(PyValueError::new_err(format!("Parse error: {}", e))),
            None => Ok(None),
        }
    }
}

/// Python module initialization
#[pymodule]
fn treesearch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTree>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<PyMatch>()?;
    m.add_class::<PyCoNLLUReader>()?;
    m.add_function(wrap_pyfunction!(py_search_query, m)?)?;
    Ok(())
}
