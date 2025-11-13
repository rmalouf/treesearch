//! Pattern representation and compilation
//!
//! This module defines the AST for dependency tree patterns used
//! in the CSP-based matching algorithm.

use std::collections::HashMap;
use std::fmt::Debug;

/// Type alias for pattern variable identifiers (indices into Pattern.vars)
pub type VarId = usize;

/// A constraint on a variable's attributes (node attributes in matched tree)
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

/// A pattern variable representing a node in the dependency tree
#[derive(Debug, Clone)]
pub struct PatternVar {
    /// Variable name to bind matched tree node to
    pub var_name: String,
    /// Constraints that the matched tree node must satisfy
    pub constraints: Constraint,
}

impl PatternVar {
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
    /// Ancestor relation (transitive closure of parent)
    Ancestor,
    /// Descendant relation (transitive closure of child)
    Descendant,
    /// Linear precedence (left sibling)
    Precedes,
    /// Linear precedence (right sibling)
    Follows,
}

/// A constraint on the structural relationship between two pattern variables
#[derive(Debug, Clone)]
pub struct EdgeConstraint {
    /// Source variable (by variable name)
    pub from: String,
    /// Target variable (by variable name)
    pub to: String,
    /// Type of structural relation required
    pub relation: RelationType,
    /// Optional constraint on the edge label (e.g., deprel in the tree)
    pub label: Option<String>,
}

/// A complete pattern to match against dependency trees
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Number of variables in the pattern
    pub n_vars: usize,
    /// Variable name -> VarId mapping
    pub var_names: HashMap<String, VarId>,
    /// Outgoing edge constraint indices by variable
    pub out_edges: Vec<Vec<usize>>,
    /// Incoming edge constraint indices by variable
    pub in_edges: Vec<Vec<usize>>,
    /// Pattern variables
    pub vars: Vec<PatternVar>,
    /// Edge constraints connecting the variables
    pub edge_constraints: Vec<EdgeConstraint>,
    /// Already compiled?
    pub(crate) compiled: bool,
}

impl Pattern {
    pub fn new() -> Self {
        Self {
            n_vars: 0,
            var_names: HashMap::new(),
            in_edges: Vec::new(),
            out_edges: Vec::new(),
            vars: Vec::new(),
            edge_constraints: Vec::new(),
            compiled: false,
        }
    }

    /// Add a pattern variable
    pub fn add_var(&mut self, var: PatternVar) {
        self.vars.push(var);
    }

    /// Add an edge constraint between variables
    pub fn add_edge_constraint(&mut self, edge_constraint: EdgeConstraint) {
        self.edge_constraints.push(edge_constraint);
    }

    pub fn compile_pattern(&mut self) {
        assert!(!self.compiled, "Can't compile pattern more than once!");

        // Compile variables - build var_names mapping and initialize edge/required vectors
        self.n_vars = self.vars.len();
        for (var_id, var) in self.vars.iter().enumerate() {
            let var_name = &var.var_name;
            if !self.var_names.contains_key(var_name) {
                self.var_names.insert(var_name.clone(), var_id);
                self.out_edges.push(Vec::new());
                self.in_edges.push(Vec::new());
            }
            // TODO: check for duplicate variables
        }

        // Compile edge constraints
        for (edge_index, edge_constraint) in self.edge_constraints.iter().enumerate() {
            let from_var_id = self.var_names.get(&edge_constraint.from).unwrap();
            let to_var_id = self.var_names.get(&edge_constraint.to).unwrap();
            self.out_edges[*from_var_id].push(edge_index);
            self.in_edges[*to_var_id].push(edge_index);
            if let Some(label) = &edge_constraint.label {
                // add deprel constraint to destination variable
                let deprel_constraint = Constraint::DepRel(label.clone());
                let dest_var = &mut self.vars[*to_var_id];

                if matches!(dest_var.constraints, Constraint::Any) {
                    dest_var.constraints = deprel_constraint;
                } else if let Constraint::And(ref mut conjuncts) = dest_var.constraints {
                    conjuncts.push(deprel_constraint);
                } else {
                    let old = std::mem::replace(&mut dest_var.constraints, Constraint::Any);
                    dest_var.constraints = Constraint::And(vec![old, deprel_constraint]);
                }
            }
        }

        self.compiled = true;
    }

    /// Get the VarId of a variable by its name
    pub fn var_id(&self, var_name: &str) -> Option<VarId> {
        self.vars.iter().position(|v| v.var_name == var_name)
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

        let verb = PatternVar::new("verb", Constraint::POS("VERB".to_string()));
        let noun = PatternVar::new("noun", Constraint::POS("NOUN".to_string()));

        pattern.add_var(verb);
        pattern.add_var(noun);

        pattern.add_edge_constraint(EdgeConstraint {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Child,
            label: Some("nsubj".to_string()),
        });

        pattern.compile_pattern();

        assert_eq!(pattern.var_names.len(), 2);
        assert_eq!(pattern.vars.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1);
        // TODO: add more assertions
    }
}
