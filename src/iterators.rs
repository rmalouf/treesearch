//! Iterators for trees and matches
//!
//! Provides convenient collection interfaces for:
//! - Iterating over trees from a string, file, or glob pattern
//! - Searching patterns across trees from a string, file, or glob pattern
//! - Sequential and parallel iteration via standard traits

use crate::conllu::TreeIterator;
use crate::pattern::Pattern;
use crate::searcher::{Match, search};
use crate::tree::Tree;
use pariter::IteratorExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Source of trees for a collection
#[derive(Debug, Clone)]
enum TreeSource {
    /// In-memory CoNLL-U text
    String(String),
    /// Single file path
    File(PathBuf),
    /// Multiple file paths (from glob or explicit paths)
    Files(Vec<PathBuf>),
}

/// Collection of trees from a string, file, or glob pattern
///
/// Provides iterator-based access to trees with optional parallel processing.
/// Errors (file open, parse errors) are logged to stderr and skipped.
///
/// # Examples
///
/// ```no_run
/// use treesearch::Treebank;
/// use pariter::IteratorExt as _;
///
/// // Sequential iteration
/// let trees = Treebank::from_file("data.conllu");
/// for tree in trees {
///     println!("Tree with {} words", tree.words.len());
/// }
///
/// // Parallel iteration
/// let count = Treebank::from_glob("data/*.conllu")
///     .unwrap()
///     .into_iter()
///     .parallel_map(|tree| tree)
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
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        Self {
            source: TreeSource::File(path.as_ref().to_path_buf()),
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

    /// Create from explicit file paths
    pub fn from_paths(file_paths: Vec<PathBuf>) -> Self {
        Self {
            source: TreeSource::Files(file_paths),
        }
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = Arc<Tree>>> {
        self.clone().into_iter()
    }
}

impl IntoIterator for Treebank {
    type Item = Arc<Tree>;
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        match self.source {
            TreeSource::String(text) => {
                let iter = TreeIterator::from_string(&text)
                    .filter_map(|result| result.ok())
                    .map(Arc::new);
                Box::new(iter)
            }
            TreeSource::File(path) => {
                let iter = open_file_trees(path);
                Box::new(iter)
            }
            TreeSource::Files(paths) => {
                let iter = paths.into_iter().flat_map(open_file_trees);
                Box::new(iter)
            }
        }
    }
}

// Parallel iteration support removed - use .into_iter().parallel_map() instead

/// Collection of matches from a TreeSet and pattern
///
/// Applies a pattern to trees and yields all matches found.
/// Provides iterator-based access with optional parallel processing.
/// Errors (file open, parse errors) are logged to stderr and skipped.
///
/// # Examples
///
/// ```no_run
/// use treesearch::{Treebank, MatchSet, parse_query};
/// use pariter::IteratorExt as _;
///
/// let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
/// let tree_set = Treebank::from_file("data.conllu");
///
/// // Sequential iteration
/// let matches = MatchSet::new(&tree_set, &pattern);
/// for (tree, m) in matches {
///     println!("Found match in tree");
/// }
///
/// // Parallel iteration with glob
/// let tree_set = Treebank::from_glob("data/*.conllu").unwrap();
/// let count = MatchSet::new(&tree_set, &pattern)
///     .into_iter()
///     .parallel_map(|m| m)
///     .count();
/// ```
#[derive(Clone)]
pub struct MatchSet {
    tree_bank: Treebank,
    pattern: Pattern,
}

impl MatchSet {
    /// Create from a Treebank and pattern
    pub fn new(tree_set: &Treebank, pattern: &Pattern) -> Self {
        Self {
            tree_bank: Treebank {
                source: tree_set.source.clone(),
            },
            pattern: pattern.clone(),
        }
    }

    pub fn iter(&self) -> Box<dyn Iterator<Item = (Arc<Tree>, Match)>> {
        self.clone().into_iter()
    }
}

impl IntoIterator for MatchSet {
    type Item = (Arc<Tree>, Match);
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        let pattern = self.pattern;
        let iter = self.tree_bank.iter().flat_map(move |tree| {
            let matches: Vec<Match> = search(&tree, &pattern).collect();
            matches.into_iter().map(move |m| (tree.clone(), m))
        });
        Box::new(iter)
    }
}

// Parallel iteration support removed - use .into_iter().parallel_map() instead

/// Helper: Open a file and return an iterator over trees
///
/// Logs file open errors to stderr and returns empty iterator on error.
/// Filters out parse errors (logs to stderr via filter_map).
fn open_file_trees(path: PathBuf) -> Box<dyn Iterator<Item = Arc<Tree>>> {
    match TreeIterator::from_file(&path) {
        Ok(reader) => Box::new(reader.filter_map(Result::ok).map(Arc::new)),
        Err(e) => {
            eprintln!("Warning: Failed to open {:?}: {}", path, e);
            Box::new(std::iter::empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_query;

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
    fn test_tree_set_from_string() {
        let trees: Vec<_> = Treebank::from_string(TWO_TREE_CONLLU).into_iter().collect();

        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].words.len(), 3);
        assert_eq!(trees[1].words.len(), 2);
    }

    #[test]
    fn test_match_set_from_string() {
        let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let tree_set = Treebank::from_string(THREE_VERB_CONLLU);
        let matches: Vec<_> = MatchSet::new(&tree_set, &pattern).into_iter().collect();

        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_match_set_multiple_matches_per_tree() {
        let conllu = "1\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                      2\tJohn\tJohn\tPROPN\tNNP\t_\t1\tobj\t_\t_\n\
                      3\trunning\trun\tVERB\tVBG\t_\t1\txcomp\t_\t_\n";

        let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let tree_set = Treebank::from_string(conllu);
        let matches: Vec<_> = MatchSet::new(&tree_set, &pattern).into_iter().collect();

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_match_set_no_matches() {
        let conllu = "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n\
                      2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n";

        let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let tree_set = Treebank::from_string(conllu);
        let matches: Vec<_> = MatchSet::new(&tree_set, &pattern).into_iter().collect();

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_match_set_with_constraints() {
        let conllu = "1\thelped\thelp\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                      2\tus\twe\tPRON\tPRP\t_\t1\tobj\t_\t_\n\
                      3\tto\tto\tPART\tTO\t_\t4\tmark\t_\t_\n\
                      4\twin\twin\tVERB\tVB\t_\t1\txcomp\t_\t_\n";

        let pattern =
            parse_query("MATCH { V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; V1 -> V2; }").unwrap();
        let tree_set = Treebank::from_string(conllu);
        let matches: Vec<_> = MatchSet::new(&tree_set, &pattern).into_iter().collect();

        assert_eq!(matches.len(), 1);
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
        fn test_tree_set_from_paths() {
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

            let results: Vec<_> = Treebank::from_paths(paths).into_iter().collect();

            assert_eq!(results.len(), 2);
            assert_eq!(results[0].words.len(), 2);
            assert_eq!(results[1].words.len(), 2);
        }

        #[test]
        fn test_tree_set_from_glob() {
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
            let results: Vec<_> = Treebank::from_glob(&pattern).unwrap().into_iter().collect();

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

            let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
            let tree_set = Treebank::from_paths(paths);
            let results: Vec<_> = MatchSet::new(&tree_set, &pattern).into_iter().collect();

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

            let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
            let glob_pattern = format!("{}/*.conllu", dir.path().display());
            let tree_set = Treebank::from_glob(&glob_pattern).unwrap();
            let results: Vec<_> = MatchSet::new(&tree_set, &pattern).into_iter().collect();

            assert_eq!(results.len(), 2);
        }

        #[test]
        fn test_skips_bad_files() {
            let (dir, mut paths) = create_test_files(&[(
                "good.conllu",
                "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
            )]);

            let good_file = paths[0].clone();
            let bad_file = dir.path().join("nonexistent.conllu");
            paths = vec![good_file.clone(), bad_file, good_file];

            let results: Vec<_> = Treebank::from_paths(paths).into_iter().collect();

            assert_eq!(results.len(), 2);
        }

        #[test]
        fn test_tree_set_par_iter() {
            let (_dir, paths) = create_test_files(&[
                (
                    "file1.conllu",
                    "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n",
                ),
                (
                    "file2.conllu",
                    "1\tCats\tcat\tNOUN\tNNS\t_\t2\tnsubj\t_\t_\n2\tsleep\tsleep\tVERB\tVBP\t_\t0\troot\t_\t_\n",
                ),
                (
                    "file3.conllu",
                    "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
            ]);

            let results: Vec<_> = Treebank::from_paths(paths)
                .into_iter()
                .parallel_map(|tree| tree)
                .collect();

            assert_eq!(results.len(), 3);
            assert_eq!(results[0].words.len(), 2);
            assert_eq!(results[1].words.len(), 2);
            assert_eq!(results[2].words.len(), 1);
        }

        #[test]
        fn test_match_set_par_iter() {
            let (_dir, paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
                ("c.conllu", "1\twalks\twalk\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
            ]);

            let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
            let tree_set = Treebank::from_paths(paths);
            let results: Vec<_> = MatchSet::new(&tree_set, &pattern)
                .into_iter()
                .parallel_map(|m| m)
                .collect();

            assert_eq!(results.len(), 3);
        }
    }
}
