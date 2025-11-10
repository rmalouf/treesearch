//! Tree data structures for dependency parsing
//!
//! This module provides complete CoNLL-U support including all fields,
//! morphological features, enhanced dependencies, and metadata.

use std::collections::HashMap;

/// Unique identifier for a node (index in tree's nodes vector)
pub type NodeId = usize;

/// Token ID from CoNLL-U file (1-based integer)
pub type TokenId = usize;

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

    // CoNLL-U ID field (1-based token number from file)
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
            .map(|&id| tree.get_node_unchecked(id))
            .filter(|child| child.deprel == deprel)
            .collect()
    }

    /// Get the parent node
    ///
    /// Returns the parent node if one exists.
    ///
    /// # Arguments
    /// * `tree` - Reference to the tree containing this node
    pub fn parent<'a>(&self, tree: &'a Tree) -> Option<&'a Node> {
        self.parent
            .map(|parent_id| tree.get_node_unchecked(parent_id))
    }

    /// Get all children nodes
    ///
    /// Returns all children of this node.
    ///
    /// # Arguments
    /// * `tree` - Reference to the tree containing this node
    pub fn children<'a>(&self, tree: &'a Tree) -> Vec<&'a Node> {
        self.children
            .iter()
            .map(|&id| tree.get_node_unchecked(id))
            .collect()
    }

    /// Create a new node with minimal attributes (for Phase 0 compatibility)
    pub fn new(id: NodeId, form: &str, lemma: &str, pos: &str, deprel: &str) -> Self {
        Self {
            id,
            position: id, // Default: position = id
            token_id: id, // Default: token_id = id (1-based)
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
    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    /// Get a node by ID (safe external API)
    ///
    /// Returns `Ok(&Node)` if the node exists, or `Err` with a descriptive message if not.
    /// Use this for external callers where the node ID might be invalid user input.
    pub fn get_node(&self, id: NodeId) -> Result<&Node, String> {
        let Some(node) = self.nodes.get(id) else {
            return Err(format!(
                "Node with id {} does not exist (tree has {} nodes)",
                id,
                self.nodes.len()
            ));
        };
        Ok(node)
    }

    /// Get a node by ID (unchecked internal API)
    ///
    /// Returns a reference to the node, panicking if the ID is invalid.
    /// This is for internal use where invalid IDs indicate bugs, not user errors.
    ///
    /// # Panics
    ///
    /// Panics if the node ID is out of bounds.
    pub(crate) fn get_node_unchecked(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    /// Set the parent of a node
    ///
    /// # Panics
    ///
    /// Panics if either the child node or parent node doesn't exist.
    /// This is an internal API for tree construction - invalid node IDs indicate a bug.
    pub fn set_parent(&mut self, child_id: NodeId, parent_id: NodeId) {
        // Validate both nodes exist
        assert!(
            child_id < self.nodes.len(),
            "Child node with id {} does not exist (tree has {} nodes)",
            child_id,
            self.nodes.len()
        );
        assert!(
            parent_id < self.nodes.len(),
            "Parent node with id {} does not exist (tree has {} nodes)",
            parent_id,
            self.nodes.len()
        );

        // Both exist, safe to modify
        self.nodes[child_id].parent = Some(parent_id);
        self.nodes[parent_id].children.push(child_id);
    }

    /// Get the parent ID of a node
    ///
    /// Returns `Ok(Some(parent_id))` if the node exists and has a parent,
    /// `Ok(None)` if the node exists but has no parent,
    /// or `Err` if the node doesn't exist.
    pub fn parent_id(&self, node_id: NodeId) -> Result<Option<NodeId>, String> {
        Ok(self.get_node(node_id)?.parent)
    }

    /// Get the children IDs of a node
    ///
    /// Returns `Ok(vec)` with the children IDs if the node exists,
    /// or `Err` if the node doesn't exist.
    pub fn children_ids(&self, node_id: NodeId) -> Result<Vec<NodeId>, String> {
        Ok(self.get_node(node_id)?.children.clone())
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
        assert_eq!(tree.parent_id(1).unwrap(), Some(0));
        assert_eq!(tree.children_ids(0).unwrap().len(), 1);
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
