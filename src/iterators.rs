//! Iterators for trees and matches
//!
//! Provides convenient iterator interfaces for:
//! - Iterating over trees from a file
//! - Searching patterns across multiple trees

use crate::conllu::{CoNLLUReader, ParseError};
use crate::pattern::Pattern;
use crate::searcher::{Match, search};
use crate::tree::Tree;
use std::io::BufRead;
use std::path::Path;

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
}
