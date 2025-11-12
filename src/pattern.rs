//! Pattern representation and compilation
//!
//! This module defines the AST for dependency tree patterns and
//! handles compilation to VM opcodes.

use std::collections::HashMap;
use std::fmt::Debug;

/// A constraint on a node's attributes
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Match any node
    Any,
    /// Match a specific lemma
    Lemma(String),
    /// Match a specific POS tag
    POS(String),
    /// Match a specific form
    Form(String),
    /// Match a specific dependency relation
    DepRel(String),
    /// Conjunction of constraints
    And(Vec<Constraint>),
    /// Disjunction of constraints
    Or(Vec<Constraint>),
}

impl Constraint {
    /// Check if a constraint is trivially true (matches anything)
    pub fn is_any(&self) -> bool {
        matches!(self, Constraint::Any)
    }
}

/// A pattern element representing a node in the pattern
#[derive(Debug, Clone)]
pub struct PatternNode {
    /// Variable name to bind matched node to
    pub var_name: String,
    /// Constraints that the node must satisfy
    pub constraints: Constraint,
}

impl PatternNode {
    pub fn new(var_name: &str, constraints: Constraint) -> Self {
        Self {
            var_name: var_name.to_string(),
            constraints,
        }
    }
}

/// Type of structural relation between nodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelationType {
    /// Direct parent-child relation
    Child,
    /// Direct child-parent relation
    Parent,
    /// Ancestor relation (transitive closure of parent)
    Ancestor,
    /// Descendant relation (transitive closure of child)
    Descendant,
    /// Linear precedence (left sibling)
    Precedes,
    /// Linear precedence (right sibling)
    Follows,
}

/// An edge in the pattern graph
#[derive(Debug, Clone)]
pub struct PatternEdge {
    /// Source node (by variable name)
    pub from: String,
    /// Target node (by variable name)
    pub to: String,
    /// Type of relation
    pub relation: RelationType,
    /// Optional constraint on the edge label (e.g., deprel)
    pub label: Option<String>,
}

/// A complete pattern to match against dependency trees
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Number of nodes
    pub n_nodes: usize,
    /// Node name -> NodeId
    pub node_names: HashMap<String, usize>,
    /// Out edge indices by node
    pub out_edges: Vec<Vec<usize>>,
    /// In edge indices by node
    pub in_edges: Vec<Vec<usize>>,

    pub required_parents: Vec<Vec<String>>,
    pub required_children: Vec<Vec<String>>,

    /// Pattern elements (nodes)
    pub nodes: Vec<PatternNode>,
    /// Edges connecting the elements
    pub edges: Vec<PatternEdge>,
    /// Already compiled?
    pub(crate) compiled: bool,
}

impl Pattern {
    pub fn new() -> Self {
        Self {
            n_nodes: 0,
            node_names: HashMap::new(),
            in_edges: Vec::new(),
            out_edges: Vec::new(),
            required_parents: Vec::new(),
            required_children: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            compiled: false,
        }
    }

    /// Add a pattern node
    pub fn add_node(&mut self, node: PatternNode) {
        self.nodes.push(node);
    }

    /// Add an edge between nodes
    pub fn add_edge(&mut self, edge: PatternEdge) {
        self.edges.push(edge);
    }

    pub fn compile_pattern(&mut self) {
        assert!(!self.compiled, "Can't compile pattern more than once!");

        // Compile nodes
        self.n_nodes = self.nodes.len();
        for node in self.nodes.iter() {
            let node_name = &node.var_name;
            if !self.node_names.contains_key(node_name) {
                self.node_names.insert(node_name.clone(), self.n_nodes);
                self.out_edges.push(Vec::new());
                self.in_edges.push(Vec::new());
                self.n_nodes += 1;
            }
            // TODO: check for duplicate nodes
        }

        // Compile edges
        for (edge_index, edge) in self.edges.iter().enumerate() {
            let from_node_id = self.node_names.get(&edge.from).unwrap();
            let to_node_id = self.node_names.get(&edge.from).unwrap();
            self.out_edges[*from_node_id].push(edge_index);
            self.in_edges[*to_node_id].push(edge_index);
            if let Some(label) = &edge.label {
                self.required_parents[*to_node_id].push(label.clone());
                self.required_children[*from_node_id].push(label.clone());
            }

            // Remove duplicates
            for node_id in 0..self.n_nodes {
                self.required_children[node_id].sort_unstable();
                self.required_children[node_id].dedup();
                self.required_parents[node_id].sort_unstable();
                self.required_parents[node_id].dedup();
            }

            self.compiled = true;
        }
    }

    /// Get the index of an element by variable name
    pub fn element_index(&self, var_name: &str) -> Option<usize> {
        self.nodes.iter().position(|e| e.var_name == var_name)
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let mut pattern = Pattern::new();

        let verb = PatternNode::new("verb", Constraint::POS("VERB".to_string()));
        let noun = PatternNode::new("noun", Constraint::POS("NOUN".to_string()));

        pattern.add_node(verb);
        pattern.add_node(noun);

        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Child,
            label: Some("nsubj".to_string()),
        });

        pattern.compile_pattern();

        assert_eq!(pattern.node_names.len(), 2);
        assert_eq!(pattern.nodes.len(), 2);
        assert_eq!(pattern.edges.len(), 1);
        // TODO: add more assertions
    }
}
