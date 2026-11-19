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

    /// All transitive dependents (children, grandchildren, etc.) of this word.
    pub fn descendants<'a>(&self, tree: &'a Tree) -> Vec<&'a Word> {
        let mut result = Vec::new();
        let mut stack = self.children.clone();
        while let Some(id) = stack.pop() {
            let word = &tree.words[id];
            result.push(word);
            stack.extend(&word.children);
        }
        result
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

    pub fn descendant_ids(&self, word_id: WordId) -> Result<Vec<WordId>, String> {
        Ok(self
            .word(word_id)?
            .descendants(self)
            .into_iter()
            .map(|word| word.id)
            .collect())
    }

    pub fn check_rel(&self, from_id: WordId, to_id: WordId) -> bool {
        self.words[from_id].children.contains(&to_id)
    }

    /// Find dependency path from ancestor X to descendant Y.
    /// Returns None if X and Y are the same node or if no path exists.
    /// Returns Some(vec![X, ..., Y]) if Y is a descendant of X.
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
    use crate::conllu::tree_from_rows;

    /// runs(0) -nsubj-> dog(1) -amod-> big(4)
    ///         -obl->   park(2)
    ///         -obl->   store(3)
    fn build_tree() -> Tree {
        tree_from_rows(&[
            "runs  run   VERB _ 0 root",
            "dog   dog   NOUN _ 1 nsubj",
            "park  park  NOUN _ 1 obl",
            "store store NOUN _ 1 obl",
            "big   big   ADJ  _ 2 amod",
        ])
    }

    #[test]
    fn test_structure() {
        let tree = build_tree();
        assert_eq!(tree.root_id, Some(0));
        assert_eq!(tree.head_id(0).unwrap(), None);
        assert_eq!(tree.head_id(4).unwrap(), Some(1));
        assert_eq!(tree.children_ids(0).unwrap(), [1, 2, 3]);
        assert!(tree.word(5).is_err());
        assert!(tree.head_id(5).is_err());

        let by_deprel = |deprel| tree.words[0].children_by_deprel(&tree, deprel).len();
        assert_eq!(by_deprel("obl"), 2);
        assert_eq!(by_deprel("nsubj"), 1);
        assert_eq!(by_deprel("obj"), 0);

        let mut descendants = tree.descendant_ids(0).unwrap();
        descendants.sort_unstable();
        assert_eq!(descendants, [1, 2, 3, 4]);
        assert_eq!(tree.descendant_ids(1).unwrap(), [4]);
        assert!(tree.descendant_ids(4).unwrap().is_empty());
    }

    #[test]
    fn test_find_path() {
        let tree = build_tree();
        let path = |x: WordId, y: WordId| {
            tree.find_path(&tree.words[x], &tree.words[y])
                .map(|p| p.iter().map(|w| w.id).collect::<Vec<_>>())
        };

        assert_eq!(path(0, 1), Some(vec![0, 1]));
        assert_eq!(path(0, 4), Some(vec![0, 1, 4]));
        assert_eq!(path(0, 3), Some(vec![0, 3]));
        assert_eq!(path(1, 2), None); // siblings
        assert_eq!(path(1, 0), None); // reverse direction
        assert_eq!(path(0, 0), None); // same node
    }
}
