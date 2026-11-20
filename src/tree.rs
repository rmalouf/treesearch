//! Tree data structures for dependency parsing

use crate::bytes::{BytestringPool, Sym};
use std::collections::HashMap;

/// Word index in tree (0-based)
pub type WordId = usize;

/// Morphological features (key=value pairs)
pub type Features = Vec<(Sym, Sym)>;

/// Node in the enhanced graph: a WordId for basic words (< `words.len()`), then empty nodes
/// (`words.len() + i` is `empty_nodes[i]`)
pub type NodeId = usize;

/// Enhanced dependency arc (DEPS field); `head` is None for `0:root`
#[derive(Debug, Clone, PartialEq)]
pub struct Dep {
    pub head: Option<NodeId>,
    pub deprel: Sym,
}

/// A word in a dependency tree
#[derive(Debug, Clone)]
pub struct Word {
    pub id: NodeId,
    /// CoNLL-U word ID (1-based); N for an empty node N.M
    pub token_id: usize,
    pub form: Sym,
    pub lemma: Sym,
    pub upos: Sym,
    pub xpos: Sym,
    pub feats: Features,
    pub head: Option<WordId>,
    pub deprel: Sym,
    pub misc: Features,
    pub children: Vec<WordId>,
    /// Incoming enhanced arcs (DEPS)
    pub enhanced_heads: Vec<Dep>,
    /// Outgoing enhanced arcs: (dependent, deprel)
    pub enhanced_children: Vec<(NodeId, Sym)>,
}

impl Word {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: NodeId,
        token_id: usize,
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
            enhanced_heads: Vec::new(),
            enhanced_children: Vec::new(),
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
    /// Empty nodes (CoNLL-U IDs like 5.1), in file order; only the enhanced graph uses them
    pub empty_nodes: Vec<Word>,
    pub root_id: Option<WordId>,
    pub sentence_text: Option<String>,
    pub metadata: HashMap<String, String>,
    pub string_pool: BytestringPool,
}

impl Tree {
    pub fn new(string_pool: &BytestringPool) -> Self {
        Self {
            words: Vec::with_capacity(25),
            empty_nodes: Vec::new(),
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
            empty_nodes: Vec::new(),
            root_id: None,
            sentence_text,
            metadata,
            string_pool: string_pool.clone(),
        }
    }

    /// Add a basic word, or an empty node after word `token_id` if `empty`
    #[allow(clippy::too_many_arguments)]
    pub fn add_word(
        &mut self,
        token_id: usize,
        empty: bool,
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

        let nodes = if empty {
            &mut self.empty_nodes
        } else {
            &mut self.words
        };
        // Empty nodes get their ids in compile_tree, once the number of words is known
        let word = Word::new(
            nodes.len(),
            token_id,
            form_sym,
            lemma_sym,
            upos_sym,
            xpos_sym,
            feats,
            head,
            deprel_sym,
            misc,
        );
        nodes.push(word);
    }

    /// Number empty nodes and fill in children, root and enhanced children
    pub fn compile_tree(&mut self) {
        let n = self.words.len();
        for (i, node) in self.empty_nodes.iter_mut().enumerate() {
            node.id = n + i;
        }
        for word_id in 0..n {
            if let Some(head) = self.words[word_id].head {
                self.words[head].children.push(word_id);
            } else {
                self.root_id = Some(word_id);
            }
        }
        for id in 0..self.num_nodes() {
            for i in 0..self.node_mut(id).enhanced_heads.len() {
                let Dep { head, deprel } = self.node_mut(id).enhanced_heads[i];
                if let Some(head) = head {
                    self.node_mut(head).enhanced_children.push((id, deprel));
                }
            }
        }
    }

    /// Words and empty nodes
    pub fn num_nodes(&self) -> usize {
        self.words.len() + self.empty_nodes.len()
    }

    /// Word or empty node by NodeId
    pub fn node(&self, id: NodeId) -> Result<&Word, String> {
        let n = self.words.len();
        let node = if id < n {
            self.words.get(id)
        } else {
            self.empty_nodes.get(id - n)
        };
        node.ok_or_else(|| {
            format!(
                "Node with id {id} does not exist (tree has {} nodes)",
                self.num_nodes()
            )
        })
    }

    /// Mutable word or empty node by NodeId (panics if out of range)
    pub fn node_mut(&mut self, id: NodeId) -> &mut Word {
        let n = self.words.len();
        if id < n {
            &mut self.words[id]
        } else {
            &mut self.empty_nodes[id - n]
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
