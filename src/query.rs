//! Query language parser
//!
//! Parses query strings into Pattern AST using pest grammar.

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::collections::HashMap;
use thiserror::Error;

use crate::pattern::{
    BasePattern, Constraint, ConstraintValue, EdgeConstraint, Pattern, PatternVar, RelationType,
};
use regex::Regex;

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

    #[error("Query error: Variable '{0}' already defined in another EXCEPT/OPTIONAL block")]
    DuplicateExtensionVariable(String),

    #[error("Query error: Invalid regex pattern '{0}': {1}")]
    InvalidRegex(String, String),
}

pub fn compile_query(input: &str) -> Result<Pattern, QueryError> {
    let mut match_pattern: Option<BasePattern> = None;
    let mut except_patterns: Vec<BasePattern> = vec![];
    let mut optional_patterns: Vec<BasePattern> = vec![];

    let mut pairs = QueryParser::parse(Rule::query, input)?;
    let query_pair = pairs.next().unwrap();

    for item in query_pair.into_inner() {
        match item.as_rule() {
            Rule::match_block => match_pattern = Some(compile_query_block(item)?),
            Rule::except_block => except_patterns.push(compile_query_block(item)?),
            Rule::optional_block => optional_patterns.push(compile_query_block(item)?),
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    if let Some(match_pattern) = match_pattern {
        // Validate that new variables in extension blocks are unique
        validate_unique_extension_variables(&match_pattern, &except_patterns, &optional_patterns)?;
        Ok(Pattern {
            match_pattern,
            except_patterns,
            optional_patterns,
        })
    } else {
        Err(QueryError::NoMATCH)
    }
}

pub fn compile_query_block(item: Pair<Rule>) -> Result<BasePattern, QueryError> {
    let mut vars: HashMap<String, PatternVar> = HashMap::new();
    let mut edges: Vec<EdgeConstraint> = Vec::new();

    for statement in item.into_inner() {
        match statement.as_rule() {
            Rule::statement => {
                let inner = statement.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::node_decl => {
                        let var = compile_var_decl(inner)?;
                        if vars.contains_key(&var.var_name) {
                            return Err(QueryError::DuplicateVariable(var.var_name));
                        };
                        vars.insert(var.var_name.to_string(), var);
                    }
                    Rule::edge_decl => {
                        let edge_constraint = compile_edge_decl(inner)?;
                        edges.push(edge_constraint);
                    }
                    Rule::precedence_decl => {
                        let edge_constraint = compile_precedence_constraint(inner)?;
                        edges.push(edge_constraint);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
    }

    Ok(BasePattern::with_constraints(vars, edges))
}

/// Validate that new variables in EXCEPT/OPTIONAL blocks are unique across all extension blocks
fn validate_unique_extension_variables(
    match_pattern: &BasePattern,
    except_patterns: &[BasePattern],
    optional_patterns: &[BasePattern],
) -> Result<(), QueryError> {
    use std::collections::HashSet;

    let match_vars: HashSet<&String> = match_pattern.var_names.iter().collect();
    let mut seen_new_vars: HashSet<&String> = HashSet::new();

    for pattern in except_patterns.iter().chain(optional_patterns.iter()) {
        for var_name in &pattern.var_names {
            if !match_vars.contains(var_name) && !seen_new_vars.insert(var_name) {
                return Err(QueryError::DuplicateExtensionVariable(var_name.clone()));
            }
        }
    }
    Ok(())
}

fn compile_var_decl(pair: Pair<Rule>) -> Result<PatternVar, QueryError> {
    let mut inner = pair.into_inner();

    let ident_pair = inner.next().unwrap();
    let var_name = ident_pair.as_str().to_string();
    let constraint_list = inner.next().unwrap();
    let constraints = compile_constraint_list(constraint_list)?;

    Ok(PatternVar::new(&var_name, constraints))
}

fn compile_constraint_list(pair: Pair<Rule>) -> Result<Constraint, QueryError> {
    let constraints: Vec<Constraint> = pair
        .into_inner()
        .map(compile_constraint)
        .collect::<Result<Vec<_>, _>>()?;

    match constraints.len() {
        0 => Ok(Constraint::Any),
        1 => Ok(constraints.into_iter().next().unwrap()),
        _ => Ok(Constraint::And(constraints)),
    }
}

fn compile_constraint(pair: Pair<Rule>) -> Result<Constraint, QueryError> {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::feature_constraint => compile_feature_constraint(inner, Constraint::Feature),
        Rule::misc_constraint => compile_feature_constraint(inner, Constraint::Misc),
        Rule::regular_constraint => compile_regular_constraint(inner),
        _ => unreachable!(),
    }
}

fn compile_feature_constraint<F>(
    pair: Pair<Rule>,
    make_constraint: F,
) -> Result<Constraint, QueryError>
where
    F: FnOnce(String, ConstraintValue) -> Constraint,
{
    let mut inner = pair.into_inner();
    let feature_key = inner.next().unwrap().as_str().to_string();
    let operator = inner.next().unwrap().as_str();
    let value_pair = inner.next().unwrap(); // constraint_value
    let value = parse_constraint_value(value_pair)?;

    let constraint = make_constraint(feature_key, value);

    if operator == "!=" {
        Ok(Constraint::Not(Box::new(constraint)))
    } else {
        Ok(constraint)
    }
}

fn parse_constraint_value(pair: Pair<Rule>) -> Result<ConstraintValue, QueryError> {
    // pair is a constraint_value, which contains either string_literal or regex_literal
    let inner = pair.into_inner().next().unwrap();
    let rule = inner.as_rule();
    let value_str = inner.into_inner().as_str().to_string();

    match rule {
        Rule::string_literal => Ok(ConstraintValue::Literal(value_str)),
        Rule::regex_literal => {
            let anchored_pattern = format!("^{}$", value_str);
            match Regex::new(&anchored_pattern) {
                Ok(regex) => Ok(ConstraintValue::Regex(value_str, regex)),
                Err(e) => Err(QueryError::InvalidRegex(value_str, e.to_string())),
            }
        }
        _ => unreachable!(),
    }
}

fn compile_regular_constraint(pair: Pair<Rule>) -> Result<Constraint, QueryError> {
    let mut inner = pair.into_inner();

    let key = inner.next().unwrap().as_str();
    let operator = inner.next().unwrap().as_str();
    let value_pair = inner.next().unwrap(); // constraint_value
    let value = parse_constraint_value(value_pair)?;

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

fn compile_edge_decl(pair: Pair<Rule>) -> Result<EdgeConstraint, QueryError> {
    let mut inner = pair.into_inner();

    let from = inner.next().unwrap().as_str().to_string();

    // Next element is the edge_op (which contains the actual operator rule)
    let edge_op = inner.next().unwrap();
    let mut op_inner = edge_op.into_inner();
    let actual_op = op_inner.next().unwrap(); // Get the actual operator (labeled_edge, etc.)
    let op_rule = actual_op.as_rule();

    let negated = matches!(op_rule, Rule::neg_labeled_edge | Rule::neg_unlabeled_edge);

    let label = if matches!(op_rule, Rule::neg_labeled_edge | Rule::labeled_edge) {
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

fn compile_precedence_constraint(pair: Pair<Rule>) -> Result<EdgeConstraint, QueryError> {
    let mut inner = pair.into_inner();

    let from = inner.next().unwrap().as_str().to_string();
    let operator = inner.next().unwrap().as_str();
    let to = inner.next().unwrap().as_str().to_string();

    let relation = match operator {
        "<<" => RelationType::Precedes,
        "<" => RelationType::ImmediatelyPrecedes,
        _ => unreachable!(),
    };

    Ok(EdgeConstraint {
        from,
        to,
        relation,
        label: None,
        negated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_constraints() {
        let query = "MATCH { Node []; }";
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.match_pattern.var_ids.get("Node").unwrap(), 0);
        assert!(matches!(
            pattern.match_pattern.var_constraints[0],
            Constraint::Any
        ));

        let query = r#"MATCH { Verb [upos="VERB"]; }"#;
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.match_pattern.var_ids.get("Verb").unwrap(), 0);
        assert_eq!(
            pattern.match_pattern.var_constraints[0],
            Constraint::UPOS(ConstraintValue::Literal("VERB".to_string()))
        );

        let query = r#"MATCH { Help [lemma="help" & upos="VERB"]; }"#;
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.match_pattern.var_ids.get("Help").unwrap(), 0);
        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert_eq!(
                    constraints[0],
                    Constraint::Lemma(ConstraintValue::Literal("help".to_string()))
                );
                assert_eq!(
                    constraints[1],
                    Constraint::UPOS(ConstraintValue::Literal("VERB".to_string()))
                );
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 2);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.match_pattern.edge_constraints[0];
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 2);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.match_pattern.edge_constraints[0];
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.match_pattern.edge_constraints[0];
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.match_pattern.edge_constraints[0];
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
        let pattern = compile_query(query).unwrap();

        let edge_constraint = &pattern.match_pattern.edge_constraints[0];
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 3);
        assert!(pattern.match_pattern.var_ids.contains_key("Help"));
        assert!(pattern.match_pattern.var_ids.contains_key("To"));
        assert!(pattern.match_pattern.var_ids.contains_key("YHead"));

        assert_eq!(pattern.match_pattern.edge_constraints.len(), 2);
        assert_eq!(pattern.match_pattern.edge_constraints[0].from, "Help");
        assert_eq!(pattern.match_pattern.edge_constraints[0].to, "To");
        assert_eq!(pattern.match_pattern.edge_constraints[1].from, "To");
        assert_eq!(pattern.match_pattern.edge_constraints[1].to, "YHead");
    }

    #[test]
    fn test_parse_all_constraint_types() {
        let query = r#"MATCH {
            N1 [lemma="run"];
            N2 [upos="VERB"];
            N3 [form="running"];
            N4 [deprel="nsubj"];
        }"#;
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 4);
        assert!(
            pattern
                .match_pattern
                .var_constraints
                .contains(&Constraint::Lemma(ConstraintValue::Literal(
                    "run".to_string()
                )))
        );
        assert!(
            pattern
                .match_pattern
                .var_constraints
                .contains(&Constraint::UPOS(ConstraintValue::Literal(
                    "VERB".to_string()
                )))
        );
        assert!(
            pattern
                .match_pattern
                .var_constraints
                .contains(&Constraint::Form(ConstraintValue::Literal(
                    "running".to_string()
                )))
        );
        assert!(
            pattern
                .match_pattern
                .var_constraints
                .contains(&Constraint::DepRel(ConstraintValue::Literal(
                    "nsubj".to_string()
                )))
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
        let pattern = compile_query(query).unwrap();

        // Parser accepts this, but should validate that all variables exist
        assert_eq!(pattern.match_pattern.var_constraints.len(), 2);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);
        assert_eq!(pattern.match_pattern.edge_constraints[0].from, "Help");
        assert_eq!(pattern.match_pattern.edge_constraints[0].to, "To");
    }

    #[test]
    fn test_both_vars_undefined_in_edge() {
        // Edge constraint where both variables are undefined
        let query = r#"MATCH {
            Node [upos="NOUN"];
            Foo -> Bar;
        }"#;
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 3);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);
        assert_eq!(pattern.match_pattern.edge_constraints[0].from, "Foo");
        assert_eq!(pattern.match_pattern.edge_constraints[0].to, "Bar");
    }

    #[test]
    fn test_self_reference_in_edge() {
        // Edge constraint where a variable references itself
        let query = r#"MATCH {
            Node [upos="NOUN"];
            Node -> Node;
        }"#;
        let pattern = compile_query(query).unwrap();

        // This is likely invalid but parser should accept it
        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);
        assert_eq!(pattern.match_pattern.edge_constraints[0].from, "Node");
        assert_eq!(pattern.match_pattern.edge_constraints[0].to, "Node");
    }

    #[test]
    fn test_duplicate_variable_definition() {
        // Same variable with conflicting constraints
        let query = r#"MATCH {
            Node [upos="NOUN"];
            Node [upos="VERB"];
            Node -> Node;
        }"#;
        let pattern = compile_query(query);
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 2);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.match_pattern.edge_constraints[0];
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 2);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1);

        let edge_constraint = &pattern.match_pattern.edge_constraints[0];
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 3);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 4);

        // Check that we have both Child and Precedes relations
        let has_child = pattern
            .match_pattern
            .edge_constraints
            .iter()
            .any(|e| e.relation == RelationType::Child);
        let has_precedes = pattern
            .match_pattern
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 3);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 2);

        // Find the immediate precedes constraint
        let immediate = pattern
            .match_pattern
            .edge_constraints
            .iter()
            .find(|e| e.relation == RelationType::ImmediatelyPrecedes)
            .unwrap();
        assert_eq!(immediate.from, "A");
        assert_eq!(immediate.to, "B");

        // Find the precedes constraint
        let precedes = pattern
            .match_pattern
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        assert_eq!(*pattern.match_pattern.var_ids.get("V").unwrap(), 0);
        match &pattern.match_pattern.var_constraints[0] {
            Constraint::Feature(key, ConstraintValue::Literal(value)) => {
                assert_eq!(key, "Tense");
                assert_eq!(value.as_str(), "Past");
            }
            _ => panic!("Expected Feature constraint"),
        }
    }

    #[test]
    fn test_parse_multiple_features() {
        let query = r#"MATCH { N [feats.Number="Plur" & feats.Case="Nom"]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Feature(k, ConstraintValue::Literal(v)) if k == "Number" && v == "Plur"
                )));
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Feature(k, ConstraintValue::Literal(v)) if k == "Case" && v == "Nom"
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_mixed_constraints() {
        let query = r#"MATCH { V [lemma="be" & feats.Tense="Past"]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert!(
                    constraints.contains(&Constraint::Lemma(ConstraintValue::Literal(
                        "be".to_string()
                    )))
                );
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Feature(k, ConstraintValue::Literal(v)) if k == "Tense" && v == "Past"
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_negative_constraint() {
        let query = r#"MATCH { V [lemma!="help"]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::Not(inner) => match inner.as_ref() {
                Constraint::Lemma(ConstraintValue::Literal(lemma)) => assert_eq!(lemma, "help"),
                _ => panic!("Expected Lemma constraint inside Not"),
            },
            _ => panic!("Expected Not constraint"),
        }
    }

    #[test]
    fn test_parse_negative_feature() {
        let query = r#"MATCH { V [feats.Tense!="Past"]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::Not(inner) => match inner.as_ref() {
                Constraint::Feature(key, ConstraintValue::Literal(value)) => {
                    assert_eq!(key, "Tense");
                    assert_eq!(value.as_str(), "Past");
                }
                _ => panic!("Expected Feature constraint inside Not"),
            },
            _ => panic!("Expected Not constraint"),
        }
    }

    #[test]
    fn test_parse_mixed_positive_negative() {
        let query = r#"MATCH { V [lemma="run" & upos!="NOUN"]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(
                    constraints.contains(&Constraint::Lemma(ConstraintValue::Literal(
                        "run".to_string()
                    )))
                );
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::Not(inner) if matches!(inner.as_ref(), Constraint::UPOS(ConstraintValue::Literal(pos)) if pos == "NOUN")
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 0); // Anonymous edges don't create edge constraints
        assert_eq!(*pattern.match_pattern.var_ids.get("X").unwrap(), 0);

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(
                    constraints.contains(&Constraint::UPOS(ConstraintValue::Literal(
                        "NOUN".to_string()
                    )))
                );
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::IsChild(Some(label)) if label == "obj"
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 0);

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(
                    constraints.contains(&Constraint::UPOS(ConstraintValue::Literal(
                        "VERB".to_string()
                    )))
                );
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::HasChild(Some(label)) if label == "nsubj"
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 0);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 0);
    }

    #[test]
    fn test_parse_anonymous_multiple() {
        // Test: Multiple anonymous edges on same variable
        let query = r#"MATCH {
            X [upos="NOUN"];
            _ -[obj]-> X;
            _ -[nsubj]-> X;
        }"#;
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 3); // UPOS + 2 HasIncomingEdge
                assert!(
                    constraints.contains(&Constraint::UPOS(ConstraintValue::Literal(
                        "NOUN".to_string()
                    )))
                );
                assert!(
                    constraints
                        .iter()
                        .filter(|c| matches!(c, Constraint::IsChild(_)))
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);

        assert!(matches!(
            &pattern.match_pattern.var_constraints[0],
            Constraint::IsChild(None)
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
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 2);
        assert_eq!(pattern.match_pattern.edge_constraints.len(), 1); // Only X -> Y creates edge constraint

        let x_constraints = &pattern.match_pattern.var_constraints
            [*pattern.match_pattern.var_ids.get("X").unwrap()];
        match x_constraints {
            Constraint::And(constraints) => {
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::IsChild(Some(label)) if label == "obj"
                )));
            }
            _ => panic!("Expected And constraint for X"),
        }
    }

    #[test]
    fn test_duplicate_extension_variable() {
        // Same new variable name in multiple EXCEPT/OPTIONAL blocks
        let query = r#"
            MATCH { V [upos="VERB"]; }
            EXCEPT { M [upos="ADV"]; V -[advmod]-> M; }
            OPTIONAL { M [upos="NOUN"]; V -[obj]-> M; }
        "#;
        let pattern = compile_query(query);
        assert!(matches!(
            pattern,
            Err(QueryError::DuplicateExtensionVariable(_))
        ));

        // Same variable in two EXCEPT blocks
        let query = r#"
            MATCH { V [upos="VERB"]; }
            EXCEPT { M []; V -[advmod]-> M; }
            EXCEPT { M []; V -[obj]-> M; }
        "#;
        let pattern = compile_query(query);
        assert!(matches!(
            pattern,
            Err(QueryError::DuplicateExtensionVariable(_))
        ));
    }

    #[test]
    fn test_parse_comments() {
        // Inline comment with #
        let query = r#"
            MATCH {
                V [upos="VERB"];  # this is a comment
            }
        "#;
        let pattern = compile_query(query).unwrap();
        assert_eq!(pattern.match_pattern.var_ids.len(), 1);

        // Full line comment with //
        let query = r#"
            MATCH {
                // find all verbs
                V [upos="VERB"];
            }
        "#;
        let pattern = compile_query(query).unwrap();
        assert_eq!(pattern.match_pattern.var_ids.len(), 1);

        // Comment at end of query
        let query = r#"MATCH { V [upos="VERB"]; } // trailing comment"#;
        let pattern = compile_query(query).unwrap();
        assert_eq!(pattern.match_pattern.var_ids.len(), 1);
    }

    #[test]
    fn test_parse_regex_constraint() {
        // Test basic regex (anchors added automatically)
        let query = r#"MATCH { V [lemma=/run.*/]; }"#;
        let pattern = compile_query(query).unwrap();

        assert_eq!(pattern.match_pattern.var_constraints.len(), 1);
        match &pattern.match_pattern.var_constraints[0] {
            Constraint::Lemma(ConstraintValue::Regex(pattern_str, _)) => {
                // Pattern string is stored without anchors (anchors are in compiled regex)
                assert_eq!(pattern_str, "run.*");
            }
            _ => panic!("Expected Lemma with Regex constraint"),
        }
    }

    #[test]
    fn test_parse_regex_upos() {
        // Test regex with alternation
        let query = r#"MATCH { W [upos=/VERB|AUX/]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::UPOS(ConstraintValue::Regex(pattern_str, _)) => {
                assert_eq!(pattern_str, "VERB|AUX");
            }
            _ => panic!("Expected UPOS with Regex constraint"),
        }
    }

    #[test]
    fn test_parse_regex_with_escapes() {
        // Test regex with escaped forward slash
        let query = r#"MATCH { W [form=/\w+ing/]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::Form(ConstraintValue::Regex(pattern_str, _)) => {
                assert_eq!(pattern_str, r"\w+ing");
            }
            _ => panic!("Expected Form with Regex constraint"),
        }
    }

    #[test]
    fn test_parse_mixed_literal_and_regex() {
        // Test mixing literal and regex constraints
        let query = r#"MATCH { W [lemma="help" & upos=/VERB|AUX/]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::And(constraints) => {
                assert_eq!(constraints.len(), 2);
                assert!(
                    constraints.contains(&Constraint::Lemma(ConstraintValue::Literal(
                        "help".to_string()
                    )))
                );
                assert!(constraints.iter().any(|c| matches!(
                    c, Constraint::UPOS(ConstraintValue::Regex(p, _)) if p == "VERB|AUX"
                )));
            }
            _ => panic!("Expected And constraint"),
        }
    }

    #[test]
    fn test_parse_regex_feature() {
        // Test regex in feature constraint
        let query = r#"MATCH { V [feats.Tense=/Past|Pres/]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::Feature(key, ConstraintValue::Regex(pattern_str, _)) => {
                assert_eq!(key, "Tense");
                assert_eq!(pattern_str, "Past|Pres");
            }
            _ => panic!("Expected Feature with Regex constraint"),
        }
    }

    #[test]
    fn test_parse_negative_regex() {
        // Test negated regex (anchors added automatically)
        let query = r#"MATCH { V [lemma!=/be.*/]; }"#;
        let pattern = compile_query(query).unwrap();

        match &pattern.match_pattern.var_constraints[0] {
            Constraint::Not(inner) => match inner.as_ref() {
                Constraint::Lemma(ConstraintValue::Regex(pattern_str, _)) => {
                    assert_eq!(pattern_str, "be.*");
                }
                _ => panic!("Expected Lemma with Regex inside Not"),
            },
            _ => panic!("Expected Not constraint"),
        }
    }

    #[test]
    fn test_parse_invalid_regex() {
        // Test that invalid regex patterns are rejected during compilation
        let query = r#"MATCH { V [lemma=/[unclosed/]; }"#;
        let result = compile_query(query);
        assert!(matches!(result, Err(QueryError::InvalidRegex(_, _))));

        // Invalid regex with unbalanced parentheses
        let query = r#"MATCH { V [upos=/VERB(/]; }"#;
        let result = compile_query(query);
        assert!(matches!(result, Err(QueryError::InvalidRegex(_, _))));
    }

    #[test]
    fn test_regex_anchor_behavior() {
        // Test to understand anchor behavior
        use regex::Regex;

        // Double anchor - Rust regex treats ^^ as ^, so it's redundant
        let re1 = Regex::new("^^w.*$").unwrap();
        assert!(re1.is_match("win")); // Matches "win" (^^ is treated as ^)

        // Single anchor
        let re2 = Regex::new("^w.*$").unwrap();
        assert!(re2.is_match("win")); // Matches "win"

        // Both are equivalent
        assert_eq!(re1.is_match("win"), re2.is_match("win"));
        assert_eq!(re1.is_match("running"), re2.is_match("running"));
    }
}
