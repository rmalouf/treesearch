//! Tree data structures for dependency parsing
//!
//! This module provides complete CoNLL-U support including all fields,
//! morphological features, enhanced dependencies, and metadata.

use std::collections::HashMap;

/// Unique identifier for a node (index in tree's nodes vector)
pub type NodeId = usize;

/// Token ID from CoNLL-U (can be single, range, or decimal)
#[derive(Debug, Clone, PartialEq)]
pub enum TokenId {
    /// Normal token: 1, 2, 3, ...
    Single(usize),
    /// Multiword token: 1-2, 3-4, ...
    Range(usize, usize),
    /// Empty node: 2.1, 3.1, ...
    Decimal(usize, usize),
}

impl TokenId {
    /// Get the primary index (first number in all cases)
    pub fn primary(&self) -> usize {
        match self {
            TokenId::Single(n) => *n,
            TokenId::Range(start, _) => *start,
            TokenId::Decimal(n, _) => *n,
        }
    }
}

/// Morphological features (key=value pairs)
pub type Features = HashMap<String, String>;

/// Enhanced dependency (for DEPS field)
#[derive(Debug, Clone, PartialEq)]
pub struct Dep {
    pub head: Option<NodeId>,
    pub deprel: String,
}

/// Miscellaneous annotations (key=value pairs)
pub type Misc = HashMap<String, String>;

/// A node in a dependency tree
#[derive(Debug, Clone)]
pub struct Node {
    // Node identifier (index in tree)
    pub id: NodeId,

    // Linear position for leftmost semantics (Phase 1)
    pub position: usize,

    // CoNLL-U ID field (can be range or decimal)
    pub token_id: TokenId,

    // CoNLL-U fields
    pub form: String,         // FORM
    pub lemma: String,        // LEMMA
    pub pos: String,          // UPOS (universal POS)
    pub xpos: Option<String>, // XPOS (language-specific POS)
    pub feats: Features,      // FEATS (morphological features)
    pub deprel: String,       // DEPREL (dependency relation)
    pub deps: Vec<Dep>,       // DEPS (enhanced dependencies)
    pub misc: Misc,           // MISC (miscellaneous)

    // Tree structure (computed from HEAD field)
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
}

impl Node {
    /// Get the parent node ID
    pub fn parent_id(&self) -> Option<NodeId> {
        self.parent
    }

    /// Get the children node IDs
    pub fn children_ids(&self) -> &[NodeId] {
        &self.children
    }

    /// Get all children with a specific dependency relation
    ///
    /// Returns all children that have the specified dependency relation.
    /// Useful for relations that can have multiple dependents (e.g., "conj" in
    /// coordinated structures).
    ///
    /// # Arguments
    /// * `tree` - Reference to the tree containing this node
    /// * `deprel` - The dependency relation name to search for (e.g., "conj", "obl")
    ///
    /// # Examples
    /// ```
    /// # use treesearch::{Tree, Node};
    /// let mut tree = Tree::new();
    /// tree.add_node(Node::new(0, "and", "and", "CCONJ", "root"));
    /// tree.add_node(Node::new(1, "cats", "cat", "NOUN", "conj"));
    /// tree.add_node(Node::new(2, "dogs", "dog", "NOUN", "conj"));
    /// tree.set_parent(1, 0);
    /// tree.set_parent(2, 0);
    ///
    /// let coord = tree.get_node(0).unwrap();
    /// let conjuncts = coord.children_by_deprel(&tree, "conj");
    /// assert_eq!(conjuncts.len(), 2);
    /// ```
    pub fn children_by_deprel<'a>(&self, tree: &'a Tree, deprel: &str) -> Vec<&'a Node> {
        self.children
            .iter()
            .filter_map(|&id| tree.get_node(id))
            .filter(|child| child.deprel == deprel)
            .collect()
    }

    /// Create a new node with minimal attributes (for Phase 0 compatibility)
    pub fn new(id: NodeId, form: &str, lemma: &str, pos: &str, deprel: &str) -> Self {
        Self {
            id,
            position: id, // Default: position = id
            token_id: TokenId::Single(id),
            form: form.to_string(),
            lemma: lemma.to_string(),
            pos: pos.to_string(),
            xpos: None,
            feats: Features::new(),
            deprel: deprel.to_string(),
            deps: Vec::new(),
            misc: Misc::new(),
            parent: None,
            children: Vec::new(),
        }
    }

    /// Create a new node with full CoNLL-U fields
    #[allow(clippy::too_many_arguments)]
    pub fn with_full_fields(
        id: NodeId,
        position: usize,
        token_id: TokenId,
        form: String,
        lemma: String,
        pos: String,
        xpos: Option<String>,
        feats: Features,
        deprel: String,
        deps: Vec<Dep>,
        misc: Misc,
    ) -> Self {
        Self {
            id,
            position,
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
    pub(crate) nodes: Vec<Node>,
    pub root_id: Option<NodeId>,

    // Sentence metadata (from CoNLL-U comments)
    pub sentence_text: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Tree {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root_id: None,
            sentence_text: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new tree with sentence metadata
    pub fn with_metadata(sentence_text: Option<String>, metadata: HashMap<String, String>) -> Self {
        Self {
            nodes: Vec::new(),
            root_id: None,
            sentence_text,
            metadata,
        }
    }

    /// Add a node to the tree
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// Get a node by ID
    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Set the parent of a node
    pub fn set_parent(&mut self, child_id: NodeId, parent_id: NodeId) {
        if let Some(child) = self.nodes.get_mut(child_id) {
            child.parent = Some(parent_id);
        }
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children.push(child_id);
        }
    }

    /// Get the children of a node
    pub fn children(&self, node_id: NodeId) -> Vec<&Node> {
        if let Some(node) = self.get_node(node_id) {
            node.children.iter().map(|&id| &self.nodes[id]).collect()
        } else {
            Vec::new()
        }
    }

    /// Get the parent of a node
    pub fn parent(&self, node_id: NodeId) -> Option<&Node> {
        self.get_node(node_id)
            .and_then(|node| node.parent)
            .and_then(|parent_id| self.get_node(parent_id))
    }

    /// Get all nodes in the tree
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Get the number of nodes in the tree
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_creation() {
        let mut tree = Tree::new();
        let root = Node::new(0, "runs", "run", "VERB", "root");
        let child = Node::new(1, "dog", "dog", "NOUN", "nsubj");

        tree.add_node(root);
        tree.add_node(child);
        tree.set_parent(1, 0);

        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.parent(1).unwrap().id, 0);
        assert_eq!(tree.children(0).len(), 1);
    }

    #[test]
    fn test_children_by_deprel_multiple_matches() {
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "and", "and", "CCONJ", "root"));
        tree.add_node(Node::new(1, "cats", "cat", "NOUN", "conj"));
        tree.add_node(Node::new(2, "dogs", "dog", "NOUN", "conj"));
        tree.add_node(Node::new(3, "birds", "bird", "NOUN", "conj"));
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 0);

        let coord = tree.get_node(0).unwrap();

        let conjuncts = coord.children_by_deprel(&tree, "conj");
        assert_eq!(conjuncts.len(), 3);
        assert_eq!(conjuncts[0].lemma, "cat");
        assert_eq!(conjuncts[1].lemma, "dog");
        assert_eq!(conjuncts[2].lemma, "bird");
    }

    #[test]
    fn test_children_by_deprel_single_match() {
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "quickly", "quickly", "ADV", "advmod"));
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);

        let verb = tree.get_node(0).unwrap();

        let subjects = verb.children_by_deprel(&tree, "nsubj");
        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].lemma, "dog");
    }

    #[test]
    fn test_children_by_deprel_no_matches() {
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.set_parent(1, 0);

        let verb = tree.get_node(0).unwrap();

        let objects = verb.children_by_deprel(&tree, "obj");
        assert_eq!(objects.len(), 0);
    }

    #[test]
    fn test_children_by_deprel_mixed_children() {
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "park", "park", "NOUN", "obl"));
        tree.add_node(Node::new(3, "store", "store", "NOUN", "obl"));
        tree.add_node(Node::new(4, "quickly", "quickly", "ADV", "advmod"));
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 0);
        tree.set_parent(4, 0);

        let verb = tree.get_node(0).unwrap();

        // Should only return obl children
        let obliques = verb.children_by_deprel(&tree, "obl");
        assert_eq!(obliques.len(), 2);
        assert_eq!(obliques[0].lemma, "park");
        assert_eq!(obliques[1].lemma, "store");
    }
}
