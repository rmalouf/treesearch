//! Query language parser
//!
//! Parses query strings into Pattern AST using pest grammar.

use pest::Parser;
use pest_derive::Parser;

use crate::pattern::{Constraint, Pattern, PatternEdge, PatternNode, RelationType};

#[derive(Parser)]
#[grammar = "query.pest"]
struct QueryParser;

/// Error type for parse failures
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(err: pest::error::Error<Rule>) -> Self {
        ParseError {
            message: err.to_string(),
        }
    }
}

/// Parse a query string into a Pattern
pub fn parse_query(input: &str) -> Result<Pattern, ParseError> {
    let mut pairs = QueryParser::parse(Rule::query, input)?;
    let mut pattern = Pattern::new();

    // Get the query rule (there should be exactly one)
    let Some(query_pair) = pairs.next() else {
        return Err(ParseError {
            message: "No query found".to_string(),
        });
    };

    // Process all statements in the query
    for statement in query_pair.into_inner() {
        match statement.as_rule() {
            Rule::statement => {
                // statement contains either node_decl or edge_decl
                let Some(inner) = statement.into_inner().next() else {
                    return Err(ParseError {
                        message: "Empty statement".to_string(),
                    });
                };

                match inner.as_rule() {
                    Rule::node_decl => {
                        let element = parse_node_decl(inner)?;
                        pattern.add_node(element);
                    }
                    Rule::edge_decl => {
                        let edge = parse_edge_decl(inner)?;
                        pattern.add_edge(edge);
                    }
                    _ => {}
                }
            }
            Rule::EOI => {} // End of input
            _ => {}
        }
    }

    pattern.compile_pattern();
    Ok(pattern)
}

/// Parse a node declaration: Name [constraint, constraint];
fn parse_node_decl(pair: pest::iterators::Pair<Rule>) -> Result<PatternNode, ParseError> {
    let mut inner = pair.into_inner();

    let Some(ident_pair) = inner.next() else {
        return Err(ParseError {
            message: "Expected identifier in node declaration".to_string(),
        });
    };
    let ident = ident_pair.as_str().to_string();

    let Some(constraint_list) = inner.next() else {
        return Err(ParseError {
            message: "Expected constraint list".to_string(),
        });
    };

    let constraints = parse_constraint_list(constraint_list)?;

    Ok(PatternNode::new(&ident, constraints))
}

/// Parse constraint list: may be empty or comma-separated constraints
fn parse_constraint_list(pair: pest::iterators::Pair<Rule>) -> Result<Constraint, ParseError> {
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
fn parse_constraint(pair: pest::iterators::Pair<Rule>) -> Result<Constraint, ParseError> {
    let mut inner = pair.into_inner();

    let Some(key_pair) = inner.next() else {
        return Err(ParseError {
            message: "Expected constraint key".to_string(),
        });
    };
    let key = key_pair.as_str();

    let Some(value_pair) = inner.next() else {
        return Err(ParseError {
            message: "Expected constraint value".to_string(),
        });
    };

    // Extract string from string_literal rule
    let Some(value_inner) = value_pair.into_inner().next() else {
        return Err(ParseError {
            message: "Expected string inner".to_string(),
        });
    };
    let value = value_inner.as_str().to_string();

    match key {
        "lemma" => Ok(Constraint::Lemma(value)),
        "pos" => Ok(Constraint::POS(value)),
        "form" => Ok(Constraint::Form(value)),
        "deprel" => Ok(Constraint::DepRel(value)),
        _ => Err(ParseError {
            message: format!("Unknown constraint key: {}", key),
        }),
    }
}

/// Parse edge declaration: Parent -[label]-> Child; or Parent -> Child;
fn parse_edge_decl(pair: pest::iterators::Pair<Rule>) -> Result<PatternEdge, ParseError> {
    let mut inner = pair.into_inner();

    let Some(from_pair) = inner.next() else {
        return Err(ParseError {
            message: "Expected source node in edge".to_string(),
        });
    };
    let from = from_pair.as_str().to_string();

    // The next element could be edge_label (if present) or the target node
    let Some(next) = inner.next() else {
        return Err(ParseError {
            message: "Expected edge label or target node".to_string(),
        });
    };

    let (label, to) = if next.as_rule() == Rule::edge_label {
        // We have a label, so get the target node next
        let label_str = next.as_str().to_string();
        let Some(to_pair) = inner.next() else {
            return Err(ParseError {
                message: "Expected target node in edge".to_string(),
            });
        };
        let to_node = to_pair.as_str().to_string();
        (Some(label_str), to_node)
    } else {
        // No label, this is the target node
        (None, next.as_str().to_string())
    };

    // For now, all edges are Child relations (parent -> child)
    // We can extend this later to support different arrow types
    Ok(PatternEdge {
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
    fn test_parse_empty_constraint() {
        let query = "Node [];";
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.nodes.len(), 1);
        assert_eq!(pattern.nodes[0].var_name, "Node");
        assert!(pattern.nodes[0].constraints.is_any());
    }

    #[test]
    fn test_parse_single_constraint() {
        let query = r#"Verb [pos="VERB"];"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.nodes.len(), 1);
        assert_eq!(pattern.nodes[0].var_name, "Verb");
        assert_eq!(
            pattern.nodes[0].constraints,
            Constraint::POS("VERB".to_string())
        );
    }

    #[test]
    fn test_parse_multiple_constraints() {
        let query = r#"Help [lemma="help", pos="VERB"];"#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.nodes.len(), 1);
        assert_eq!(pattern.nodes[0].var_name, "Help");

        match &pattern.nodes[0].constraints {
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

        assert!(pattern.compiled);
        assert_eq!(pattern.nodes.len(), 2);
        assert_eq!(pattern.edges.len(), 1);

        let edge = &pattern.edges[0];
        assert_eq!(edge.from, "Help");
        assert_eq!(edge.to, "To");
        assert_eq!(edge.relation, RelationType::Child);
        assert_eq!(edge.label, Some("xcomp".to_string()));
    }

    #[test]
    fn test_parse_unconstrained_edge() {
        let query = r#"
            Parent [];
            Child [];
            Parent -> Child;
        "#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.nodes.len(), 2);
        assert_eq!(pattern.edges.len(), 1);

        let edge = &pattern.edges[0];
        assert_eq!(edge.from, "Parent");
        assert_eq!(edge.to, "Child");
        assert_eq!(edge.relation, RelationType::Child);
        assert_eq!(edge.label, None);
    }

    #[test]
    fn test_parse_mixed_edges() {
        let query = r#"
            A [];
            B [];
            C [];
            A -[nsubj]-> B;
            B -> C;
        "#;
        let pattern = parse_query(query).unwrap();

        assert_eq!(pattern.nodes.len(), 3);
        assert_eq!(pattern.edges.len(), 2);

        // First edge has a label
        assert_eq!(pattern.edges[0].label, Some("nsubj".to_string()));

        // Second edge has no label (unconstrained)
        assert_eq!(pattern.edges[1].label, None);
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

        assert_eq!(pattern.nodes.len(), 3);
        assert_eq!(pattern.edges.len(), 2);

        // Verify nodes
        assert_eq!(pattern.nodes[0].var_name, "Help");
        assert_eq!(pattern.nodes[1].var_name, "To");
        assert_eq!(pattern.nodes[2].var_name, "YHead");

        // Verify edges
        assert_eq!(pattern.edges[0].from, "Help");
        assert_eq!(pattern.edges[0].to, "To");
        assert_eq!(pattern.edges[1].from, "To");
        assert_eq!(pattern.edges[1].to, "YHead");
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

        assert_eq!(pattern.nodes.len(), 4);
        assert_eq!(
            pattern.nodes[0].constraints,
            Constraint::Lemma("run".to_string())
        );
        assert_eq!(
            pattern.nodes[1].constraints,
            Constraint::POS("VERB".to_string())
        );
        assert_eq!(
            pattern.nodes[2].constraints,
            Constraint::Form("running".to_string())
        );
        assert_eq!(
            pattern.nodes[3].constraints,
            Constraint::DepRel("nsubj".to_string())
        );
    }
}
