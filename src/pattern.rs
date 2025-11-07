//! Pattern representation and compilation
//!
//! This module defines the AST for dependency tree patterns and
//! handles compilation to VM bytecode.

use crate::tree::NodeId;

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
pub struct PatternElement {
    /// Variable name to bind matched node to
    pub var_name: String,
    /// Constraints that the node must satisfy
    pub constraints: Constraint,
}

impl PatternElement {
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
    /// Pattern elements (nodes)
    pub elements: Vec<PatternElement>,
    /// Edges connecting the elements
    pub edges: Vec<PatternEdge>,
}

impl Pattern {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add a pattern element
    pub fn add_element(&mut self, element: PatternElement) {
        self.elements.push(element);
    }

    /// Add an edge between elements
    pub fn add_edge(&mut self, edge: PatternEdge) {
        self.edges.push(edge);
    }

    /// Get the index of an element by variable name
    pub fn element_index(&self, var_name: &str) -> Option<usize> {
        self.elements.iter().position(|e| e.var_name == var_name)
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

        let verb = PatternElement::new("verb", Constraint::POS("VERB".to_string()));
        let noun = PatternElement::new("noun", Constraint::POS("NOUN".to_string()));

        pattern.add_element(verb);
        pattern.add_element(noun);

        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Child,
            label: Some("nsubj".to_string()),
        });

        assert_eq!(pattern.elements.len(), 2);
        assert_eq!(pattern.edges.len(), 1);
    }
}
