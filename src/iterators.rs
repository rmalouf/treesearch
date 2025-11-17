//! Iterators for trees and matches
//!
//! Provides convenient iterator interfaces for:
//! - Iterating over trees from a file
//! - Searching patterns across multiple trees
//! - Iterating over trees from multiple files (glob patterns)
//! - Searching patterns across multiple files

use crate::conllu::{ParseError, TreeIterator};
use crate::pattern::Pattern;
use crate::searcher::{Match, search};
use crate::tree::Tree;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// Iterator over matches across multiple trees
///
/// Applies a pattern to each tree and yields all matches found.
pub struct MatchIterator {
    inner: Box<dyn Iterator<Item = (Tree, Match)>>,
}

impl MatchIterator {
    /// Create a match iterator from a file and pattern
    pub fn from_file(path: &Path, pattern: Pattern) -> std::io::Result<Self> {
        let trees = TreeIterator::from_file(path)?;
        Ok(Self::new(trees, pattern))
    }

    /// Create a match iterator from a string and pattern
    pub fn from_string(text: &str, pattern: Pattern) -> Self {
        let trees = TreeIterator::from_string(text);
        Self::new(trees, pattern)
    }

    fn new<R: BufRead + 'static>(trees: TreeIterator<R>, pattern: Pattern) -> Self {
        let inner = trees.filter_map(Result::ok).flat_map(move |tree| {
            let matches: Vec<Match> = search(&tree, &pattern).collect();
            matches.into_iter().map(move |m| (tree.clone(), m))
        });

        Self {
            inner: Box::new(inner),
        }
    }
}

impl Iterator for MatchIterator {
    /// Returns (tree, match)
    type Item = (Tree, Match);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterator over trees from multiple CoNLL-U files
///
/// Discovers files matching a glob pattern and iterates over all trees
/// across all files. Files are processed in sorted order for deterministic results.
/// Files that fail to open are skipped with a warning to stderr.
pub struct MultiFileTreeIterator {
    file_paths: Vec<PathBuf>,
    current_iter: Option<Box<dyn Iterator<Item = Result<Tree, ParseError>>>>,
    file_index: usize,
}

impl MultiFileTreeIterator {
    /// Create a multi-file tree iterator from a glob pattern
    pub fn from_glob(pattern: &str) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> = glob::glob(pattern)?.filter_map(Result::ok).collect();
        file_paths.sort();
        Ok(Self::from_paths(file_paths))
    }

    /// Create a multi-file tree iterator from explicit file paths
    pub fn from_paths(file_paths: Vec<PathBuf>) -> Self {
        Self {
            file_paths,
            current_iter: None,
            file_index: 0,
        }
    }

    /// Convert to a parallel iterator over trees (file-level parallelism)
    ///
    /// Processes multiple files in parallel using rayon. Trees within each file
    /// are still processed sequentially.
    pub fn par_iter(self) -> impl rayon::iter::ParallelIterator<Item = Result<Tree, ParseError>> {
        use rayon::prelude::*;
        self.file_paths
            .into_par_iter()
            .flat_map_iter(|path| match TreeIterator::from_file(&path) {
                Ok(reader) => {
                    Box::new(reader) as Box<dyn Iterator<Item = Result<Tree, ParseError>>>
                }
                Err(e) => {
                    eprintln!("Warning: Failed to open {:?}: {}", path, e);
                    Box::new(std::iter::empty())
                }
            })
    }
}

impl Iterator for MultiFileTreeIterator {
    type Item = Result<Tree, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try current iterator
            if let Some(iter) = &mut self.current_iter
                && let Some(item) = iter.next()
            {
                return Some(item);
            }

            // Move to next file
            if self.file_index >= self.file_paths.len() {
                return None;
            }

            let path = &self.file_paths[self.file_index];
            self.file_index += 1;

            self.current_iter = match TreeIterator::from_file(path) {
                Ok(reader) => Some(Box::new(reader)),
                Err(e) => {
                    eprintln!("Warning: Failed to open {:?}: {}", path, e);
                    None
                }
            };
        }
    }
}

/// Iterator over matches across multiple CoNLL-U files
///
/// Applies a pattern to all trees across multiple files discovered by a glob pattern.
/// Files are processed in sorted order. Files that fail to open or trees that fail
/// to parse are skipped with warnings to stderr.
pub struct MultiFileMatchIterator {
    file_paths: Vec<PathBuf>,
    pattern: Pattern,
    current_iter: Option<MatchIterator>,
    file_index: usize,
}

impl MultiFileMatchIterator {
    /// Create a multi-file match iterator from a glob pattern
    pub fn from_glob(glob_pattern: &str, pattern: Pattern) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> =
            glob::glob(glob_pattern)?.filter_map(Result::ok).collect();
        file_paths.sort();
        Ok(Self::from_paths(file_paths, pattern))
    }

    /// Create a multi-file match iterator from explicit file paths
    pub fn from_paths(file_paths: Vec<PathBuf>, pattern: Pattern) -> Self {
        Self {
            file_paths,
            pattern,
            current_iter: None,
            file_index: 0,
        }
    }

    /// Convert to a parallel iterator over matches (file-level parallelism)
    ///
    /// Processes multiple files in parallel using rayon. Trees within each file
    /// are still processed sequentially.
    pub fn par_iter(self) -> impl rayon::iter::ParallelIterator<Item = (Tree, Match)> {
        use rayon::prelude::*;
        let pattern = self.pattern;
        self.file_paths.into_par_iter().flat_map_iter(move |path| {
            match MatchIterator::from_file(&path, pattern.clone()) {
                Ok(iter) => Box::new(iter) as Box<dyn Iterator<Item = (Tree, Match)>>,
                Err(e) => {
                    eprintln!("Warning: Failed to open {:?}: {}", path, e);
                    Box::new(std::iter::empty())
                }
            }
        })
    }
}

impl Iterator for MultiFileMatchIterator {
    type Item = (Tree, Match);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try current iterator
            if let Some(iter) = &mut self.current_iter
                && let Some(item) = iter.next()
            {
                return Some(item);
            }

            // Move to next file
            if self.file_index >= self.file_paths.len() {
                return None;
            }

            let path = &self.file_paths[self.file_index];
            self.file_index += 1;

            self.current_iter = match MatchIterator::from_file(path, self.pattern.clone()) {
                Ok(iter) => Some(iter),
                Err(e) => {
                    eprintln!("Warning: Failed to open {:?}: {}", path, e);
                    None
                }
            };
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
    fn test_conllu_reader_from_string() {
        let trees: Vec<_> = TreeIterator::from_string(TWO_TREE_CONLLU)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].words.len(), 3);
        assert_eq!(trees[1].words.len(), 2);
    }

    #[test]
    fn test_match_iterator_from_string() {
        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(THREE_VERB_CONLLU, pattern).collect();

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].1, vec![0]);
        assert_eq!(matches[1].1, vec![0]);
        assert_eq!(matches[2].1, vec![0]);
    }

    #[test]
    fn test_match_iterator_multiple_matches_per_tree() {
        let conllu = "1\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                      2\tJohn\tJohn\tPROPN\tNNP\t_\t1\tobj\t_\t_\n\
                      3\trunning\trun\tVERB\tVBG\t_\t1\txcomp\t_\t_\n";

        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(conllu, pattern).collect();

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_match_iterator_no_matches() {
        let conllu = "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n\
                      2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n";

        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(conllu, pattern).collect();

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_match_iterator_with_constraints() {
        let conllu = "1\thelped\thelp\tVERB\tVBD\t_\t0\troot\t_\t_\n\
                      2\tus\twe\tPRON\tPRP\t_\t1\tobj\t_\t_\n\
                      3\tto\tto\tPART\tTO\t_\t4\tmark\t_\t_\n\
                      4\twin\twin\tVERB\tVB\t_\t1\txcomp\t_\t_\n";

        let pattern = parse_query("V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; V1 -> V2;").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(conllu, pattern).collect();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1, vec![0, 3]);
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
        fn test_tree_iterator_from_paths() {
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

            let results: Vec<_> = MultiFileTreeIterator::from_paths(paths)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            assert_eq!(results.len(), 2);
            assert_eq!(results[0].words.len(), 2);
            assert_eq!(results[1].words.len(), 2);
        }

        #[test]
        fn test_tree_iterator_from_glob() {
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
            let results: Vec<_> = MultiFileTreeIterator::from_glob(&pattern)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            assert_eq!(results.len(), 2);
            assert_eq!(results[0].words.len(), 2);
            assert_eq!(results[1].words.len(), 2);
        }

        #[test]
        fn test_match_iterator_from_paths() {
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

            let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
            let results: Vec<_> = MultiFileMatchIterator::from_paths(paths, pattern).collect();

            assert_eq!(results.len(), 2);
            assert_eq!(results[0].1, vec![0]);
            assert_eq!(results[1].1, vec![0]);
        }

        #[test]
        fn test_match_iterator_from_glob() {
            let (dir, _paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
            ]);

            let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
            let glob_pattern = format!("{}/*.conllu", dir.path().display());
            let results: Vec<_> = MultiFileMatchIterator::from_glob(&glob_pattern, pattern)
                .unwrap()
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

            let results: Vec<_> = MultiFileTreeIterator::from_paths(paths)
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 2);
        }

        #[test]
        fn test_tree_iterator_par_iter() {
            use rayon::prelude::*;

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

            let results: Vec<_> = MultiFileTreeIterator::from_paths(paths)
                .par_iter()
                .filter_map(Result::ok)
                .collect();

            assert_eq!(results.len(), 3);
            assert_eq!(results[0].words.len(), 2);
            assert_eq!(results[1].words.len(), 2);
            assert_eq!(results[2].words.len(), 1);
        }

        #[test]
        fn test_match_iterator_par_iter() {
            use rayon::prelude::*;

            let (_dir, paths) = create_test_files(&[
                ("a.conllu", "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
                (
                    "b.conllu",
                    "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n",
                ),
                ("c.conllu", "1\twalks\twalk\tVERB\tVBZ\t_\t0\troot\t_\t_\n"),
            ]);

            let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
            let results: Vec<_> = MultiFileMatchIterator::from_paths(paths, pattern)
                .par_iter()
                .collect();

            assert_eq!(results.len(), 3);
            assert_eq!(results[0].1, vec![0]);
            assert_eq!(results[1].1, vec![0]);
            assert_eq!(results[2].1, vec![0]);
        }
    }
}
