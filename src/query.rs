//! Query language parser
//!
//! Parses query strings into Pattern AST using pest grammar.

use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use thiserror::Error;

use crate::pattern::{
    Constraint, EdgeConstraint, Pattern, PatternVar, RelationType, compile_pattern,
};

#[derive(Parser)]
#[grammar = "query_grammar.pest"]
struct QueryParser;

/// Error type for query parsing failures
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Query error: {0}")]
    ParseError(#[from] pest::error::Error<Rule>),

    #[error("Query error: Unknown constraint key: {0}")]
    UnknownConstraintKey(String),

    #[error("Query error: Duplicate variable: {0}")]
    DuplicateVariable(String),
}

/// Parse a query string into a Pattern
pub fn parse_query(input: &str) -> Result<Pattern, QueryError> {
    let mut pairs = QueryParser::parse(Rule::query, input)?;
    let mut vars: HashMap<String, PatternVar> = HashMap::new();
    let mut edges: Vec<EdgeConstraint> = Vec::new();

    // Process all statements in the query
    let query_pair = pairs.next().unwrap();
    for statement in query_pair.into_inner() {
        match statement.as_rule() {
            Rule::statement => {
                // statement contains either node_decl or edge_decl
                let inner = statement.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::node_decl => {
                        let var = parse_var_decl(inner)?;
                        if vars.contains_key(&var.var_name) {
                            return Err(QueryError::DuplicateVariable(var.var_name));
                        };
                        vars.insert(var.var_name.to_string(), var);
                        //                        pattern.add_var(var);
                    }
                    Rule::edge_decl => {
                        let edge_constraint = parse_edge_decl(inner)?;
                        edges.push(edge_constraint);
                        //                        pattern.add_edge_constraint(edge_constraint);
                    }
                    _ => {
                        panic!("Unexpected statement type")
                    }
                }
            }
            Rule::EOI => {} // End of input
            _ => {}
        }
    }

    Ok(compile_pattern(vars, edges))
}

/// Parse a variable declaration: Name [constraint, constraint];
fn parse_var_decl(pair: pest::iterators::Pair<Rule>) -> Result<PatternVar, QueryError> {
    let mut inner = pair.into_inner();

    let ident_pair = inner.next().unwrap();
    let var_name = ident_pair.as_str().to_string();

    let constraint_list = inner.next().unwrap();
    let constraints = parse_constraint_list(constraint_list)?;

    Ok(PatternVar::new(&var_name, constraints))
}

/// Parse constraint list: may be empty or comma-separated constraints
fn parse_constraint_list(pair: pest::iterators::Pair<Rule>) -> Result<Constraint, QueryError> {
    let constraints: Vec<Constraint> = pair
        .into_inner()
        .map(parse_constraint)
        .collect::<Result<Vec<_>, _>>()?;

    match constraints.len() {
        0 => Ok(Constraint::Any),
        1 => Ok(constraints.into_iter().next().unwrap()),
        _ => Ok(Constraint::And(constraints)),
    }
}

/// Parse a single constraint: key="value"
fn parse_constraint(pair: pest::iterators::Pair<Rule>) -> Result<Constraint, QueryError> {
    let mut inner = pair.into_inner();

    let key = inner.next().unwrap().as_str();
    let value = inner.next().unwrap().into_inner().as_str().to_string();

    match key {
        "lemma" => Ok(Constraint::Lemma(value)),
        "pos" => Ok(Constraint::POS(value)),
        "form" => Ok(Constraint::Form(value)),
        "deprel" => Ok(Constraint::DepRel(value)),
        _ => Err(QueryError::UnknownConstraintKey(key.to_string())),
    }
}

/// Parse edge declaration: Source -[label]-> Target; or Source -> Target;
fn parse_edge_decl(pair: pest::iterators::Pair<Rule>) -> Result<EdgeConstraint, QueryError> {
    let mut inner = pair.into_inner();

    let from = inner.next().unwrap().as_str().to_string();

    // The next element could be edge_label (if present) or the target variable
    let next = inner.next().unwrap();

    let (label, to) = if next.as_rule() == Rule::edge_label {
        // We have a label, so get the target variable next
        let label_str = next.as_str().to_string();
        let to_var = inner.next().unwrap().as_str().to_string();
        (Some(label_str), to_var)
    } else {
        // No label, this is the target variable
        (None, next.as_str().to_string())
    };

    // For now, all edge constraints use Child relation (parent -> child)
    // We can extend this later to support different arrow types for other relations
    Ok(EdgeConstraint {
        from,
        to,
        relation: RelationType::Child,
        label,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_constraints() {
        let query = "Node [];";
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.var_ids.get("Node").unwrap(), 0);
        assert!(pattern.var_constraints[0].is_any());

        let query = r#"Verb [pos="VERB"];"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.var_ids.get("Verb").unwrap(), 0);
        assert_eq!(
            pattern.var_constraints[0],
            Constraint::POS("VERB".to_string())
        );

        let query = r#"Help [lemma="help", pos="VERB"];"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.var_ids.get("Help").unwrap(), 0);
        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert_eq!(constraints[0], Constraint::Lemma("help".to_string()));
                assert_eq!(constraints[1], Constraint::POS("VERB".to_string()));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_edge() {
        let query = r#"
            Help [lemma="help"];
            To [lemma="to"];
            Help -[xcomp]-> To;
        "#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.edge_constraints[0];
        assert_eq!(edge_constraint.from, "Help");
        assert_eq!(edge_constraint.to, "To");
        assert_eq!(edge_constraint.relation, RelationType::Child);
        assert_eq!(edge_constraint.label, Some("xcomp".to_string()));
    }

    #[test]
    fn test_parse_unconstrained_edge() {
        let query = r#"
            Parent [];
            Child [];
            Parent -> Child;
        "#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.edge_constraints[0];
        assert_eq!(edge_constraint.from, "Parent");
        assert_eq!(edge_constraint.to, "Child");
        assert_eq!(edge_constraint.relation, RelationType::Child);
        assert_eq!(edge_constraint.label, None);
    }

    #[test]
    fn test_parse_complex_query() {
        let query = r#"
            // Find help-to-verb constructions
            Help [lemma="help"];
            To [lemma="to"];
            YHead [];

            Help -[xcomp]-> To;
            To -[obj]-> YHead;
        "#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 3);
        assert!(pattern.var_ids.contains_key("Help"));
        assert!(pattern.var_ids.contains_key("To"));
        assert!(pattern.var_ids.contains_key("YHead"));

        assert_eq!(pattern.edge_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints[0].from, "Help");
        assert_eq!(pattern.edge_constraints[0].to, "To");
        assert_eq!(pattern.edge_constraints[1].from, "To");
        assert_eq!(pattern.edge_constraints[1].to, "YHead");
    }

    #[test]
    fn test_parse_all_constraint_types() {
        let query = r#"
            N1 [lemma="run"];
            N2 [pos="VERB"];
            N3 [form="running"];
            N4 [deprel="nsubj"];
        "#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 4);
        assert!(
            pattern
                .var_constraints
                .contains(&Constraint::Lemma("run".to_string()))
        );
        assert!(
            pattern
                .var_constraints
                .contains(&Constraint::POS("VERB".to_string()))
        );
        assert!(
            pattern
                .var_constraints
                .contains(&Constraint::Form("running".to_string()))
        );
        assert!(
            pattern
                .var_constraints
                .contains(&Constraint::DepRel("nsubj".to_string()))
        );
    }

    #[test]
    fn test_forward_reference_in_edge() {
        // Edge constraint references a variable defined later in the query
        let query = r#"
            Help [lemma="help"];
            Help -> To;
            To [lemma="to"];
        "#;
        let pattern = parse_query(query).unwrap();

        // Parser accepts this, but should validate that all variables exist
        assert_eq!(pattern.var_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1);
        assert_eq!(pattern.edge_constraints[0].from, "Help");
        assert_eq!(pattern.edge_constraints[0].to, "To");
    }

    #[test]
    fn test_both_vars_undefined_in_edge() {
        // Edge constraint where both variables are undefined
        let query = r#"
            Node [pos="NOUN"];
            Foo -> Bar;
        "#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 3);
        assert_eq!(pattern.edge_constraints.len(), 1);
        assert_eq!(pattern.edge_constraints[0].from, "Foo");
        assert_eq!(pattern.edge_constraints[0].to, "Bar");
    }

    #[test]
    fn test_self_reference_in_edge() {
        // Edge constraint where a variable references itself
        let query = r#"
            Node [pos="NOUN"];
            Node -> Node;
        "#;
        let pattern = parse_query(query).unwrap();

        // This is likely invalid but parser should accept it
        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(pattern.edge_constraints.len(), 1);
        assert_eq!(pattern.edge_constraints[0].from, "Node");
        assert_eq!(pattern.edge_constraints[0].to, "Node");
    }

    #[test]
    fn test_duplicate_variable_definition() {
        // Same variable with conflicting constraints
        let query = r#"
            Node [pos="NOUN"];
            Node [pos="VERB"];
            Node -> Node;
        "#;
        let pattern = parse_query(query);
        assert!(matches!(pattern, Err(QueryError::DuplicateVariable(_))));
    }
}
