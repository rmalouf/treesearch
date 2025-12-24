//! Tree data structures for dependency parsing

use crate::bytes::{BytestringPool, Sym};
use std::collections::HashMap;

/// Word index in tree (0-based)
pub type WordId = usize;

/// Token ID from CoNLL-U (1-based)
pub type TokenId = usize;

/// Morphological features (key=value pairs)
pub type Features = Vec<(Sym, Sym)>;

/// Enhanced dependency (DEPS field)
#[derive(Debug, Clone, PartialEq)]
pub struct Dep {
    pub head: Option<WordId>,
    pub deprel: Sym,
}

/// Miscellaneous annotations (MISC field)
pub type Misc = HashMap<String, String>;

/// A word in a dependency tree
#[derive(Debug, Clone)]
pub struct Word {
    pub id: WordId,
    pub token_id: TokenId,
    pub form: Sym,
    pub lemma: Sym,
    pub upos: Sym,
    pub xpos: Sym,
    pub feats: Features,
    pub head: Option<WordId>,
    pub deprel: Sym,
    pub misc: Features,
    pub children: Vec<WordId>,
}

impl Word {
    pub fn new_minimal(
        id: WordId,
        form: Sym,
        lemma: Sym,
        upos: Sym,
        xpos: Sym,
        head: Option<WordId>,
        deprel: Sym,
    ) -> Self {
        Self {
            id,
            token_id: id,
            form,
            lemma,
            upos,
            xpos,
            feats: Features::new(),
            head,
            deprel,
            misc: Features::new(),
            children: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: WordId,
        token_id: TokenId,
        form: Sym,
        lemma: Sym,
        upos: Sym,
        xpos: Sym,
        feats: Features,
        head: Option<WordId>,
        deprel: Sym,
        misc: Features,
    ) -> Self {
        Self {
            id,
            token_id,
            form,
            lemma,
            upos,
            xpos,
            feats,
            head,
            deprel,
            misc,
            children: Vec::new(),
        }
    }

    pub fn children_by_deprel<'a>(&self, tree: &'a Tree, deprel: &str) -> Vec<&'a Word> {
        self.children
            .iter()
            .map(|&id| &tree.words[id])
            .filter(|child| *tree.string_pool.resolve(child.deprel) == *deprel.as_bytes())
            .collect()
    }

    pub fn parent<'a>(&self, tree: &'a Tree) -> Option<&'a Word> {
        let id = self.head?;
        Some(&tree.words[id])
    }

    pub fn children<'a>(&self, tree: &'a Tree) -> Vec<&'a Word> {
        self.children.iter().map(|&id| &tree.words[id]).collect()
    }
}

/// A dependency tree (sentence)
#[derive(Debug, Clone)]
pub struct Tree {
    pub words: Vec<Word>,
    pub root_id: Option<WordId>,
    pub sentence_text: Option<String>,
    pub metadata: HashMap<String, String>,
    pub string_pool: BytestringPool,
}

impl Tree {
    pub fn new(string_pool: &BytestringPool) -> Self {
        Self {
            words: Vec::with_capacity(25),
            root_id: None,
            sentence_text: None,
            metadata: HashMap::new(),
            string_pool: string_pool.clone(),
        }
    }

    pub fn with_metadata(
        string_pool: &BytestringPool,
        sentence_text: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            words: Vec::with_capacity(50),
            root_id: None,
            sentence_text,
            metadata,
            string_pool: string_pool.clone(),
        }
    }

    pub fn add_minimal_word(
        &mut self,
        id: WordId,
        form: &[u8],
        lemma: &[u8],
        upos: &[u8],
        xpos: &[u8],
        head: Option<WordId>,
        deprel: &[u8],
    ) {
        let form_sym = self.string_pool.get_or_intern(form);
        let lemma_sym = self.string_pool.get_or_intern(lemma);
        let upos_sym = self.string_pool.get_or_intern(upos);
        let xpos_sym = self.string_pool.get_or_intern(xpos);
        let deprel_sym = self.string_pool.get_or_intern(deprel);
        let word = Word::new_minimal(
            id, form_sym, lemma_sym, upos_sym, xpos_sym, head, deprel_sym,
        );
        self.words.push(word);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_word(
        &mut self,
        word_id: WordId,
        token_id: TokenId,
        form: &[u8],
        lemma: &[u8],
        upos: &[u8],
        xpos: &[u8],
        feats: Features,
        head: Option<WordId>,
        deprel: &[u8],
        misc: Features,
    ) {
        let form_sym = self.string_pool.get_or_intern(form);
        let lemma_sym = self.string_pool.get_or_intern(lemma);
        let upos_sym = self.string_pool.get_or_intern(upos);
        let xpos_sym = self.string_pool.get_or_intern(xpos);
        let deprel_sym = self.string_pool.get_or_intern(deprel);

        let word = Word::new(
            word_id, token_id, form_sym, lemma_sym, upos_sym, xpos_sym, feats, head, deprel_sym,
            misc,
        );
        self.words.push(word);
    }

    /// Fill in children
    pub fn compile_tree(&mut self) {
        for word_id in 0..self.words.len() {
            if let Some(head) = self.words[word_id].head {
                self.words[head].children.push(word_id);
            } else {
                self.root_id = Some(word_id);
            }
        }
    }

    pub fn word(&self, id: WordId) -> Result<&Word, String> {
        let Some(word) = self.words.get(id) else {
            return Err(format!(
                "Word with id {} does not exist (tree has {} words)",
                id,
                self.words.len()
            ));
        };
        Ok(word)
    }

    /*
    /// Set parent-child relationship (panics if word IDs invalid)
    pub fn set_parent(&mut self, child_id: WordId, parent_id: WordId) {
        assert!(
            child_id < self.words.len(),
            "Child word with id {} does not exist (tree has {} words)",
            child_id,
            self.words.len()
        );
        assert!(
            parent_id < self.words.len(),
            "Parent words with id {} does not exist (tree has {} words)",
            parent_id,
            self.words.len()
        );

        self.words[child_id].parent = Some(parent_id);
        self.words[parent_id].children.push(child_id);
    }
    */

    pub fn head_id(&self, word_id: WordId) -> Result<Option<WordId>, String> {
        Ok(self.word(word_id)?.head)
    }

    pub fn children_ids(&self, word_id: WordId) -> Result<Vec<WordId>, String> {
        Ok(self.word(word_id)?.children.clone())
    }

    pub fn check_rel(&self, from_id: WordId, to_id: WordId) -> bool {
        self.words[from_id].children.contains(&to_id)
    }

    /// Find dependency path from ancestor X to descendant Y.
    /// Returns None if X and Y are the same node or if no path exists.
    /// Returns Some(vec![X, ..., Y]) if Y is a descendant of X.
    ///
    /// # Examples
    ///
    /// ```
    /// # use treesearch::Tree;
    /// let mut tree = Tree::default();
    /// tree.add_minimal_word(0, b"runs", b"run", b"VERB", b"_", None, b"root");
    /// tree.add_minimal_word(1, b"dog", b"dog", b"NOUN", b"_", Some(0), b"nsubj");
    /// tree.compile_tree();
    ///
    /// let x = &tree.words[0];
    /// let y = &tree.words[1];
    /// let path = tree.find_path(x, y).unwrap();
    /// assert_eq!(path.len(), 2);
    /// assert_eq!(path[0].id, 0);
    /// assert_eq!(path[1].id, 1);
    /// ```
    pub fn find_path<'a>(&'a self, x: &'a Word, y: &'a Word) -> Option<Vec<&'a Word>> {
        // Return None if same node
        if x.id == y.id {
            return None;
        }

        let mut path = vec![x];
        self.dfs_find_path(x, y, &mut path)
    }

    /// Helper method for find_path: DFS traversal to find target node.
    fn dfs_find_path<'a>(
        &'a self,
        current: &'a Word,
        target: &'a Word,
        path: &mut Vec<&'a Word>,
    ) -> Option<Vec<&'a Word>> {
        // Check each child
        for &child_id in &current.children {
            let child = &self.words[child_id];

            // Found target
            if child.id == target.id {
                path.push(child);
                return Some(path.clone());
            }

            // Recursively search in child's subtree
            path.push(child);
            if let Some(found) = self.dfs_find_path(child, target, path) {
                return Some(found);
            }
            path.pop(); // backtrack
        }

        None
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

impl Default for Tree {
    fn default() -> Self {
        let string_pool = BytestringPool::new();
        Self::new(&string_pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_creation() {
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"runs", b"run", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"dog", b"dog", b"NOUN", b"_", Some(0), b"nsubj");
        tree.compile_tree();

        assert_eq!(tree.words.len(), 2);
        assert_eq!(tree.head_id(1).unwrap(), Some(0));
        assert_eq!(tree.children_ids(0).unwrap().len(), 1);
    }

    #[test]
    fn test_children_by_deprel() {
        // Test multiple matches
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"and", b"and", b"CCONJ", b"_", None, b"root");
        tree.add_minimal_word(1, b"cats", b"cat", b"NOUN", b"_", Some(0), b"conj");
        tree.add_minimal_word(2, b"dogs", b"dog", b"NOUN", b"_", Some(0), b"conj");
        tree.add_minimal_word(3, b"birds", b"bird", b"NOUN", b"_", Some(0), b"conj");
        tree.compile_tree();

        let coord = tree.word(0).unwrap();
        let conjuncts = coord.children_by_deprel(&tree, "conj");
        assert_eq!(conjuncts.len(), 3);

        // Test single match
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"runs", b"run", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"dog", b"dog", b"NOUN", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"quickly", b"quickly", b"ADV", b"_", Some(0), b"advmod");
        tree.compile_tree();

        let verb = tree.word(0).unwrap();
        let subjects = verb.children_by_deprel(&tree, "nsubj");
        assert_eq!(subjects.len(), 1);

        // Test no matches
        let objects = verb.children_by_deprel(&tree, "obj");
        assert_eq!(objects.len(), 0);

        // Test filtering among mixed children
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"runs", b"run", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"dog", b"dog", b"NOUN", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"park", b"park", b"NOUN", b"_", Some(0), b"obl");
        tree.add_minimal_word(3, b"store", b"store", b"NOUN", b"_", Some(0), b"obl");
        tree.add_minimal_word(4, b"quickly", b"quickly", b"ADV", b"_", Some(0), b"advmod");
        tree.compile_tree();

        let verb = tree.word(0).unwrap();
        let obliques = verb.children_by_deprel(&tree, "obl");
        assert_eq!(obliques.len(), 2);
    }

    #[test]
    fn test_find_path() {
        // Tree structure:
        //       runs (0)
        //      /    \
        //   dog(1)  park(2)
        //    |        |
        //  big(3)   the(4)
        let mut tree = Tree::default();
        tree.add_minimal_word(0, b"runs", b"run", b"VERB", b"_", None, b"root");
        tree.add_minimal_word(1, b"dog", b"dog", b"NOUN", b"_", Some(0), b"nsubj");
        tree.add_minimal_word(2, b"park", b"park", b"NOUN", b"_", Some(0), b"obl");
        tree.add_minimal_word(3, b"big", b"big", b"ADJ", b"_", Some(1), b"amod");
        tree.add_minimal_word(4, b"the", b"the", b"DET", b"_", Some(2), b"det");
        tree.compile_tree();

        // Direct child: 0 -> 1
        let path = tree.find_path(&tree.words[0], &tree.words[1]).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].id, 0);
        assert_eq!(path[1].id, 1);

        // Multi-level: 0 -> 1 -> 3
        let path = tree.find_path(&tree.words[0], &tree.words[3]).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id, 0);
        assert_eq!(path[1].id, 1);
        assert_eq!(path[2].id, 3);

        // Different branch: 0 -> 2 -> 4
        let path = tree.find_path(&tree.words[0], &tree.words[4]).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id, 0);
        assert_eq!(path[1].id, 2);
        assert_eq!(path[2].id, 4);

        // No path (siblings)
        assert!(tree.find_path(&tree.words[1], &tree.words[2]).is_none());

        // No path (reverse direction)
        assert!(tree.find_path(&tree.words[1], &tree.words[0]).is_none());

        // Same node
        assert!(tree.find_path(&tree.words[0], &tree.words[0]).is_none());
    }
}
