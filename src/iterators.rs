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
use rayon::prelude::*;
use std::path::{Path, PathBuf};

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
/// Provides both sequential and parallel iteration over trees.
/// Errors (file open, parse errors) are logged to stderr and skipped.
///
/// # Examples
///
/// ```no_run
/// use treesearch::TreeSet;
/// use rayon::prelude::*;
///
/// // Sequential iteration
/// let trees = TreeSet::from_file("data.conllu");
/// for tree in trees {
///     println!("Tree with {} words", tree.words.len());
/// }
///
/// // Parallel iteration
/// let count = TreeSet::from_glob("data/*.conllu")
///     .unwrap()
///     .into_par_iter()
///     .count();
/// ```
pub struct TreeSet {
    source: TreeSource,
}

impl TreeSet {
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
}

impl IntoIterator for TreeSet {
    type Item = Tree;
    type IntoIter = Box<dyn Iterator<Item = Tree>>;

    fn into_iter(self) -> Self::IntoIter {
        match self.source {
            TreeSource::String(text) => {
                let iter = TreeIterator::from_string(&text).filter_map(|result| result.ok());
                Box::new(iter)
            }
            TreeSource::File(path) => Box::new(open_file_trees(path)),
            TreeSource::Files(paths) => {
                let iter = paths.into_iter().flat_map(open_file_trees);
                Box::new(iter)
            }
        }
    }
}

impl IntoParallelIterator for TreeSet {
    type Item = Tree;
    type Iter = rayon::iter::Either<
        rayon::iter::FlatMapIter<
            rayon::vec::IntoIter<PathBuf>,
            fn(PathBuf) -> Box<dyn Iterator<Item = Tree>>,
        >,
        rayon::vec::IntoIter<Tree>,
    >;

    fn into_par_iter(self) -> Self::Iter {
        match self.source {
            TreeSource::Files(paths) => {
                // File-level parallelism (optimal for multi-file)
                rayon::iter::Either::Left(paths.into_par_iter().flat_map_iter(open_file_trees))
            }
            _ => {
                // Collect then parallelize (for single file or string)
                let trees: Vec<_> = self.into_iter().collect();
                rayon::iter::Either::Right(trees.into_par_iter())
            }
        }
    }
}

/// Collection of matches from a string, file, or glob pattern
///
/// Applies a pattern to trees and yields all matches found.
/// Provides both sequential and parallel iteration.
/// Errors (file open, parse errors) are logged to stderr and skipped.
///
/// # Examples
///
/// ```no_run
/// use treesearch::{MatchSet, parse_query};
/// use rayon::prelude::*;
///
/// let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
///
/// // Sequential iteration
/// let matches = MatchSet::from_file("data.conllu", pattern.clone());
/// for (tree, m) in matches {
///     println!("Found match in tree");
/// }
///
/// // Parallel iteration
/// let count = MatchSet::from_glob("data/*.conllu", pattern)
///     .unwrap()
///     .into_par_iter()
///     .count();
/// ```
pub struct MatchSet {
    tree_source: TreeSource,
    pattern: Pattern,
}

impl MatchSet {
    /// Create from an in-memory CoNLL-U string and pattern
    pub fn from_string(text: &str, pattern: Pattern) -> Self {
        Self {
            tree_source: TreeSource::String(text.to_string()),
            pattern,
        }
    }

    /// Create from a single file path and pattern
    pub fn from_file(path: impl AsRef<Path>, pattern: Pattern) -> Self {
        Self {
            tree_source: TreeSource::File(path.as_ref().to_path_buf()),
            pattern,
        }
    }

    /// Create from a glob pattern and search pattern
    ///
    /// Files are processed in sorted order for deterministic results.
    pub fn from_glob(glob_pattern: &str, pattern: Pattern) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> =
            glob::glob(glob_pattern)?.filter_map(Result::ok).collect();
        file_paths.sort();
        Ok(Self::from_paths(file_paths, pattern))
    }

    /// Create from explicit file paths and pattern
    pub fn from_paths(file_paths: Vec<PathBuf>, pattern: Pattern) -> Self {
        Self {
            tree_source: TreeSource::Files(file_paths),
            pattern,
        }
    }
}

impl IntoIterator for MatchSet {
    type Item = (Tree, Match);
    type IntoIter = Box<dyn Iterator<Item = (Tree, Match)>>;

    fn into_iter(self) -> Self::IntoIter {
        match self.tree_source {
            TreeSource::String(text) => {
                let pattern = self.pattern;
                let iter = TreeIterator::from_string(&text)
                    .filter_map(Result::ok)
                    .flat_map(move |tree| {
                        let matches: Vec<Match> = search(&tree, &pattern).collect();
                        matches.into_iter().map(move |m| (tree.clone(), m))
                    });
                Box::new(iter)
            }
            TreeSource::File(path) => Box::new(open_file_matches(path, self.pattern)),
            TreeSource::Files(paths) => {
                let pattern = self.pattern;
                let iter = paths
                    .into_iter()
                    .flat_map(move |path| open_file_matches(path, pattern.clone()));
                Box::new(iter)
            }
        }
    }
}

impl IntoParallelIterator for MatchSet {
    type Item = (Tree, Match);
    type Iter = rayon::iter::Either<
        rayon::iter::FlatMapIter<
            rayon::vec::IntoIter<(PathBuf, Pattern)>,
            fn((PathBuf, Pattern)) -> Box<dyn Iterator<Item = (Tree, Match)>>,
        >,
        rayon::vec::IntoIter<(Tree, Match)>,
    >;

    fn into_par_iter(self) -> Self::Iter {
        match self.tree_source {
            TreeSource::Files(paths) => {
                // File-level parallelism (optimal for multi-file)
                // Pair each path with a clone of the pattern
                let pattern = self.pattern;
                let path_pattern_pairs: Vec<_> = paths
                    .into_iter()
                    .map(|path| (path, pattern.clone()))
                    .collect();

                rayon::iter::Either::Left(
                    path_pattern_pairs
                        .into_par_iter()
                        .flat_map_iter(|(path, pattern)| open_file_matches(path, pattern)),
                )
            }
            _ => {
                // Collect then parallelize (for single file or string)
                let matches: Vec<_> = self.into_iter().collect();
                rayon::iter::Either::Right(matches.into_par_iter())
            }
        }
    }
}

/// Helper: Open a file and return an iterator over trees
///
/// Logs file open errors to stderr and returns empty iterator on error.
/// Filters out parse errors (logs to stderr via filter_map).
fn open_file_trees(path: PathBuf) -> Box<dyn Iterator<Item = Tree>> {
    match TreeIterator::from_file(&path) {
        Ok(reader) => Box::new(reader.filter_map(Result::ok)),
        Err(e) => {
            eprintln!("Warning: Failed to open {:?}: {}", path, e);
            Box::new(std::iter::empty())
        }
    }
}

/// Helper: Open a file and return an iterator over matches
///
/// Logs file open errors to stderr and returns empty iterator on error.
/// Filters out parse errors (logs to stderr via filter_map).
fn open_file_matches(path: PathBuf, pattern: Pattern) -> Box<dyn Iterator<Item = (Tree, Match)>> {
    match TreeIterator::from_file(&path) {
        Ok(reader) => {
            let iter = reader.filter_map(Result::ok).flat_map(move |tree| {
                let matches: Vec<Match> = search(&tree, &pattern).collect();
                matches.into_iter().map(move |m| (tree.clone(), m))
            });
            Box::new(iter)
        }
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
        let trees: Vec<_> = TreeSet::from_string(TWO_TREE_CONLLU).into_iter().collect();

        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].words.len(), 3);
        assert_eq!(trees[1].words.len(), 2);
    }

    #[test]
    fn test_match_set_from_string() {
        let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let matches: Vec<_> = MatchSet::from_string(THREE_VERB_CONLLU, pattern)
            .into_iter()
            .collect();

        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_match_set_multiple_matches_per_tree() {
        let conllu = "1\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                      2\tJohn\tJohn\tPROPN\tNNP\t_\t1\tobj\t_\t_\n\
                      3\trunning\trun\tVERB\tVBG\t_\t1\txcomp\t_\t_\n";

        let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let matches: Vec<_> = MatchSet::from_string(conllu, pattern).into_iter().collect();

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_match_set_no_matches() {
        let conllu = "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n\
                      2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n";

        let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
        let matches: Vec<_> = MatchSet::from_string(conllu, pattern).into_iter().collect();

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
        let matches: Vec<_> = MatchSet::from_string(conllu, pattern).into_iter().collect();

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

            let results: Vec<_> = TreeSet::from_paths(paths).into_iter().collect();

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
            let results: Vec<_> = TreeSet::from_glob(&pattern).unwrap().into_iter().collect();

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
            let results: Vec<_> = MatchSet::from_paths(paths, pattern).into_iter().collect();

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
            let results: Vec<_> = MatchSet::from_glob(&glob_pattern, pattern)
                .unwrap()
                .into_iter()
                .collect();

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

            let results: Vec<_> = TreeSet::from_paths(paths).into_iter().collect();

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

            let results: Vec<_> = TreeSet::from_paths(paths).into_par_iter().collect();

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
            let results: Vec<_> = MatchSet::from_paths(paths, pattern)
                .into_par_iter()
                .collect();

            assert_eq!(results.len(), 3);
        }
    }
}
