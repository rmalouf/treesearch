//! Pattern representation and compilation
//!
//! This module defines the AST for dependency tree patterns used
//! in the CSP-based matching algorithm.

use regex::Regex;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::Debug;

/// Type alias for pattern variable identifiers (indices into Pattern.vars)
pub type VarId = usize;

/// Value in a constraint: either a literal string or a regex pattern
#[derive(Clone)]
pub enum ConstraintValue {
    Literal(String),
    Regex(String, Regex), // Pattern string + compiled regex
}

// Manual Debug implementation
impl Debug for ConstraintValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstraintValue::Literal(s) => f.debug_tuple("Literal").field(s).finish(),
            ConstraintValue::Regex(pattern, _) => f.debug_tuple("Regex").field(pattern).finish(),
        }
    }
}

// Manual PartialEq implementation (compare pattern strings, not compiled regex)
impl PartialEq for ConstraintValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ConstraintValue::Literal(a), ConstraintValue::Literal(b)) => a == b,
            (ConstraintValue::Regex(a, _), ConstraintValue::Regex(b, _)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    Any,
    Lemma(ConstraintValue),
    UPOS(ConstraintValue),
    XPOS(ConstraintValue),
    Form(ConstraintValue),
    DepRel(ConstraintValue),
    Feature(String, ConstraintValue),
    Misc(String, ConstraintValue),
    And(Vec<Constraint>),
    Not(Box<Constraint>),
    IsChild(Option<String>),
    HasChild(Option<String>),
}

pub fn merge_constraints(a: &Constraint, b: &Constraint) -> Constraint {
    match (&a, &b) {
        (&x, &Constraint::Any) | (&Constraint::Any, &x) => x.clone(),
        (&Constraint::And(x_list), &Constraint::And(y_list)) => Constraint::And(
            x_list
                .iter()
                .cloned()
                .chain(y_list.iter().cloned())
                .collect(),
        ),
        (&Constraint::And(x_list), &y) | (&y, &Constraint::And(x_list)) => {
            let y_list = std::iter::once(y.clone());
            Constraint::And(x_list.iter().cloned().chain(y_list).collect())
        }
        (&x, &y) => Constraint::And(vec![x.clone(), y.clone()]),
    }
}

/// A pattern variable representing a node in the dependency tree
#[derive(Debug, Clone)]
pub struct PatternVar {
    pub var_name: String,
    pub constraint: Constraint,
}

impl PatternVar {
    pub fn new(var_name: &str, constr: Constraint) -> Self {
        Self {
            var_name: var_name.to_string(),
            constraint: constr,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelationType {
    Child,
    Precedes,
    ImmediatelyPrecedes,
}

#[derive(Debug, Clone)]
pub struct EdgeConstraint {
    pub from: String,
    pub to: String,
    pub relation: RelationType,
    pub label: Option<String>,
    pub negated: bool,
}

#[derive(Debug, Clone)]
pub enum DirectedEdge {
    In(usize),
    Out(usize),
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub match_pattern: BasePattern,
    pub except_patterns: Vec<BasePattern>,
    pub optional_patterns: Vec<BasePattern>,
}

/// A complete pattern to match against dependency trees
#[derive(Debug, Clone)]
pub struct BasePattern {
    pub n_vars: usize,
    pub var_ids: HashMap<String, VarId>,
    pub var_names: Vec<String>,
    pub out_edges: Vec<Vec<usize>>,
    pub in_edges: Vec<Vec<usize>>,
    pub incident_edges: Vec<Vec<DirectedEdge>>,
    pub var_constraints: Vec<Constraint>,
    pub edge_constraints: Vec<EdgeConstraint>,
}

impl BasePattern {
    pub fn new() -> Self {
        Self {
            n_vars: 0,
            var_ids: HashMap::new(),
            var_names: Vec::new(),
            in_edges: Vec::new(),
            out_edges: Vec::new(),
            incident_edges: Vec::new(),
            var_constraints: Vec::new(),
            edge_constraints: Vec::new(),
        }
    }

    pub fn with_constraints(
        vars: HashMap<String, PatternVar>,
        edges: Vec<EdgeConstraint>,
    ) -> BasePattern {
        let mut pattern = BasePattern::new();

        for var in vars.into_values() {
            pattern.add_var(&var.var_name, var.constraint);
        }

        for edge_constraint in edges.into_iter() {
            pattern.add_edge_constraint(edge_constraint);
        }

        pattern.n_vars = pattern.var_constraints.len();
        pattern
    }

    pub fn add_var(&mut self, var_name: &str, constr: Constraint) {
        match self.var_ids.entry(var_name.to_owned()) {
            Entry::Occupied(e) => {
                let id = *e.get();
                self.var_constraints[id] = merge_constraints(&self.var_constraints[id], &constr);
            }
            Entry::Vacant(e) => {
                let var_id = self.var_constraints.len();
                e.insert(var_id);
                self.var_names.push(var_name.to_string());
                self.var_constraints.push(constr);
                self.out_edges.push(Vec::new());
                self.in_edges.push(Vec::new());
                self.incident_edges.push(Vec::new()); // TODO: replace in_edges, out_edges someday
            }
        }
    }

    /// Add an edge constraint between variables
    pub fn add_edge_constraint(&mut self, edge_constraint: EdgeConstraint) {
        let from_is_anon = edge_constraint.from == "_";
        let to_is_anon = edge_constraint.to == "_";

        match (from_is_anon, to_is_anon) {
            (true, true) => {
                // _ -> _: trivially satisfied, ignore
            }
            (true, false) => {
                // _ -[rel]-> X: X has incoming edge
                // _ !-[rel]-> X: X does NOT have incoming edge
                let constraint = Constraint::IsChild(edge_constraint.label);
                let final_constraint = if edge_constraint.negated {
                    Constraint::Not(Box::new(constraint))
                } else {
                    constraint
                };
                self.add_var(&edge_constraint.to, final_constraint);
            }
            (false, true) => {
                // X -[rel]-> _: X has outgoing edge
                // X !-[rel]-> _: X does NOT have outgoing edge
                let constraint = Constraint::HasChild(edge_constraint.label);
                let final_constraint = if edge_constraint.negated {
                    Constraint::Not(Box::new(constraint))
                } else {
                    constraint
                };
                self.add_var(&edge_constraint.from, final_constraint);
            }
            (false, false) => {
                // Normal edge between two named variables
                // For positive labeled edges, add DepRel constraint to target
                // For negative labeled edges, skip DepRel (Y's deprel is unconstrained)
                self.add_var(&edge_constraint.from, Constraint::Any);
                if let Some(label) = &edge_constraint.label {
                    if !edge_constraint.negated {
                        self.add_var(
                            &edge_constraint.to,
                            Constraint::DepRel(ConstraintValue::Literal(label.clone())),
                        );
                    } else {
                        self.add_var(&edge_constraint.to, Constraint::Any);
                    }
                } else {
                    self.add_var(&edge_constraint.to, Constraint::Any);
                }

                let edge_id = self.edge_constraints.len();
                let from_var_id = self.var_ids.get(&edge_constraint.from).unwrap();
                let to_var_id = self.var_ids.get(&edge_constraint.to).unwrap();

                self.out_edges[*from_var_id].push(edge_id);
                self.in_edges[*to_var_id].push(edge_id);
                self.incident_edges[*from_var_id].push(DirectedEdge::Out(edge_id));
                self.incident_edges[*to_var_id].push(DirectedEdge::In(edge_id));
                self.edge_constraints.push(edge_constraint);
            }
        }
    }
}

impl Default for BasePattern {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let mut vars = HashMap::new();
        vars.insert(
            "verb".to_string(),
            PatternVar::new(
                "verb",
                Constraint::UPOS(ConstraintValue::Literal("VERB".to_string())),
            ),
        );
        vars.insert(
            "noun".to_string(),
            PatternVar::new(
                "noun",
                Constraint::UPOS(ConstraintValue::Literal("NOUN".to_string())),
            ),
        );

        let edges = vec![EdgeConstraint {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Child,
            label: Some("nsubj".to_string()),
            negated: false,
        }];

        let pattern = BasePattern::with_constraints(vars, edges);

        assert_eq!(pattern.var_names.len(), 2);
        assert_eq!(pattern.var_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1);
        // TODO: add more assertions
    }
}
