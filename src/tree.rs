//! Minimal tree data structures for pattern matching
//!
//! This module provides the basic data structures needed for testing
//! the pattern matching VM. Full CoNLL-U support will be added in Phase 1.

use std::rc::Rc;
use std::cell::RefCell;

/// Unique identifier for a node
pub type NodeId = usize;

/// A node in a dependency tree
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub form: String,
    pub lemma: String,
    pub pos: String,
    pub deprel: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

impl Node {
    /// Create a new node with the given attributes
    pub fn new(id: NodeId, form: &str, lemma: &str, pos: &str, deprel: &str) -> Self {
        Self {
            id,
            form: form.to_string(),
            lemma: lemma.to_string(),
            pos: pos.to_string(),
            deprel: deprel.to_string(),
            parent: None,
            children: Vec::new(),
        }
    }
}

/// A dependency tree (sentence)
#[derive(Debug, Clone)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub root_id: Option<NodeId>,
}

impl Tree {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root_id: None,
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

    /// Get a mutable reference to a node by ID
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Set the parent of a node
    pub fn set_parent(&mut self, child_id: NodeId, parent_id: NodeId) {
        if let Some(child) = self.get_node_mut(child_id) {
            child.parent = Some(parent_id);
        }
        if let Some(parent) = self.get_node_mut(parent_id) {
            parent.children.push(child_id);
        }
    }

    /// Get the children of a node
    pub fn children(&self, node_id: NodeId) -> Vec<&Node> {
        if let Some(node) = self.get_node(node_id) {
            node.children
                .iter()
                .filter_map(|&id| self.get_node(id))
                .collect()
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
}
