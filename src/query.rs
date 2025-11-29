//! Query language parser
//!
//! Parses query strings into Pattern AST using pest grammar.

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::collections::HashMap;
use thiserror::Error;

use crate::pattern::{Constraint, EdgeConstraint, Pattern, PatternVar, RelationType};

#[derive(Parser)]
#[grammar = "query_grammar.pest"]
struct QueryParser;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Query error: {0}")]
    ParseError(#[from] pest::error::Error<Rule>),

    #[error("Query error: Unknown constraint key: {0}")]
    UnknownConstraintKey(String),

    #[error("Query error: Duplicate variable: {0}")]
    DuplicateVariable(String),

    #[error("Query error: No MATCH block found")]
    NoMATCH,
}

pub fn parse_query(input: &str) -> Result<Pattern, QueryError> {
    let mut match_pattern: Option<Pattern> = None;

    let mut pairs = QueryParser::parse(Rule::query, input)?;
    let query_pair = pairs.next().unwrap();

    for item in query_pair.into_inner() {
        match item.as_rule() {
            Rule::match_block => match_pattern = Some(parse_query_block(item)?),
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    if let Some(match_pattern) = match_pattern {
        Ok(match_pattern)
    } else {
        Err(QueryError::NoMATCH)
    }
}

pub fn parse_query_block(item: Pair<Rule>) -> Result<Pattern, QueryError> {
    let mut vars: HashMap<String, PatternVar> = HashMap::new();
    let mut edges: Vec<EdgeConstraint> = Vec::new();

    for statement in item.into_inner() {
        match statement.as_rule() {
            Rule::statement => {
                let inner = statement.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::node_decl => {
                        let var = parse_var_decl(inner)?;
                        if vars.contains_key(&var.var_name) {
                            return Err(QueryError::DuplicateVariable(var.var_name));
                        };
                        vars.insert(var.var_name.to_string(), var);
                    }
                    Rule::edge_decl => {
                        let edge_constraint = parse_edge_decl(inner)?;
                        edges.push(edge_constraint);
                    }
                    Rule::precedence_decl => {
                        let edge_constraint = parse_precedence_decl(inner)?;
                        edges.push(edge_constraint);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
    }

    Ok(Pattern::with_constraints(vars, edges))
}

fn parse_var_decl(pair: pest::iterators::Pair<Rule>) -> Result<PatternVar, QueryError> {
    let mut inner = pair.into_inner();

    let ident_pair = inner.next().unwrap();
    let var_name = ident_pair.as_str().to_string();
    let constraint_list = inner.next().unwrap();
    let constraints = parse_constraint_list(constraint_list)?;

    Ok(PatternVar::new(&var_name, constraints))
}

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

fn parse_constraint(pair: pest::iterators::Pair<Rule>) -> Result<Constraint, QueryError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::feature_constraint => parse_feature_constraint(inner),
        Rule::regular_constraint => parse_regular_constraint(inner),
        _ => panic!("Unexpected constraint type: {:?}", inner.as_rule()),
    }
}

fn parse_feature_constraint(pair: pest::iterators::Pair<Rule>) -> Result<Constraint, QueryError> {
    let mut inner = pair.into_inner();
    let feature_key = inner.next().unwrap().as_str().to_string();
    let operator = inner.next().unwrap().as_str();
    let value = inner.next().unwrap().into_inner().as_str().to_string();

    let constraint = Constraint::Feature(feature_key, value);

    if operator == "!=" {
        Ok(Constraint::Not(Box::new(constraint)))
    } else {
        Ok(constraint)
    }
}

fn parse_regular_constraint(pair: pest::iterators::Pair<Rule>) -> Result<Constraint, QueryError> {
    let mut inner = pair.into_inner();

    let key = inner.next().unwrap().as_str();
    let operator = inner.next().unwrap().as_str();
    let value = inner.next().unwrap().into_inner().as_str().to_string();

    let constraint = match key {
        "lemma" => Constraint::Lemma(value),
        "upos" => Constraint::UPOS(value),
        "xpos" => Constraint::XPOS(value),
        "form" => Constraint::Form(value),
        "deprel" => Constraint::DepRel(value),
        _ => return Err(QueryError::UnknownConstraintKey(key.to_string())),
    };

    if operator == "!=" {
        Ok(Constraint::Not(Box::new(constraint)))
    } else {
        Ok(constraint)
    }
}

fn parse_edge_decl(pair: pest::iterators::Pair<Rule>) -> Result<EdgeConstraint, QueryError> {
    let mut inner = pair.into_inner();

    let from = inner.next().unwrap().as_str().to_string();

    // Next element is the edge_op (which contains the actual operator rule)
    let edge_op = inner.next().unwrap();
    let mut op_inner = edge_op.into_inner();
    let actual_op = op_inner.next().unwrap(); // Get the actual operator (labeled_edge, etc.)
    let op_rule = actual_op.as_rule();

    let negated = matches!(op_rule, Rule::neg_labeled_edge | Rule::neg_unlabeled_edge);

    // Check if there's a label inside the actual operator
    let label = if matches!(op_rule, Rule::neg_labeled_edge | Rule::labeled_edge) {
        // Extract the edge_label from within the labeled edge operator
        actual_op
            .into_inner()
            .next()
            .map(|p| p.as_str().to_string())
    } else {
        None
    };

    let to = inner.next().unwrap().as_str().to_string();

    Ok(EdgeConstraint {
        from,
        to,
        relation: RelationType::Child,
        label,
        negated,
    })
}

fn parse_precedence_decl(pair: pest::iterators::Pair<Rule>) -> Result<EdgeConstraint, QueryError> {
    let mut inner = pair.into_inner();

    let from = inner.next().unwrap().as_str().to_string();

    // The operator is a precedence_op rule
    let op_pair = inner.next().unwrap();
    let operator = op_pair.as_str();

    let to = inner.next().unwrap().as_str().to_string();

    let relation = match operator {
        "<<" => RelationType::Precedes,
        "<" => RelationType::ImmediatelyPrecedes,
        _ => panic!("Unexpected precedence operator: {}", operator),
    };

    Ok(EdgeConstraint {
        from,
        to,
        relation,
        label: None,
        negated: false, // Negation not supported for precedence
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_constraints() {
        let query = "MATCH { Node []; }";
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.var_ids.get("Node").unwrap(), 0);
        assert!(pattern.var_constraints[0].is_any());

        let query = r#"MATCH { Verb [upos="VERB"]; }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.var_ids.get("Verb").unwrap(), 0);
        assert_eq!(
            pattern.var_constraints[0],
            Constraint::UPOS("VERB".to_string())
        );

        let query = r#"MATCH { Help [lemma="help", upos="VERB"]; }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.var_ids.get("Help").unwrap(), 0);
        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert_eq!(constraints[0], Constraint::Lemma("help".to_string()));
                assert_eq!(constraints[1], Constraint::UPOS("VERB".to_string()));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_edge() {
        let query = r#"MATCH {
            Help [lemma="help"];
            To [lemma="to"];
            Help -[xcomp]-> To;
        }"#;
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
        let query = r#"MATCH {
            Parent [];
            Child [];
            Parent -> Child;
        }"#;
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
    fn test_parse_negative_unlabeled_edge() {
        let query = r#"MATCH {
            Help [];
            To [];
            Help !-> To;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.edge_constraints[0];
        assert_eq!(edge_constraint.from, "Help");
        assert_eq!(edge_constraint.to, "To");
        assert_eq!(edge_constraint.relation, RelationType::Child);
        assert_eq!(edge_constraint.label, None);
        assert_eq!(edge_constraint.negated, true);
    }

    #[test]
    fn test_parse_negative_labeled_edge() {
        let query = r#"MATCH {
            Help [lemma="help"];
            To [lemma="to"];
            Help !-[xcomp]-> To;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.edge_constraints[0];
        assert_eq!(edge_constraint.from, "Help");
        assert_eq!(edge_constraint.to, "To");
        assert_eq!(edge_constraint.relation, RelationType::Child);
        assert_eq!(edge_constraint.label, Some("xcomp".to_string()));
        assert_eq!(edge_constraint.negated, true);
    }

    #[test]
    fn test_parse_positive_edge_not_negated() {
        // Verify positive edges have negated=false
        let query = r#"MATCH {
            Help [];
            To [];
            Help -[xcomp]-> To;
        }"#;
        let pattern = parse_query(query).unwrap();

        let edge_constraint = &pattern.edge_constraints[0];
        assert_eq!(edge_constraint.negated, false);
    }

    #[test]
    fn test_parse_complex_query() {
        let query = r#"MATCH {
            // Find help-to-verb constructions
            Help [lemma="help"];
            To [lemma="to"];
            YHead [];

            Help -[xcomp]-> To;
            To -[obj]-> YHead;
        }"#;
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
        let query = r#"MATCH {
            N1 [lemma="run"];
            N2 [upos="VERB"];
            N3 [form="running"];
            N4 [deprel="nsubj"];
        }"#;
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
                .contains(&Constraint::UPOS("VERB".to_string()))
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
        let query = r#"MATCH {
            Help [lemma="help"];
            Help -> To;
            To [lemma="to"];
        }"#;
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
        let query = r#"MATCH {
            Node [upos="NOUN"];
            Foo -> Bar;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 3);
        assert_eq!(pattern.edge_constraints.len(), 1);
        assert_eq!(pattern.edge_constraints[0].from, "Foo");
        assert_eq!(pattern.edge_constraints[0].to, "Bar");
    }

    #[test]
    fn test_self_reference_in_edge() {
        // Edge constraint where a variable references itself
        let query = r#"MATCH {
            Node [upos="NOUN"];
            Node -> Node;
        }"#;
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
        let query = r#"MATCH {
            Node [upos="NOUN"];
            Node [upos="VERB"];
            Node -> Node;
        }"#;
        let pattern = parse_query(query);
        assert!(matches!(pattern, Err(QueryError::DuplicateVariable(_))));
    }

    #[test]
    fn test_parse_precedes() {
        // Test << (precedes) operator
        let query = r#"MATCH {
            First [upos="NOUN"];
            Second [upos="VERB"];
            First << Second;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.edge_constraints[0];
        assert_eq!(edge_constraint.from, "First");
        assert_eq!(edge_constraint.to, "Second");
        assert_eq!(edge_constraint.relation, RelationType::Precedes);
        assert_eq!(edge_constraint.label, None);
    }

    #[test]
    fn test_parse_immediately_precedes() {
        // Test < (immediately precedes) operator
        let query = r#"MATCH {
            Adj [upos="ADJ"];
            Noun [upos="NOUN"];
            Adj < Noun;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.edge_constraints[0];
        assert_eq!(edge_constraint.from, "Adj");
        assert_eq!(edge_constraint.to, "Noun");
        assert_eq!(edge_constraint.relation, RelationType::ImmediatelyPrecedes);
        assert_eq!(edge_constraint.label, None);
    }

    #[test]
    fn test_parse_mixed_precedence_and_dependency() {
        // Test query with both dependency edges and precedence constraints
        let query = r#"
MATCH {
            Verb [upos="VERB"];
            Subj [upos="NOUN"];
            Obj [upos="NOUN"];
            Verb -[nsubj]-> Subj;
            Verb -[obj]-> Obj;
            Subj << Verb;
            Verb << Obj;
        
}"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 3);
        assert_eq!(pattern.edge_constraints.len(), 4);

        // Check that we have both Child and Precedes relations
        let has_child = pattern
            .edge_constraints
            .iter()
            .any(|e| e.relation == RelationType::Child);
        let has_precedes = pattern
            .edge_constraints
            .iter()
            .any(|e| e.relation == RelationType::Precedes);

        assert!(has_child);
        assert!(has_precedes);
    }

    #[test]
    fn test_parse_precedence_chain() {
        // Test chained precedence: A < B << C
        let query = r#"
MATCH {
            A [];
            B [];
            C [];
            A < B;
            B << C;
        
}"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 3);
        assert_eq!(pattern.edge_constraints.len(), 2);

        // Find the immediate precedes constraint
        let immediate = pattern
            .edge_constraints
            .iter()
            .find(|e| e.relation == RelationType::ImmediatelyPrecedes)
            .unwrap();
        assert_eq!(immediate.from, "A");
        assert_eq!(immediate.to, "B");

        // Find the precedes constraint
        let precedes = pattern
            .edge_constraints
            .iter()
            .find(|e| e.relation == RelationType::Precedes)
            .unwrap();
        assert_eq!(precedes.from, "B");
        assert_eq!(precedes.to, "C");
    }

    #[test]
    fn test_parse_feature_constraint() {
        let query = r#"MATCH { V [feats.Tense="Past"]; }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.var_ids.get("V").unwrap(), 0);
        match &pattern.var_constraints[0] {
            Constraint::Feature(key, value) => {
                assert_eq!(key, "Tense");
                assert_eq!(value, "Past");
            }
            _ => panic!("Expected Feature constraint"),
        }
    }

    #[test]
    fn test_parse_multiple_features() {
        let query = r#"MATCH { N [feats.Number="Plur", feats.Case="Nom"]; }"#;
        let pattern = parse_query(query).unwrap();

        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Feature(k, v) if k == "Number" && v == "Plur"
                )));
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Feature(k, v) if k == "Case" && v == "Nom"
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_mixed_constraints() {
        let query = r#"MATCH { V [lemma="be", feats.Tense="Past"]; }"#;
        let pattern = parse_query(query).unwrap();

        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert!(constraints.contains(&Constraint::Lemma("be".to_string())));
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Feature(k, v) if k == "Tense" && v == "Past"
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_negative_constraint() {
        let query = r#"MATCH { V [lemma!="help"]; }"#;
        let pattern = parse_query(query).unwrap();

        match &pattern.var_constraints[0] {
            Constraint::Not(inner) => match inner.as_ref() {
                Constraint::Lemma(lemma) => assert_eq!(lemma, "help"),
                _ => panic!("Expected Lemma constraint inside Not"),
            },
            _ => panic!("Expected Not constraint"),
        }
    }

    #[test]
    fn test_parse_negative_feature() {
        let query = r#"MATCH { V [feats.Tense!="Past"]; }"#;
        let pattern = parse_query(query).unwrap();

        match &pattern.var_constraints[0] {
            Constraint::Not(inner) => match inner.as_ref() {
                Constraint::Feature(key, value) => {
                    assert_eq!(key, "Tense");
                    assert_eq!(value, "Past");
                }
                _ => panic!("Expected Feature constraint inside Not"),
            },
            _ => panic!("Expected Not constraint"),
        }
    }

    #[test]
    fn test_parse_mixed_positive_negative() {
        let query = r#"MATCH { V [lemma="run", upos!="NOUN"]; }"#;
        let pattern = parse_query(query).unwrap();

        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(constraints.contains(&Constraint::Lemma("run".to_string())));
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Not(inner) if matches!(inner.as_ref(), Constraint::UPOS(pos) if pos == "NOUN")
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_anonymous_incoming_edge() {
        // Test: _ -[obj]-> X
        let query = r#"MATCH {
            X [upos="NOUN"];
            _ -[obj]-> X;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(pattern.edge_constraints.len(), 0); // Anonymous edges don't create edge constraints
        assert_eq!(*pattern.var_ids.get("X").unwrap(), 0);

        // Check that X has HasIncomingEdge constraint
        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(constraints.contains(&Constraint::UPOS("NOUN".to_string())));
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::HasIncomingEdge(RelationType::Child, Some(label)) if label == "obj"
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_anonymous_outgoing_edge() {
        // Test: X -[nsubj]-> _
        let query = r#"MATCH {
            X [upos="VERB"];
            X -[nsubj]-> _;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);
        assert_eq!(pattern.edge_constraints.len(), 0);

        // Check that X has HasOutgoingEdge constraint
        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(constraints.contains(&Constraint::UPOS("VERB".to_string())));
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::HasOutgoingEdge(RelationType::Child, Some(label)) if label == "nsubj"
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_anonymous_both_sides() {
        // Test: _ -> _ (trivially satisfied, should be ignored)
        let query = r#"MATCH {
            _ -> _;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 0);
        assert_eq!(pattern.edge_constraints.len(), 0);
    }

    #[test]
    fn test_parse_anonymous_multiple() {
        // Test: Multiple anonymous edges on same variable
        let query = r#"MATCH {
            X [upos="NOUN"];
            _ -[obj]-> X;
            _ -[nsubj]-> X;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);

        // Check that X has both HasIncomingEdge constraints
        match &pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 3); // UPOS + 2 HasIncomingEdge
                assert!(constraints.contains(&Constraint::UPOS("NOUN".to_string())));
                assert!(
                    constraints
                        .iter()
                        .filter(|c| matches!(
                            c,
                            Constraint::HasIncomingEdge(RelationType::Child, _)
                        ))
                        .count()
                        == 2
                );
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_anonymous_no_label() {
        // Test: _ -> X (no label specified)
        let query = r#"MATCH {
            X [];
            _ -> X;
        }"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 1);

        // Check that X has HasIncomingEdge with no label
        assert!(matches!(
            &pattern.var_constraints[0],
            Constraint::HasIncomingEdge(RelationType::Child, None)
        ));
    }

    #[test]
    fn test_parse_mixed_anonymous_and_normal() {
        // Test: Mix of anonymous and normal edges
        let query = r#"
MATCH {
            X [upos="VERB"];
            Y [upos="NOUN"];
            _ -[obj]-> X;
            X -[nsubj]-> Y;
        
}"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.var_constraints.len(), 2);
        assert_eq!(pattern.edge_constraints.len(), 1); // Only X -> Y creates edge constraint

        // X should have HasIncomingEdge constraint
        let x_constraints = &pattern.var_constraints[*pattern.var_ids.get("X").unwrap()];
        match x_constraints {
            Constraint::And(constraints) => {
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::HasIncomingEdge(RelationType::Child, Some(label)) if label == "obj"
                )));
            }
            _ => panic!("Expected And constraint for X"),
        }
    }
}
