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
    ///
    /// # Example
    /// ```no_run
    /// use treesearch::TreeIterator;
    /// use std::path::Path;
    ///
    /// let path = Path::new("corpus.conllu");
    /// let trees = TreeIterator::from_file(path).unwrap();
    /// for tree_result in trees {
    ///     let tree = tree_result.unwrap();
    ///     // Process tree...
    /// }
    /// ```
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let reader = CoNLLUReader::from_file(path)?;
        Ok(Self { reader })
    }
}

impl TreeIterator<std::io::BufReader<std::io::Cursor<String>>> {
    /// Create a tree iterator from a string
    ///
    /// # Example
    /// ```
    /// use treesearch::TreeIterator;
    ///
    /// let conllu = "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n\n";
    /// let trees = TreeIterator::from_string(conllu);
    /// ```
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
/// Each match includes both the tree index and the word bindings.
pub struct MatchIterator<R: BufRead> {
    trees: TreeIterator<R>,
    pattern: Pattern,
    current_tree: Option<Tree>,
    current_tree_idx: usize,
    current_matches: std::vec::IntoIter<Match>,
}

impl MatchIterator<std::io::BufReader<Box<dyn std::io::Read>>> {
    /// Create a match iterator from a file and pattern
    ///
    /// # Example
    /// ```no_run
    /// use treesearch::{MatchIterator, parse_query};
    /// use std::path::Path;
    ///
    /// let path = Path::new("corpus.conllu");
    /// let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
    /// let matches = MatchIterator::from_file(path, pattern).unwrap();
    ///
    /// for (tree_idx, tree, match_) in matches {
    ///     println!("Found match in tree {}: {:?}", tree_idx, match_);
    /// }
    /// ```
    pub fn from_file(path: &Path, pattern: Pattern) -> std::io::Result<Self> {
        let trees = TreeIterator::from_file(path)?;
        Ok(Self {
            trees,
            pattern,
            current_tree: None,
            current_tree_idx: 0,
            current_matches: Vec::new().into_iter(),
        })
    }
}

impl MatchIterator<std::io::BufReader<std::io::Cursor<String>>> {
    /// Create a match iterator from a string and pattern
    ///
    /// # Example
    /// ```
    /// use treesearch::{MatchIterator, parse_query};
    ///
    /// let conllu = "1\thelped\thelp\tVERB\tVBD\t_\t0\troot\t_\t_\n\n";
    /// let pattern = parse_query("V [lemma=\"help\"];").unwrap();
    /// let matches = MatchIterator::from_string(conllu, pattern);
    ///
    /// for (tree_idx, tree, match_) in matches {
    ///     println!("Found match in tree {}: {:?}", tree_idx, match_);
    /// }
    /// ```
    pub fn from_string(text: &str, pattern: Pattern) -> Self {
        let trees = TreeIterator::from_string(text);
        Self {
            trees,
            pattern,
            current_tree: None,
            current_tree_idx: 0,
            current_matches: Vec::new().into_iter(),
        }
    }
}

impl<R: BufRead> Iterator for MatchIterator<R> {
    /// Returns (tree_index, tree, match)
    type Item = (usize, Tree, Match);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next match from current tree
            if let Some(match_) = self.current_matches.next() {
                // Clone the tree for return (caller may need it)
                let tree = self.current_tree.as_ref().unwrap().clone();
                return Some((self.current_tree_idx, tree, match_));
            }

            // No more matches in current tree, get next tree
            match self.trees.next() {
                Some(Ok(tree)) => {
                    self.current_tree_idx += 1;
                    let matches: Vec<Match> = search(&tree, self.pattern.clone()).collect();
                    self.current_matches = matches.into_iter();
                    self.current_tree = Some(tree);
                    // Continue loop to try getting first match from this tree
                }
                Some(Err(_)) => {
                    // Parse error - skip this tree and continue
                    // TODO: Consider whether to expose parse errors
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
    ///
    /// # Example
    /// ```no_run
    /// use treesearch::MultiFileTreeIterator;
    ///
    /// let trees = MultiFileTreeIterator::from_glob("data/*.conllu").unwrap();
    /// for (path, tree_result) in trees {
    ///     match tree_result {
    ///         Ok(tree) => println!("Tree from {:?}: {} words", path, tree.words.len()),
    ///         Err(e) => eprintln!("Parse error in {:?}: {}", path, e),
    ///     }
    /// }
    /// ```
    pub fn from_glob(pattern: &str) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> = glob::glob(pattern)?
            .filter_map(Result::ok)
            .collect();

        // Sort for deterministic ordering
        file_paths.sort();

        Ok(Self {
            file_paths,
            current_file_idx: 0,
            current_iterator: None,
        })
    }

    /// Create a multi-file tree iterator from explicit file paths
    ///
    /// # Example
    /// ```no_run
    /// use treesearch::MultiFileTreeIterator;
    /// use std::path::PathBuf;
    ///
    /// let paths = vec![
    ///     PathBuf::from("file1.conllu"),
    ///     PathBuf::from("file2.conllu"),
    /// ];
    /// let trees = MultiFileTreeIterator::from_paths(paths);
    /// ```
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
    /// Returns (file_path, parse_result)
    type Item = (PathBuf, Result<Tree, ParseError>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next tree from current file
            if let Some(ref mut iter) = self.current_iterator {
                if let Some(tree_result) = iter.next() {
                    let current_path = self.file_paths[self.current_file_idx - 1].clone();
                    return Some((current_path, tree_result));
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
    ///
    /// # Example
    /// ```no_run
    /// use treesearch::{MultiFileMatchIterator, parse_query};
    ///
    /// let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
    /// let matches = MultiFileMatchIterator::from_glob("data/*.conllu", pattern).unwrap();
    ///
    /// for (path, tree, match_) in matches {
    ///     println!("Match in {:?}: {:?}", path, match_);
    /// }
    /// ```
    pub fn from_glob(glob_pattern: &str, pattern: Pattern) -> Result<Self, glob::PatternError> {
        let mut file_paths: Vec<PathBuf> = glob::glob(glob_pattern)?
            .filter_map(Result::ok)
            .collect();

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
    ///
    /// # Example
    /// ```no_run
    /// use treesearch::{MultiFileMatchIterator, parse_query};
    /// use std::path::PathBuf;
    ///
    /// let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
    /// let paths = vec![
    ///     PathBuf::from("file1.conllu"),
    ///     PathBuf::from("file2.conllu"),
    /// ];
    /// let matches = MultiFileMatchIterator::from_paths(paths, pattern);
    /// ```
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
    /// Returns (file_path, tree, match)
    type Item = (PathBuf, Tree, Match);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next match from current file
            if let Some(ref mut iter) = self.current_iterator {
                if let Some((_tree_idx, tree, match_)) = iter.next() {
                    let current_path = self.file_paths[self.current_file_idx - 1].clone();
                    return Some((current_path, tree, match_));
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

    #[test]
    fn test_tree_iterator_from_string() {
        let conllu = r#"# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	_

# text = Cats sleep.
1	Cats	cat	NOUN	NNS	_	2	nsubj	_	_
2	sleep	sleep	VERB	VBP	_	0	root	_	_

"#;

        let trees: Vec<_> = TreeIterator::from_string(conllu)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].words.len(), 3);
        assert_eq!(trees[1].words.len(), 2);
    }

    #[test]
    fn test_match_iterator_from_string() {
        let conllu = r#"1	helped	help	VERB	VBD	_	0	root	_	_
2	us	we	PRON	PRP	_	1	obj	_	_

1	ran	run	VERB	VBD	_	0	root	_	_
2	quickly	quickly	ADV	RB	_	1	advmod	_	_

1	sleeps	sleep	VERB	VBZ	_	0	root	_	_

"#;

        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(conllu, pattern).collect();

        // Should find 3 verbs total (one in each tree)
        assert_eq!(matches.len(), 3);

        // Check tree indices
        assert_eq!(matches[0].0, 1); // First tree (tree_idx starts at 1)
        assert_eq!(matches[1].0, 2); // Second tree
        assert_eq!(matches[2].0, 3); // Third tree

        // Check that each match found the verb (word 0 in each tree)
        assert_eq!(matches[0].2, vec![0]);
        assert_eq!(matches[1].2, vec![0]);
        assert_eq!(matches[2].2, vec![0]);
    }

    #[test]
    fn test_match_iterator_multiple_matches_per_tree() {
        let conllu = r#"1	saw	see	VERB	VBD	_	0	root	_	_
2	John	John	PROPN	NNP	_	1	obj	_	_
3	running	run	VERB	VBG	_	1	xcomp	_	_

"#;

        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(conllu, pattern).collect();

        // Should find 2 verbs in the single tree
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].0, 1); // Both from tree 1
        assert_eq!(matches[1].0, 1);
    }

    #[test]
    fn test_match_iterator_no_matches() {
        let conllu = r#"1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	0	root	_	_

"#;

        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(conllu, pattern).collect();

        // Should find no verbs
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_match_iterator_with_constraints() {
        let conllu = r#"1	helped	help	VERB	VBD	_	0	root	_	_
2	us	we	PRON	PRP	_	1	obj	_	_
3	to	to	PART	TO	_	4	mark	_	_
4	win	win	VERB	VB	_	1	xcomp	_	_

"#;

        let pattern = parse_query("V1 [lemma=\"help\"]; V2 [lemma=\"win\"]; V1 -> V2;").unwrap();
        let matches: Vec<_> = MatchIterator::from_string(conllu, pattern).collect();

        // Should find the help->win relationship
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].2, vec![0, 3]); // helped (word 0) -> win (word 3)
    }

    #[test]
    fn test_multi_file_tree_iterator_from_paths() {
        use std::fs;
        use std::io::Write;
        use tempfile::tempdir;

        // Create temporary directory and files
        let dir = tempdir().unwrap();

        let file1_path = dir.path().join("file1.conllu");
        let mut file1 = fs::File::create(&file1_path).unwrap();
        writeln!(
            file1,
            "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        let file2_path = dir.path().join("file2.conllu");
        let mut file2 = fs::File::create(&file2_path).unwrap();
        writeln!(
            file2,
            "1\tCats\tcat\tNOUN\tNNS\t_\t2\tnsubj\t_\t_\n2\tsleep\tsleep\tVERB\tVBP\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        // Test with explicit paths
        let paths = vec![file1_path.clone(), file2_path.clone()];
        let results: Vec<_> = MultiFileTreeIterator::from_paths(paths)
            .collect();

        assert_eq!(results.len(), 2);

        // Check first tree
        assert_eq!(results[0].0, file1_path);
        assert!(results[0].1.is_ok());
        assert_eq!(results[0].1.as_ref().unwrap().words.len(), 2);

        // Check second tree
        assert_eq!(results[1].0, file2_path);
        assert!(results[1].1.is_ok());
        assert_eq!(results[1].1.as_ref().unwrap().words.len(), 2);
    }

    #[test]
    fn test_multi_file_tree_iterator_from_glob() {
        use std::fs;
        use std::io::Write;
        use tempfile::tempdir;

        // Create temporary directory and files
        let dir = tempdir().unwrap();

        let file1_path = dir.path().join("test1.conllu");
        let mut file1 = fs::File::create(&file1_path).unwrap();
        writeln!(
            file1,
            "1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n2\tdog\tdog\tNOUN\tNN\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        let file2_path = dir.path().join("test2.conllu");
        let mut file2 = fs::File::create(&file2_path).unwrap();
        writeln!(
            file2,
            "1\tCats\tcat\tNOUN\tNNS\t_\t2\tnsubj\t_\t_\n2\tsleep\tsleep\tVERB\tVBP\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        // Create a non-matching file
        let _other_file = dir.path().join("other.txt");
        fs::File::create(&_other_file).unwrap();

        // Test with glob pattern
        let pattern = format!("{}/*.conllu", dir.path().display());
        let results: Vec<_> = MultiFileTreeIterator::from_glob(&pattern)
            .unwrap()
            .collect();

        assert_eq!(results.len(), 2);

        // Results should be sorted by filename
        assert!(results[0].0.to_str().unwrap().contains("test1.conllu"));
        assert!(results[1].0.to_str().unwrap().contains("test2.conllu"));

        // Both should parse successfully
        assert!(results[0].1.is_ok());
        assert!(results[1].1.is_ok());
    }

    #[test]
    fn test_multi_file_match_iterator_from_paths() {
        use std::fs;
        use std::io::Write;
        use tempfile::tempdir;

        // Create temporary directory and files with verbs
        let dir = tempdir().unwrap();

        let file1_path = dir.path().join("file1.conllu");
        let mut file1 = fs::File::create(&file1_path).unwrap();
        writeln!(
            file1,
            "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        let file2_path = dir.path().join("file2.conllu");
        let mut file2 = fs::File::create(&file2_path).unwrap();
        writeln!(
            file2,
            "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        // Search for verbs across files
        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let paths = vec![file1_path.clone(), file2_path.clone()];
        let results: Vec<_> = MultiFileMatchIterator::from_paths(paths, pattern).collect();

        assert_eq!(results.len(), 2);

        // Check first match
        assert_eq!(results[0].0, file1_path);
        assert_eq!(results[0].2, vec![0]); // Word 0 = "runs"

        // Check second match
        assert_eq!(results[1].0, file2_path);
        assert_eq!(results[1].2, vec![0]); // Word 0 = "sleeps"
    }

    #[test]
    fn test_multi_file_match_iterator_from_glob() {
        use std::fs;
        use std::io::Write;
        use tempfile::tempdir;

        // Create temporary directory and files
        let dir = tempdir().unwrap();

        let file1_path = dir.path().join("a.conllu");
        let mut file1 = fs::File::create(&file1_path).unwrap();
        writeln!(
            file1,
            "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        let file2_path = dir.path().join("b.conllu");
        let mut file2 = fs::File::create(&file2_path).unwrap();
        writeln!(
            file2,
            "1\tsleeps\tsleep\tVERB\tVBZ\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        // Search for verbs across files
        let pattern = parse_query("V [pos=\"VERB\"];").unwrap();
        let glob_pattern = format!("{}/*.conllu", dir.path().display());
        let results: Vec<_> =
            MultiFileMatchIterator::from_glob(&glob_pattern, pattern)
                .unwrap()
                .collect();

        assert_eq!(results.len(), 2);

        // Results should be sorted (a.conllu before b.conllu)
        assert!(results[0].0.to_str().unwrap().contains("a.conllu"));
        assert!(results[1].0.to_str().unwrap().contains("b.conllu"));
    }

    #[test]
    fn test_multi_file_iterator_skips_bad_files() {
        use std::fs;
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();

        // Create one good file
        let good_file = dir.path().join("good.conllu");
        let mut file = fs::File::create(&good_file).unwrap();
        writeln!(
            file,
            "1\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n"
        )
        .unwrap();

        // Reference a non-existent file
        let bad_file = dir.path().join("nonexistent.conllu");

        let paths = vec![good_file.clone(), bad_file, good_file.clone()];
        let results: Vec<_> = MultiFileTreeIterator::from_paths(paths)
            .filter_map(|(_path, result)| result.ok())
            .collect();

        // Should get 2 trees (good file appears twice, bad file skipped)
        assert_eq!(results.len(), 2);
    }
}
