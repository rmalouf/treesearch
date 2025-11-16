//! Iterators for trees and matches
//!
//! Provides convenient iterator interfaces for:
//! - Iterating over trees from a file
//! - Searching patterns across multiple trees
//! - Iterating over trees from multiple files (glob patterns)
//! - Searching patterns across multiple files

use crate::conllu::{CoNLLUReader, ParseError};
use crate::pattern::Pattern;
use crate::searcher::{Match, search};
use crate::tree::Tree;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// Iterator over trees from a CoNLL-U file
///
/// This is a wrapper around CoNLLUReader that provides a cleaner API
/// for iterating over trees. The underlying CoNLLUReader already
/// implements Iterator<Item = Result<Tree, ParseError>>.
pub struct TreeIterator<R: BufRead> {
    reader: CoNLLUReader<R>,
}

impl TreeIterator<std::io::BufReader<Box<dyn std::io::Read>>> {
    /// Create a tree iterator from a file path
    ///
    /// Automatically detects and handles gzip-compressed files.
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let reader = CoNLLUReader::from_file(path)?;
        Ok(Self { reader })
    }
}

impl TreeIterator<std::io::BufReader<std::io::Cursor<String>>> {
    /// Create a tree iterator from a string
    pub fn from_string(text: &str) -> Self {
        let reader = CoNLLUReader::from_string(text);
        Self { reader }
    }
}

impl<R: BufRead> Iterator for TreeIterator<R> {
    type Item = Result<Tree, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next()
    }
}

/// Iterator over matches across multiple trees
///
/// Applies a pattern to each tree and yields all matches found.
pub struct MatchIterator<R: BufRead> {
    trees: TreeIterator<R>,
    pattern: Pattern,
    current_tree: Option<Tree>,
    current_matches: std::vec::IntoIter<Match>,
}

impl MatchIterator<std::io::BufReader<Box<dyn std::io::Read>>> {
    /// Create a match iterator from a file and pattern
    pub fn from_file(path: &Path, pattern: Pattern) -> std::io::Result<Self> {
        let trees = TreeIterator::from_file(path)?;
        Ok(Self {
            trees,
            pattern,
            current_tree: None,
            current_matches: Vec::new().into_iter(),
        })
    }
}

impl MatchIterator<std::io::BufReader<std::io::Cursor<String>>> {
    /// Create a match iterator from a string and pattern
    pub fn from_string(text: &str, pattern: Pattern) -> Self {
        let trees = TreeIterator::from_string(text);
        Self {
            trees,
            pattern,
            current_tree: None,
            current_matches: Vec::new().into_iter(),
        }
    }
}

impl<R: BufRead> Iterator for MatchIterator<R> {
    /// Returns (tree, match)
    type Item = (Tree, Match);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next match from current tree
            if let Some(match_) = self.current_matches.next() {
                // Clone the tree for return (caller may need it)
                let tree = self.current_tree.as_ref().unwrap().clone();
                return Some((tree, match_));
            }

            // No more matches in current tree, get next tree
            match self.trees.next() {
                Some(Ok(tree)) => {
                    let matches: Vec<Match> = search(&tree, &self.pattern).collect();
                    self.current_matches = matches.into_iter();
                    self.current_tree = Some(tree);
                    // Continue loop to try getting first match from this tree
                }
                Some(Err(_)) => {
                    // Parse error - skip this tree and continue
                    continue;
                }
                None => {
                    // No more trees
                    return None;
                }
            }
        }
    }
}

/// Iterator over trees from multiple CoNLL-U files
///
/// Discovers files matching a glob pattern and iterates over all trees
/// across all files. Files are processed in sorted order for deterministic results.
/// Files that fail to open are skipped with a warning to stderr.
pub struct MultiFileTreeIterator {
    file_paths: Vec<PathBuf>,
    current_file_idx: usize,
    current_iterator: Option<TreeIterator<std::io::BufReader<Box<dyn std::io::Read>>>>,
}

impl MultiFileTreeIterator {
    /// Create a multi-file tree iterator from a glob pattern
    pub fn from_glob(pattern: &str) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> = glob::glob(pattern)?.filter_map(Result::ok).collect();

        // Sort for deterministic ordering
        file_paths.sort();

        Ok(Self {
            file_paths,
            current_file_idx: 0,
            current_iterator: None,
        })
    }

    /// Create a multi-file tree iterator from explicit file paths
    pub fn from_paths(file_paths: Vec<PathBuf>) -> Self {
        Self {
            file_paths,
            current_file_idx: 0,
            current_iterator: None,
        }
    }

    /// Get the current file path being processed
    pub fn current_file(&self) -> Option<&Path> {
        if self.current_file_idx > 0 && self.current_file_idx <= self.file_paths.len() {
            Some(&self.file_paths[self.current_file_idx - 1])
        } else {
            None
        }
    }
}

impl Iterator for MultiFileTreeIterator {
    /// Returns parse_result
    type Item = Result<Tree, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next tree from current file
            if let Some(ref mut iter) = self.current_iterator {
                if let Some(tree_result) = iter.next() {
                    return Some(tree_result);
                }
            }

            // Current file exhausted or no file open, try next file
            if self.current_file_idx >= self.file_paths.len() {
                // No more files
                return None;
            }

            let file_path = &self.file_paths[self.current_file_idx];
            self.current_file_idx += 1;

            match TreeIterator::from_file(file_path) {
                Ok(iter) => {
                    self.current_iterator = Some(iter);
                    // Continue loop to get first tree from this file
                }
                Err(e) => {
                    // Skip file with warning
                    eprintln!("Warning: Failed to open {:?}: {}", file_path, e);
                    // Continue to next file
                    continue;
                }
            }
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
    current_file_idx: usize,
    current_iterator: Option<MatchIterator<std::io::BufReader<Box<dyn std::io::Read>>>>,
}

impl MultiFileMatchIterator {
    /// Create a multi-file match iterator from a glob pattern
    pub fn from_glob(glob_pattern: &str, pattern: Pattern) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> =
            glob::glob(glob_pattern)?.filter_map(Result::ok).collect();

        // Sort for deterministic ordering
        file_paths.sort();

        Ok(Self {
            file_paths,
            pattern,
            current_file_idx: 0,
            current_iterator: None,
        })
    }

    /// Create a multi-file match iterator from explicit file paths
    pub fn from_paths(file_paths: Vec<PathBuf>, pattern: Pattern) -> Self {
        Self {
            file_paths,
            pattern,
            current_file_idx: 0,
            current_iterator: None,
        }
    }
}

impl Iterator for MultiFileMatchIterator {
    /// Returns (tree, match)
    type Item = (Tree, Match);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next match from current file
            if let Some(ref mut iter) = self.current_iterator {
                if let Some((tree, match_)) = iter.next() {
                    return Some((tree, match_));
                }
            }

            // Current file exhausted or no file open, try next file
            if self.current_file_idx >= self.file_paths.len() {
                // No more files
                return None;
            }

            let file_path = &self.file_paths[self.current_file_idx];
            self.current_file_idx += 1;

            match MatchIterator::from_file(file_path, self.pattern.clone()) {
                Ok(iter) => {
                    self.current_iterator = Some(iter);
                    // Continue loop to get first match from this file
                }
                Err(e) => {
                    // Skip file with warning
                    eprintln!("Warning: Failed to open {:?}: {}", file_path, e);
                    // Continue to next file
                    continue;
                }
            }
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
    fn test_tree_iterator_from_string() {
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
    }
}
