//! Tree data structures for dependency parsing

use lasso::{Capacity, Spur, ThreadedRodeo};
use rustc_hash::FxBuildHasher;
use std::collections::HashMap;
use std::sync::Arc;

pub const STRING_POOL_CAPACITY: usize = 5000;

/// Thread-safe string interner using FxHash for POS tags, XPOS, DEPREL
pub type StringPool = Arc<ThreadedRodeo<Spur, FxBuildHasher>>;

/// Create a new string pool with FxHash and pre-allocated capacity
pub fn create_string_pool() -> StringPool {
    Arc::new(ThreadedRodeo::with_capacity_and_hasher(
        Capacity::for_strings(STRING_POOL_CAPACITY),
        FxBuildHasher,
    ))
}

/// Word index in tree (0-based)
pub type WordId = usize;

/// Token ID from CoNLL-U (1-based)
pub type TokenId = usize;

/// Morphological features (key=value pairs)
pub type Features = Vec<(Spur, Spur)>;

/// Enhanced dependency (DEPS field)
#[derive(Debug, Clone, PartialEq)]
pub struct Dep {
    pub head: Option<WordId>,
    pub deprel: Spur,
}

/// Miscellaneous annotations (MISC field)
pub type Misc = HashMap<String, String>;

/// A word in a dependency tree
#[derive(Debug, Clone)]
pub struct Word {
    pub id: WordId,
    pub token_id: TokenId,

    // CoNLL-U fields
    pub form: String,
    pub lemma: String,
    pub pos: Spur,
    pub xpos: Option<Spur>,
    pub feats: Features,
    pub deprel: Spur,
    pub deps: Vec<Dep>,
    pub misc: Misc,

    // Tree structure
    pub(crate) parent: Option<WordId>,
    pub(crate) children: Vec<WordId>,
}

impl Word {
    pub fn children_by_deprel<'a>(&self, tree: &'a Tree, deprel: &str) -> Vec<&'a Word> {
        self.children
            .iter()
            .map(|&id| &tree.words[id])
            .filter(|child| tree.string_pool.resolve(&child.deprel) == deprel)
            .collect()
    }

    pub fn parent<'a>(&self, tree: &'a Tree) -> Option<&'a Word> {
        let id = self.parent?;
        Some(&tree.words[id])
    }

    pub fn children<'a>(&self, tree: &'a Tree) -> Vec<&'a Word> {
        self.children.iter().map(|&id| &tree.words[id]).collect()
    }

    pub fn new(id: WordId, form: &str, lemma: &str, pos: Spur, deprel: Spur) -> Self {
        Self {
            id,
            token_id: id,
            form: form.to_string(),
            lemma: lemma.to_string(),
            pos,
            xpos: None,
            feats: Features::new(),
            deprel,
            deps: Vec::new(),
            misc: Misc::new(),
            parent: None,
            children: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_full_fields(
        id: WordId,
        token_id: TokenId,
        form: String,
        lemma: String,
        pos: Spur,
        xpos: Option<Spur>,
        feats: Features,
        deprel: Spur,
        deps: Vec<Dep>,
        misc: Misc,
    ) -> Self {
        Self {
            id,
            token_id,
            form,
            lemma,
            pos,
            xpos,
            feats,
            deprel,
            deps,
            misc,
            parent: None,
            children: Vec::new(),
        }
    }
}

/// A dependency tree (sentence)
#[derive(Debug, Clone)]
pub struct Tree {
    pub(crate) words: Vec<Word>,
    pub root_id: Option<WordId>,
    pub sentence_text: Option<String>,
    pub metadata: HashMap<String, String>,
    pub string_pool: StringPool,
}

impl Tree {
    pub fn new(string_pool: &StringPool) -> Self {
        Self {
            words: Vec::with_capacity(25),
            root_id: None,
            sentence_text: None,
            metadata: HashMap::new(),
            string_pool: Arc::clone(string_pool),
        }
    }

    pub fn with_metadata(
        string_pool: &StringPool,
        sentence_text: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            words: Vec::with_capacity(50),
            root_id: None,
            sentence_text,
            metadata,
            string_pool: Arc::clone(string_pool),
        }
    }

    pub fn intern_string(&self, string: &str) -> Spur {
        self.string_pool.get_or_intern(string)
    }

    pub fn add_word(&mut self, id: WordId, form: &str, lemma: &str, pos: &str, deprel: &str) {
        let pos_spur = self.intern_string(pos);
        let deprel_spur = self.intern_string(deprel);
        let word = Word::new(id, form, lemma, pos_spur, deprel_spur);
        self.words.push(word);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_word_full_fields(
        &mut self,
        word_id: WordId,
        token_id: usize,
        form: String,
        lemma: String,
        pos: Spur,
        xpos: Option<Spur>,
        feats: Features,
        deprel: Spur,
        deps: Vec<Dep>,
        misc: Misc,
        head: Option<WordId>,
    ) {
        let mut word = Word::with_full_fields(
            word_id, token_id, form, lemma, pos, xpos, feats, deprel, deps, misc,
        );
        word.parent = head;
        self.words.push(word);
    }

    pub fn get_word(&self, id: WordId) -> Result<&Word, String> {
        let Some(word) = self.words.get(id) else {
            return Err(format!(
                "Word with id {} does not exist (tree has {} words)",
                id,
                self.words.len()
            ));
        };
        Ok(word)
    }

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

    pub fn parent_id(&self, word_id: WordId) -> Result<Option<WordId>, String> {
        Ok(self.get_word(word_id)?.parent)
    }

    pub fn children_ids(&self, word_id: WordId) -> Result<Vec<WordId>, String> {
        Ok(self.get_word(word_id)?.children.clone())
    }

    pub fn check_rel(&self, from_id: WordId, to_id: WordId) -> bool {
        self.words[from_id].children.contains(&to_id)
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
        let string_pool = create_string_pool();
        Self::new(&string_pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_creation() {
        let mut tree = Tree::default();
        tree.add_word(0, "runs", "run", "VERB", "root");
        tree.add_word(1, "dog", "dog", "NOUN", "nsubj");
        tree.set_parent(1, 0);

        assert_eq!(tree.words.len(), 2);
        assert_eq!(tree.parent_id(1).unwrap(), Some(0));
        assert_eq!(tree.children_ids(0).unwrap().len(), 1);
    }

    #[test]
    fn test_children_by_deprel_multiple_matches() {
        let mut tree = Tree::default();
        tree.add_word(0, "and", "and", "CCONJ", "root");
        tree.add_word(1, "cats", "cat", "NOUN", "conj");
        tree.add_word(2, "dogs", "dog", "NOUN", "conj");
        tree.add_word(3, "birds", "bird", "NOUN", "conj");
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 0);

        let coord = tree.get_word(0).unwrap();

        let conjuncts = coord.children_by_deprel(&tree, "conj");
        assert_eq!(conjuncts.len(), 3);
        assert_eq!(conjuncts[0].lemma, "cat");
        assert_eq!(conjuncts[1].lemma, "dog");
        assert_eq!(conjuncts[2].lemma, "bird");
    }

    #[test]
    fn test_children_by_deprel_single_match() {
        let mut tree = Tree::default();
        tree.add_word(0, "runs", "run", "VERB", "root");
        tree.add_word(1, "dog", "dog", "NOUN", "nsubj");
        tree.add_word(2, "quickly", "quickly", "ADV", "advmod");
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);

        let verb = tree.get_word(0).unwrap();

        let subjects = verb.children_by_deprel(&tree, "nsubj");
        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].lemma, "dog");
    }

    #[test]
    fn test_children_by_deprel_no_matches() {
        let mut tree = Tree::default();
        tree.add_word(0, "runs", "run", "VERB", "root");
        tree.add_word(1, "dog", "dog", "NOUN", "nsubj");
        tree.set_parent(1, 0);

        let verb = tree.get_word(0).unwrap();

        let objects = verb.children_by_deprel(&tree, "obj");
        assert_eq!(objects.len(), 0);
    }

    #[test]
    fn test_children_by_deprel_mixed_children() {
        let mut tree = Tree::default();
        tree.add_word(0, "runs", "run", "VERB", "root");
        tree.add_word(1, "dog", "dog", "NOUN", "nsubj");
        tree.add_word(2, "park", "park", "NOUN", "obl");
        tree.add_word(3, "store", "store", "NOUN", "obl");
        tree.add_word(4, "quickly", "quickly", "ADV", "advmod");
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 0);
        tree.set_parent(4, 0);

        let verb = tree.get_word(0).unwrap();

        // Should only return obl children
        let obliques = verb.children_by_deprel(&tree, "obl");
        assert_eq!(obliques.len(), 2);
        assert_eq!(obliques[0].lemma, "park");
        assert_eq!(obliques[1].lemma, "store");
    }
}
