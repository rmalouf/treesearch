//! Query language parser
//!
//! Parses query strings into Pattern AST using pest grammar.

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::collections::HashSet;
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

    #[error("Query error: Undeclared variable: {0}")]
    UndeclaredVariable(String),

    #[error("Query error: No MATCH block found")]
    NoMATCH,

    #[error("Query error: Variable '{0}' already defined in another EXCEPT/OPTIONAL block")]
    DuplicateExtensionVariable(String),

    #[error(
        "Query error: Variable '{0}' is declared in MATCH and cannot be redeclared in EXCEPT/OPTIONAL"
    )]
    RedeclaredMatchVariable(String),

    #[error("Query error: Invalid regex pattern '{0}': {1}")]
    InvalidRegex(String, String),

    #[error("Query error: Invalid distance '{0}' in '<{0}<': must be a positive integer")]
    InvalidDistance(String),
}

pub fn compile_query(input: &str) -> Result<Pattern, QueryError> {
    let mut match_pattern: Option<BasePattern> = None;
    let mut except_patterns: Vec<BasePattern> = vec![];
    let mut optional_patterns: Vec<BasePattern> = vec![];

    let mut pairs = QueryParser::parse(Rule::query, input)?;
    let query_pair = pairs.next().unwrap();

    // The grammar guarantees MATCH comes first, so its variables are known
    // by the time EXCEPT/OPTIONAL blocks are compiled
    for item in query_pair.into_inner() {
        let match_vars = match_pattern.as_ref().map_or(&[][..], |p| &p.var_names[..]);
        match item.as_rule() {
            Rule::match_block => match_pattern = Some(compile_query_block(item, &[])?),
            Rule::except_block => except_patterns.push(compile_query_block(item, match_vars)?),
            Rule::optional_block => optional_patterns.push(compile_query_block(item, match_vars)?),
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    if let Some(match_pattern) = match_pattern {
        validate_variables(&match_pattern, &except_patterns, &optional_patterns)?;
        Ok(Pattern {
            match_pattern,
            except_patterns,
            optional_patterns,
        })
    } else {
        Err(QueryError::NoMATCH)
    }
}

/// Compile one block. `match_vars` are MATCH variables, which may be used in
/// edges but not redeclared (empty when compiling MATCH itself). Every other
/// variable used in an edge must be declared in the block.
pub fn compile_query_block(
    item: Pair<Rule>,
    match_vars: &[String],
) -> Result<BasePattern, QueryError> {
    let mut vars: Vec<PatternVar> = Vec::new();
    let mut edges: Vec<EdgeConstraint> = Vec::new();

    for statement in item.into_inner() {
        match statement.as_rule() {
            Rule::statement => {
                let inner = statement.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::node_decl => {
                        let var = compile_var_decl(inner)?;
                        if match_vars.contains(&var.var_name) {
                            return Err(QueryError::RedeclaredMatchVariable(var.var_name));
                        }
                        if vars.iter().any(|v| v.var_name == var.var_name) {
                            return Err(QueryError::DuplicateVariable(var.var_name));
                        };
                        vars.push(var);
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

    for edge in &edges {
        for name in [&edge.from, &edge.to] {
            if name != "_"
                && !match_vars.contains(name)
                && !vars.iter().any(|v| &v.var_name == name)
            {
                return Err(QueryError::UndeclaredVariable(name.clone()));
            }
        }
    }

    Ok(BasePattern::with_constraints(vars, edges))
}

/// Validate that new variables in EXCEPT/OPTIONAL blocks are unique across all extension blocks
fn validate_variables(
    match_pattern: &BasePattern,
    except_patterns: &[BasePattern],
    optional_patterns: &[BasePattern],
) -> Result<(), QueryError> {
    let match_vars = &match_pattern.var_names;
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
    match pair.into_inner().next() {
        None => Ok(Constraint::Any),
        Some(or_pair) => compile_or_constraint(or_pair),
    }
}

fn compile_or_constraint(pair: Pair<Rule>) -> Result<Constraint, QueryError> {
    let branches: Vec<Constraint> = pair
        .into_inner()
        .map(compile_and_constraint)
        .collect::<Result<_, _>>()?;
    match branches.len() {
        1 => Ok(branches.into_iter().next().unwrap()),
        _ => Ok(Constraint::Or(branches)),
    }
}

fn compile_and_constraint(pair: Pair<Rule>) -> Result<Constraint, QueryError> {
    let constraints: Vec<Constraint> = pair
        .into_inner()
        .map(compile_atom_constraint)
        .collect::<Result<_, _>>()?;
    match constraints.len() {
        0 => Ok(Constraint::Any),
        1 => Ok(constraints.into_iter().next().unwrap()),
        _ => Ok(Constraint::And(constraints)),
    }
}

fn compile_atom_constraint(pair: Pair<Rule>) -> Result<Constraint, QueryError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::or_constraint => compile_or_constraint(inner),
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
            let anchored_pattern = format!("^(?:{})$", value_str);
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

    let negated = matches!(
        op_rule,
        Rule::neg_labeled_edge | Rule::neg_unlabeled_edge | Rule::neg_regex_edge
    );

    let label = match op_rule {
        Rule::labeled_edge | Rule::neg_labeled_edge => actual_op
            .into_inner()
            .next()
            .map(|p| ConstraintValue::Literal(p.as_str().to_string())),
        Rule::regex_edge | Rule::neg_regex_edge => {
            let regex_lit = actual_op.into_inner().next().unwrap();
            let pattern = regex_lit.into_inner().as_str().to_string();
            let anchored = format!("^(?:{})$", pattern);
            match Regex::new(&anchored) {
                Ok(regex) => Some(ConstraintValue::Regex(pattern, regex)),
                Err(e) => return Err(QueryError::InvalidRegex(pattern, e.to_string())),
            }
        }
        _ => None,
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
    let operator = inner.next().unwrap();
    let to = inner.next().unwrap().as_str().to_string();

    let relation = match operator.as_str() {
        "<<" => RelationType::Precedes,
        "<" => RelationType::ImmediatelyPrecedes,
        _ => {
            // within_op: <n<
            let distance = operator
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap();
            match distance.as_str().parse::<usize>() {
                Ok(n) if n > 0 => RelationType::PrecedesWithin(n),
                _ => return Err(QueryError::InvalidDistance(distance.as_str().to_string())),
            }
        }
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
    use Constraint::*;

    fn lit(s: &str) -> ConstraintValue {
        ConstraintValue::Literal(s.to_string())
    }

    /// Regex value for comparisons (equality only looks at the pattern string)
    fn re(s: &str) -> ConstraintValue {
        ConstraintValue::Regex(s.to_string(), Regex::new(s).unwrap())
    }

    fn not(c: Constraint) -> Constraint {
        Not(Box::new(c))
    }

    fn compile_match(body: &str) -> BasePattern {
        compile_query(&format!("MATCH {{ {body} }}"))
            .unwrap()
            .match_pattern
    }

    #[test]
    fn test_parse_node_constraints() {
        let cases = [
            ("N []", Any),
            (r#"V [lemma="run"]"#, Lemma(lit("run"))),
            (r#"V [upos="VERB"]"#, UPOS(lit("VERB"))),
            (r#"V [xpos="VBZ"]"#, XPOS(lit("VBZ"))),
            (r#"V [form="running"]"#, Form(lit("running"))),
            (r#"V [deprel="nsubj"]"#, DepRel(lit("nsubj"))),
            (
                r#"V [feats.Tense="Past"]"#,
                Feature("Tense".into(), lit("Past")),
            ),
            (
                r#"V [misc.SpaceAfter="No"]"#,
                Misc("SpaceAfter".into(), lit("No")),
            ),
            // Negation
            (r#"V [lemma!="help"]"#, not(Lemma(lit("help")))),
            (
                r#"V [feats.Tense!="Past"]"#,
                not(Feature("Tense".into(), lit("Past"))),
            ),
            // Regexes: stored without the anchors that are added on compilation
            (r#"V [lemma=/run.*/]"#, Lemma(re("run.*"))),
            (r#"V [form=/\w+ing/]"#, Form(re(r"\w+ing"))),
            (
                r#"V [feats.Tense=/Past|Pres/]"#,
                Feature("Tense".into(), re("Past|Pres")),
            ),
            (r#"V [lemma!=/be.*/]"#, not(Lemma(re("be.*")))),
            // Boolean combinations
            (
                r#"V [lemma="run" & upos!="NOUN"]"#,
                And(vec![Lemma(lit("run")), not(UPOS(lit("NOUN")))]),
            ),
            (
                r#"V [lemma="be" & feats.Tense=/Past|Pres/]"#,
                And(vec![
                    Lemma(lit("be")),
                    Feature("Tense".into(), re("Past|Pres")),
                ]),
            ),
            (
                r#"N [upos="NOUN" | upos="PROPN"]"#,
                Or(vec![UPOS(lit("NOUN")), UPOS(lit("PROPN"))]),
            ),
            (
                // & binds tighter than |
                r#"V [lemma="be" & upos="VERB" | lemma="have" & upos="AUX"]"#,
                Or(vec![
                    And(vec![Lemma(lit("be")), UPOS(lit("VERB"))]),
                    And(vec![Lemma(lit("have")), UPOS(lit("AUX"))]),
                ]),
            ),
            (
                r#"N [(upos="NOUN" | upos="PROPN") & deprel="nsubj"]"#,
                And(vec![
                    Or(vec![UPOS(lit("NOUN")), UPOS(lit("PROPN"))]),
                    DepRel(lit("nsubj")),
                ]),
            ),
            // Edges to `_` become constraints on the named end
            (r#"X []; _ -> X"#, IsChild(None)),
            (r#"X []; _ !-> X"#, not(IsChild(None))),
            (
                r#"X [upos="NOUN"]; _ -[obj]-> X"#,
                And(vec![UPOS(lit("NOUN")), IsChild(Some(lit("obj")))]),
            ),
            (
                r#"X [upos="NOUN"]; _ -[obj]-> X; _ -/nsubj.*/-> X"#,
                And(vec![
                    UPOS(lit("NOUN")),
                    IsChild(Some(lit("obj"))),
                    IsChild(Some(re("nsubj.*"))),
                ]),
            ),
            (
                r#"X [upos="VERB"]; X -[nsubj]-> _"#,
                And(vec![UPOS(lit("VERB")), HasChild(Some(lit("nsubj")))]),
            ),
            (r#"X []; X !-> _"#, not(HasChild(None))),
            // A labeled edge constrains the target's deprel, unless it is negated
            (r#"X []; Y []; Y -[obj]-> X"#, DepRel(lit("obj"))),
            (r#"X []; Y []; Y !-[obj]-> X"#, Any),
        ];

        // The first declared variable is the one under test
        for (body, expected) in cases {
            assert_eq!(compile_match(body).var_constraints[0], expected, "{body}");
        }
    }

    #[test]
    fn test_parse_edges() {
        use RelationType::*;
        let cases = [
            ("A -> B", Child, None, false),
            ("A -[xcomp]-> B", Child, Some(lit("xcomp")), false),
            ("A !-> B", Child, None, true),
            ("A !-[xcomp]-> B", Child, Some(lit("xcomp")), true),
            ("A -/nsubj.*/-> B", Child, Some(re("nsubj.*")), false),
            ("A !-/obj|iobj/-> B", Child, Some(re("obj|iobj")), true),
            ("A << B", Precedes, None, false),
            ("A < B", ImmediatelyPrecedes, None, false),
            ("A <3< B", PrecedesWithin(3), None, false),
            ("A<12<B", PrecedesWithin(12), None, false),
        ];

        for (edge, relation, label, negated) in cases {
            let pattern = compile_match(&format!("A []; B []; {edge};"));
            let [e] = &pattern.edge_constraints[..] else {
                panic!("{edge}: expected one edge constraint");
            };
            assert_eq!(
                (
                    e.from.as_str(),
                    e.to.as_str(),
                    e.relation,
                    &e.label,
                    e.negated
                ),
                ("A", "B", relation, &label, negated),
                "{edge}"
            );
        }
    }

    #[test]
    fn test_parse_query_structure() {
        let pattern = compile_match(
            r#"
            // Variables can be used before they are declared
            Help [lemma="help"];    # inline comment
            Help -[xcomp]-> To;
            To [lemma="to"];
            To << Y;
            Y [];
            _ -[obj]-> Y;           // edges to `_` are not edge constraints
            _ -> _;                 // and this one is ignored entirely
            "#,
        );

        assert_eq!(pattern.var_names, ["Help", "To", "Y"]);
        let edges: Vec<_> = pattern
            .edge_constraints
            .iter()
            .map(|e| (e.from.as_str(), e.to.as_str(), e.relation))
            .collect();
        assert_eq!(
            edges,
            [
                ("Help", "To", RelationType::Child),
                ("To", "Y", RelationType::Precedes)
            ]
        );

        assert!(compile_match("_ -> _;").var_names.is_empty());
        assert!(compile_query(r#"MATCH { V []; } // trailing comment"#).is_ok());
    }

    #[test]
    fn test_compile_errors() {
        use QueryError::*;
        type Check = fn(&QueryError) -> bool;
        let undeclared: Check = |e| matches!(e, UndeclaredVariable(_));
        let duplicate_ext: Check = |e| matches!(e, DuplicateExtensionVariable(_));
        let redeclared: Check = |e| matches!(e, RedeclaredMatchVariable(v) if v == "V");
        let regex: Check = |e| matches!(e, InvalidRegex(..));
        let distance: Check = |e| matches!(e, InvalidDistance(_));
        let syntax: Check = |e| matches!(e, ParseError(_));

        let cases: [(&str, Check); 18] = [
            ("MATCH { Node []; Foo -> Node; }", undeclared),
            ("MATCH { Node []; Node -> Bar; }", undeclared),
            ("MATCH { Node []; Node < Bar; }", undeclared),
            ("MATCH { Node []; } EXCEPT { Node -> Bar; }", undeclared),
            (
                "MATCH { Node []; } OPTIONAL { Node -[obj]-> Bar; }",
                undeclared,
            ),
            (r#"MATCH { N [upos="NOUN"]; N [upos="VERB"]; }"#, |e| {
                matches!(e, DuplicateVariable(_))
            }),
            // New variable names must be unique across EXCEPT/OPTIONAL blocks
            (
                "MATCH { V []; } EXCEPT { M []; V -> M; } OPTIONAL { M []; V -> M; }",
                duplicate_ext,
            ),
            (
                "MATCH { V []; } EXCEPT { M []; V -> M; } EXCEPT { M []; V -> M; }",
                duplicate_ext,
            ),
            // ... and can't redeclare MATCH variables
            ("MATCH { V []; } EXCEPT { V []; }", redeclared),
            ("MATCH { V []; } OPTIONAL { V []; }", redeclared),
            ("MATCH { V [lemma=/[unclosed/]; }", regex),
            ("MATCH { V [upos=/VERB(/]; }", regex),
            ("MATCH { A []; B []; A -/(/-> B; }", regex),
            ("MATCH { A []; B []; A <0< B; }", distance),
            (
                "MATCH { A []; B []; A <99999999999999999999999< B; }",
                distance,
            ),
            ("MATCH { A []; B []; A < 3 < B; }", syntax),
            ("MATCH { A []; B []; A <x< B; }", syntax),
            ("EXCEPT { A []; }", syntax),
        ];
        for (query, check) in cases {
            let err = compile_query(query).unwrap_err();
            assert!(check(&err), "{query}: {err:?}");
        }

        // MATCH variables can be used in extension block edges
        for block in ["EXCEPT", "OPTIONAL"] {
            let query = format!("MATCH {{ V []; }} {block} {{ O []; V -[obj]-> O; }}");
            assert!(compile_query(&query).is_ok(), "{query}");
        }
    }
}
