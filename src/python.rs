//! Python bindings for treesearch
//!
//! This module provides PyO3-based Python bindings for the Rust core.
//!
//! We use `py.detach()` to release the GIL (in GIL-enabled Python) or detach from
//! the Python thread state (in free-threaded Python) during expensive Rust operations,
//! allowing better parallel performance.

use pyo3::exceptions::{PyIOError, PyImportError, PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyType};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use crate::bytes::Sym;
use crate::iterators::{IntoPattern, Treebank, TreebankError};
use crate::pattern::Pattern as RustPattern;
use crate::query::{QueryError, compile_query};
use crate::searcher::{Bindings, find_all_bindings};
use crate::tree::{Features, Tree as RustTree, Word as RustWord};

/// Convert TreebankError to Python exception
impl From<TreebankError> for PyErr {
    fn from(err: TreebankError) -> PyErr {
        match err {
            TreebankError::Parse(e) => PyValueError::new_err(format!("Parse error: {}", e)),
            TreebankError::FileOpen { path, source } => PyIOError::new_err(format!(
                "Failed to open file {}: {}",
                path.display(),
                source
            )),
        }
    }
}

/// Convert QueryError to Python exception
impl From<QueryError> for PyErr {
    fn from(err: QueryError) -> PyErr {
        PyValueError::new_err(format!("Query parse error: {}", err))
    }
}

/// Resolve an interned symbol to an owned string
fn sym_str(tree: &RustTree, sym: Sym) -> String {
    String::from_utf8_lossy(&tree.string_pool.resolve(sym)).into_owned()
}

/// Resolve interned key-value pairs to a string map
fn feats_map(tree: &RustTree, feats: &Features) -> HashMap<String, String> {
    feats
        .iter()
        .map(|&(k, v)| (sym_str(tree, k), sym_str(tree, v)))
        .collect()
}

type ResultIter<T> = Box<dyn Iterator<Item = Result<T, TreebankError>> + Send>;

/// Advance a result iterator with the GIL released (or, in free-threaded Python,
/// detached from the thread state) during the expensive parsing/matching work
fn next_detached<T: Send>(py: Python<'_>, iter: &mut ResultIter<T>) -> PyResult<Option<T>> {
    Ok(py.detach(|| iter.next()).transpose()?)
}

#[pymodule]
mod treesearch {
    use super::*;

    /// A dependency tree (one sentence).
    ///
    /// Words are addressed by 0-based ID: `tree.word(id)` or `tree[id]`.
    /// `len(tree)` is the number of words, and iterating a tree yields its words.
    #[pyclass(name = "Tree", from_py_object)]
    #[derive(Clone)]
    pub struct PyTree {
        pub(crate) inner: Arc<RustTree>,
    }

    impl PyTree {
        fn get(&self, id: isize, index: usize) -> PyResult<PyWord> {
            if index < self.inner.words.len() {
                Ok(PyWord {
                    tree: Arc::clone(&self.inner),
                    id: index,
                })
            } else {
                Err(PyIndexError::new_err(format!(
                    "word index out of range: {}",
                    id
                )))
            }
        }
    }

    #[pymethods]
    impl PyTree {
        /// Get a word by ID (0-based index).
        ///
        /// Raises:
        ///     IndexError: If the ID is out of range
        fn word(&self, id: isize) -> PyResult<PyWord> {
            self.get(id, usize::try_from(id).unwrap_or(usize::MAX))
        }

        /// Get a word by position; negative positions count from the end.
        fn __getitem__(&self, id: isize) -> PyResult<PyWord> {
            let index = if id < 0 {
                id + self.inner.words.len() as isize
            } else {
                id
            };
            self.get(id, usize::try_from(index).unwrap_or(usize::MAX))
        }

        fn __len__(&self) -> usize {
            self.inner.words.len()
        }

        /// Sentence text from the `# text = ...` comment, if present.
        #[getter]
        fn sentence_text(&self) -> Option<String> {
            self.inner.sentence_text.clone()
        }

        /// Other metadata from CoNLL-U comment lines.
        #[getter]
        fn metadata(&self) -> HashMap<String, String> {
            self.inner.metadata.clone()
        }

        /// Convert to displaCy's manual rendering format.
        ///
        /// Returns:
        ///     Dictionary with 'words' and 'arcs' keys
        fn to_displacy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
            let tree = &*self.inner;
            let words = PyList::empty(py);
            let arcs = PyList::empty(py);
            for word in &tree.words {
                let entry = PyDict::new(py);
                entry.set_item("text", sym_str(tree, word.form))?;
                entry.set_item("tag", sym_str(tree, word.upos))?;
                words.append(entry)?;

                if let Some(head) = word.head {
                    let arc = PyDict::new(py);
                    arc.set_item("start", head.min(word.id))?;
                    arc.set_item("end", head.max(word.id))?;
                    arc.set_item("label", sym_str(tree, word.deprel))?;
                    arc.set_item("dir", if head < word.id { "right" } else { "left" })?;
                    arcs.append(arc)?;
                }
            }
            let data = PyDict::new(py);
            data.set_item("words", words)?;
            data.set_item("arcs", arcs)?;
            Ok(data)
        }

        /// Render as an SVG dependency visualization using displaCy.
        ///
        /// Args:
        ///     **options: Additional options passed to displacy.render()
        ///         (e.g. jupyter=True, options={"compact": True})
        ///
        /// Returns:
        ///     SVG markup string
        ///
        /// Raises:
        ///     ImportError: If spaCy is not installed
        #[pyo3(signature = (**options))]
        fn render<'py>(
            &self,
            py: Python<'py>,
            options: Option<&Bound<'py, PyDict>>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let displacy = py.import("spacy.displacy").map_err(|e| {
                if e.is_instance_of::<PyImportError>(py) {
                    PyImportError::new_err(
                        "spaCy is required for rendering. \
                         Install it with: pip install treesearch-ud[viz]",
                    )
                } else {
                    e
                }
            })?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("style", "dep")?;
            kwargs.set_item("manual", true)?;
            if let Some(options) = options {
                kwargs.update(options.as_mapping())?;
            }
            displacy.call_method("render", (self.to_displacy(py)?,), Some(&kwargs))
        }

        fn __repr__(&self) -> String {
            let tree = &*self.inner;
            let n = tree.words.len();
            if n == 0 {
                return "<Tree (empty)>".to_string();
            }

            let words: Vec<String> = tree
                .words
                .iter()
                .take(3)
                .map(|word| sym_str(tree, word.form))
                .collect();

            if n > 3 {
                format!("<Tree len={} words='{} ...'>", n, words.join(" "))
            } else {
                format!("<Tree len={} words='{}'>", n, words.join(" "))
            }
        }
    }

    /// A single word (node) in a dependency tree.
    ///
    /// Two words are equal when they are the same word of the same Tree object.
    #[pyclass(name = "Word", frozen)]
    pub struct PyWord {
        tree: Arc<RustTree>,
        id: usize,
    }

    impl PyWord {
        fn word(&self) -> &RustWord {
            &self.tree.words[self.id]
        }

        fn sibling(&self, word: &RustWord) -> PyWord {
            PyWord {
                tree: Arc::clone(&self.tree),
                id: word.id,
            }
        }

        fn siblings(&self, words: Vec<&RustWord>) -> Vec<PyWord> {
            words.into_iter().map(|word| self.sibling(word)).collect()
        }
    }

    #[pymethods]
    impl PyWord {
        /// Word ID (0-based index in tree).
        #[getter]
        fn id(&self) -> usize {
            self.id
        }

        /// Token ID from CoNLL-U (1-based).
        #[getter]
        fn token_id(&self) -> usize {
            self.word().token_id
        }

        /// Word form (surface text).
        #[getter]
        fn form(&self) -> String {
            sym_str(&self.tree, self.word().form)
        }

        /// Lemma (base form).
        #[getter]
        fn lemma(&self) -> String {
            sym_str(&self.tree, self.word().lemma)
        }

        /// Universal POS tag.
        #[getter]
        fn upos(&self) -> String {
            sym_str(&self.tree, self.word().upos)
        }

        /// Language-specific POS tag, None if unspecified (`_`).
        #[getter]
        fn xpos(&self) -> Option<String> {
            let xpos = sym_str(&self.tree, self.word().xpos);
            (xpos != "_").then_some(xpos)
        }

        /// Dependency relation to parent.
        #[getter]
        fn deprel(&self) -> String {
            sym_str(&self.tree, self.word().deprel)
        }

        /// Head word ID (0-based index), None for root.
        #[getter]
        fn head(&self) -> Option<usize> {
            self.word().head
        }

        /// Morphological features as key-value pairs.
        #[getter]
        fn feats(&self) -> HashMap<String, String> {
            feats_map(&self.tree, &self.word().feats)
        }

        /// Miscellaneous annotations as key-value pairs.
        #[getter]
        fn misc(&self) -> HashMap<String, String> {
            feats_map(&self.tree, &self.word().misc)
        }

        /// Get parent word, None for root.
        fn parent(&self) -> Option<PyWord> {
            self.word()
                .parent(&self.tree)
                .map(|word| self.sibling(word))
        }

        /// IDs of all children words.
        #[getter]
        fn children_ids(&self) -> Vec<usize> {
            self.word().children.clone()
        }

        /// Get all children words.
        fn children(&self) -> Vec<PyWord> {
            self.siblings(self.word().children(&self.tree))
        }

        /// Get children with a specific dependency relation.
        fn children_by_deprel(&self, deprel: &str) -> Vec<PyWord> {
            self.siblings(self.word().children_by_deprel(&self.tree, deprel))
        }

        /// IDs of all transitive dependents (children, grandchildren, etc.).
        #[getter]
        fn descendant_ids(&self) -> Vec<usize> {
            self.word()
                .descendants(&self.tree)
                .into_iter()
                .map(|word| word.id)
                .collect()
        }

        /// Get all transitive dependents (children, grandchildren, etc.).
        fn descendants(&self) -> Vec<PyWord> {
            self.siblings(self.word().descendants(&self.tree))
        }

        fn __eq__(&self, other: &Self) -> bool {
            self.id == other.id && Arc::ptr_eq(&self.tree, &other.tree)
        }

        fn __hash__(&self) -> u64 {
            let mut hasher = DefaultHasher::new();
            (Arc::as_ptr(&self.tree), self.id).hash(&mut hasher);
            hasher.finish()
        }

        // TODO: add xpos and head to these (but they're optional)
        fn __repr__(&self) -> String {
            format!(
                "<Word id={} form='{}' lemma='{}' upos='{}' deprel='{}'>",
                self.id,
                self.form(),
                self.lemma(),
                self.upos(),
                self.deprel()
            )
        }
    }

    /// A compiled query pattern for tree matching.
    ///
    /// Created by compile_query() and used with search functions. Patterns are
    /// reusable and should be compiled once then used across multiple searches
    /// for best performance.
    #[pyclass(name = "Pattern", from_py_object)]
    #[derive(Clone)]
    pub struct PyPattern {
        pub(crate) inner: RustPattern,
    }

    #[pymethods]
    impl PyPattern {
        fn __repr__(&self) -> String {
            format!("Pattern({} vars)", self.inner.match_pattern.n_vars)
        }
    }

    /// Wrapper that accepts either a query string or compiled Pattern
    #[derive(FromPyObject)]
    enum QueryArg {
        String(String),
        Pattern(PyPattern),
    }

    impl IntoPattern for QueryArg {
        fn into_pattern(self) -> Result<RustPattern, QueryError> {
            match self {
                QueryArg::String(s) => compile_query(&s),
                QueryArg::Pattern(p) => Ok(p.inner),
            }
        }
    }

    /// Compile a query string into a reusable Pattern.
    ///
    /// Args:
    ///     query: Query string in the treesearch query language
    ///
    /// Returns:
    ///     Compiled Pattern
    ///
    /// Raises:
    ///     ValueError: If the query is invalid
    #[pyfunction(name = "compile_query")]
    fn py_compile_query(query: &str) -> PyResult<PyPattern> {
        Ok(PyPattern {
            inner: compile_query(query)?,
        })
    }

    /// A collection of dependency trees from files or strings.
    ///
    /// Provides methods for iterating over trees and searching for patterns.
    /// Supports multiple iterations by cloning internally.
    #[pyclass(name = "Treebank", from_py_object)]
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
        fn from_string(_cls: &Bound<'_, PyType>, text: &str) -> Self {
            PyTreebank {
                inner: Treebank::from_string(text),
            }
        }

        /// Create a Treebank from a CoNLL-U file.
        ///
        /// Automatically detects and handles gzip-compressed (.conllu.gz) and
        /// zstd-compressed (.conllu.zst) files. The file is opened lazily:
        /// a missing file raises OSError when the treebank is iterated.
        ///
        /// Args:
        ///     file_path: Path to CoNLL-U file (str or os.PathLike)
        ///
        /// Returns:
        ///     Treebank instance
        #[classmethod]
        fn from_file(_cls: &Bound<'_, PyType>, file_path: PathBuf) -> Self {
            PyTreebank {
                inner: Treebank::from_path(file_path),
            }
        }

        /// Create a Treebank from multiple file paths.
        ///
        /// Files are processed in the order given.
        ///
        /// Args:
        ///     file_paths: List of paths to CoNLL-U files (str or os.PathLike)
        ///
        /// Returns:
        ///     Treebank instance
        ///
        /// Example:
        ///     >>> tb = Treebank.from_files(["file1.conllu", "file2.conllu"])
        ///     >>> for tree in tb.trees():
        ///     ...     print(tree)
        #[classmethod]
        fn from_files(_cls: &Bound<'_, PyType>, file_paths: Vec<PathBuf>) -> Self {
            PyTreebank {
                inner: Treebank::from_paths(file_paths),
            }
        }

        /// Create a Treebank from a glob pattern.
        ///
        /// Matching files are sorted, so results are deterministic across runs.
        /// A pattern that matches no files gives an empty treebank.
        ///
        /// Args:
        ///     pattern: Glob pattern, e.g. "data/**/*.conllu.gz"
        ///
        /// Returns:
        ///     Treebank instance
        ///
        /// Raises:
        ///     ValueError: If the glob pattern is malformed
        ///
        /// Example:
        ///     >>> tb = Treebank.from_glob("data/*.conllu")
        #[classmethod]
        fn from_glob(_cls: &Bound<'_, PyType>, pattern: &str) -> PyResult<Self> {
            let inner = Treebank::from_glob(pattern)
                .map_err(|e| PyValueError::new_err(format!("Invalid glob pattern: {}", e)))?;
            Ok(PyTreebank { inner })
        }

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
                        .trees(ordered)
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
        ///     pattern: Compiled pattern from compile_query() or a query string
        ///     ordered: If True (default), matches are returned in deterministic order.
        ///              If False, matches may arrive in any order for better performance.
        ///
        /// Returns:
        ///     Iterator over (tree, match) tuples
        ///
        /// Example:
        ///     >>> tb = Treebank.from_glob("data/*.conllu")
        ///     >>> # Can use compiled pattern:
        ///     >>> pattern = compile_query("MATCH { V [upos='VERB']; }")
        ///     >>> for tree, match in tb.search(pattern, ordered=True):
        ///     ...     print(match)
        ///     >>> # Or use query string directly:
        ///     >>> for tree, match in tb.search("MATCH { V [upos='VERB']; }"):
        ///     ...     print(match)
        #[pyo3(signature = (pattern, ordered=true))]
        fn search(&self, pattern: QueryArg, ordered: bool) -> PyResult<PyMatchIterator> {
            Ok(PyMatchIterator {
                inner: Box::new(
                    self.inner
                        .clone()
                        .search(pattern, ordered)?
                        .map(|result| result.map(|m| (m.tree, m.bindings))),
                ),
            })
        }

        /// Filter trees that match a pattern.
        ///
        /// Returns only trees that have at least one match for the pattern.
        /// More efficient than search() when you only need to know which trees
        /// match, not the specific bindings.
        ///
        /// Args:
        ///     pattern: Compiled pattern from compile_query() or a query string
        ///     ordered: If True (default), trees are returned in deterministic order.
        ///              If False, trees may arrive in any order for better performance.
        ///
        /// Returns:
        ///     Iterator over Tree objects
        ///
        /// Example:
        ///     >>> tb = Treebank.from_file("data.conllu")
        ///     >>> pattern = compile_query("MATCH { V [upos='VERB']; }")
        ///     >>> for tree in tb.filter(pattern):
        ///     ...     print(tree.sentence_text)
        #[pyo3(signature = (pattern, ordered=true))]
        fn filter(&self, pattern: QueryArg, ordered: bool) -> PyResult<PyTreeIterator> {
            Ok(PyTreeIterator {
                inner: Box::new(
                    self.inner
                        .clone()
                        .filter(pattern, ordered)?
                        .map(|result| result.map(Arc::new)),
                ),
            })
        }

        // TODO: make this more interesting (number of files? start of string?)
        fn __repr__(&self) -> String {
            "<Treebank>".to_string()
        }
    }

    /// Iterator over trees from a treebank.
    ///
    /// Note: Marked as unsendable because iterators have mutable state and shouldn't
    /// be shared across threads. However, we release the GIL during iteration to allow
    /// other Python threads to run in parallel.
    #[pyclass(name = "TreeIterator", unsendable)]
    struct PyTreeIterator {
        inner: ResultIter<Arc<RustTree>>,
    }

    #[pymethods]
    impl PyTreeIterator {
        fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
            slf
        }

        fn __next__(&mut self, py: Python) -> PyResult<Option<PyTree>> {
            Ok(next_detached(py, &mut self.inner)?.map(|inner| PyTree { inner }))
        }
    }

    type TreeMatch = (Arc<RustTree>, Bindings);

    /// Iterator over (tree, match) tuples from a pattern search.
    ///
    /// Note: Marked as unsendable because iterators have mutable state and shouldn't
    /// be shared across threads. However, we release the GIL during iteration to allow
    /// other Python threads to run in parallel.
    #[pyclass(name = "MatchIterator", unsendable)]
    struct PyMatchIterator {
        inner: ResultIter<TreeMatch>,
    }

    #[pymethods]
    impl PyMatchIterator {
        fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
            slf
        }

        fn __next__(&mut self, py: Python) -> PyResult<Option<(PyTree, Bindings)>> {
            Ok(next_detached(py, &mut self.inner)?
                .map(|(inner, bindings)| (PyTree { inner }, bindings)))
        }
    }

    /// Search a tree or an iterable of trees for pattern matches.
    ///
    /// Returns an iterator over (tree, match) tuples for all matches found across
    /// all trees. Each match is a dictionary mapping variable names from the query
    /// to word IDs in the tree.
    ///
    /// Args:
    ///     source: Single Tree or iterable of Trees
    ///     query: Compiled pattern from compile_query() or a query string
    ///
    /// Returns:
    ///     Iterator over (tree, match) tuples
    ///
    /// Example:
    ///     for tree, match in treesearch.search_trees([tree1, tree2], pattern):
    ///         print(match)
    #[pyfunction]
    fn search_trees(source: &Bound<'_, PyAny>, query: QueryArg) -> PyResult<PyMatchIterator> {
        let pattern = query.into_pattern()?;
        let trees: Vec<PyTree> = match source.extract::<PyTree>() {
            Ok(tree) => vec![tree],
            Err(_) => source
                .try_iter()?
                .map(|item| item?.extract::<PyTree>().map_err(PyErr::from))
                .collect::<PyResult<_>>()?,
        };
        let results: Vec<_> = trees
            .into_iter()
            .flat_map(|tree| {
                find_all_bindings(&tree.inner, &pattern)
                    .into_iter()
                    .map(move |bindings| Ok((Arc::clone(&tree.inner), bindings)))
            })
            .collect();

        Ok(PyMatchIterator {
            inner: Box::new(results.into_iter()),
        })
    }
}
