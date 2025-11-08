//! Inverted indices for efficient candidate lookup
//!
//! This module provides indexing structures to quickly find candidate
//! nodes that might match pattern constraints, before running the VM.

use crate::tree::{Node, NodeId, Tree};
use std::collections::HashMap;

/// Inverted index for tree nodes
#[derive(Debug, Clone)]
pub struct TreeIndex {
    /// Index by lemma
    by_lemma: HashMap<String, Vec<NodeId>>,
    /// Index by POS tag
    by_pos: HashMap<String, Vec<NodeId>>,
    /// Index by dependency relation
    by_deprel: HashMap<String, Vec<NodeId>>,
    /// Index by form
    by_form: HashMap<String, Vec<NodeId>>,
}

impl TreeIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            by_lemma: HashMap::new(),
            by_pos: HashMap::new(),
            by_deprel: HashMap::new(),
            by_form: HashMap::new(),
        }
    }

    /// Build an index from a tree
    pub fn build(tree: &Tree) -> Self {
        let mut index = Self::new();

        for node in &tree.nodes {
            index.add_node(node);
        }

        index
    }

    /// Add a node to the index
    fn add_node(&mut self, node: &Node) {
        // Index by lemma
        self.by_lemma
            .entry(node.lemma.clone())
            .or_default()
            .push(node.id);

        // Index by POS
        self.by_pos
            .entry(node.pos.clone())
            .or_default()
            .push(node.id);

        // Index by deprel
        self.by_deprel
            .entry(node.deprel.clone())
            .or_default()
            .push(node.id);

        // Index by form
        self.by_form
            .entry(node.form.clone())
            .or_default()
            .push(node.id);
    }

    /// Get candidate nodes by lemma
    pub fn get_by_lemma(&self, lemma: &str) -> Option<&[NodeId]> {
        self.by_lemma.get(lemma).map(|v| v.as_slice())
    }

    /// Get candidate nodes by POS tag
    pub fn get_by_pos(&self, pos: &str) -> Option<&[NodeId]> {
        self.by_pos.get(pos).map(|v| v.as_slice())
    }

    /// Get candidate nodes by dependency relation
    pub fn get_by_deprel(&self, deprel: &str) -> Option<&[NodeId]> {
        self.by_deprel.get(deprel).map(|v| v.as_slice())
    }

    /// Get candidate nodes by form
    pub fn get_by_form(&self, form: &str) -> Option<&[NodeId]> {
        self.by_form.get(form).map(|v| v.as_slice())
    }
}

impl Default for TreeIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_building() {
        let mut tree = Tree::new();
        let node1 = Node::new(0, "runs", "run", "VERB", "root");
        let node2 = Node::new(1, "dog", "dog", "NOUN", "nsubj");
        tree.add_node(node1);
        tree.add_node(node2);

        let index = TreeIndex::build(&tree);

        assert_eq!(index.get_by_lemma("run").unwrap(), &[0]);
        assert_eq!(index.get_by_pos("NOUN").unwrap(), &[1]);
    }
}
