//! Pattern compilation to VM opcodes
//!
//! This module compiles high-level Pattern AST into optimized VM opcodes.
//! The compiler uses an anchor-based strategy with interleaved verification.

use crate::pattern::{Constraint, Pattern, PatternEdge, RelationType};
use crate::vm::Instruction;
use std::collections::HashMap;

/// Selectivity estimate for choosing the best anchor
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Selectivity {
    /// Very selective (e.g., specific lemma)
    High = 3,
    /// Moderately selective (e.g., POS tag)
    Medium = 2,
    /// Low selectivity (e.g., any node)
    Low = 1,
}

/// Estimate the selectivity of a constraint
fn estimate_selectivity(constraint: &Constraint) -> Selectivity {
    match constraint {
        Constraint::Any => Selectivity::Low,
        Constraint::Lemma(_) => Selectivity::High,
        Constraint::Form(_) => Selectivity::High,
        Constraint::POS(_) => Selectivity::Medium,
        Constraint::DepRel(_) => Selectivity::Medium,
        Constraint::And(constraints) => {
            // And is as selective as its most selective constraint
            assert!(
                !constraints.is_empty(),
                "Compiler bug: empty And constraint"
            );
            constraints.iter().map(estimate_selectivity).max().unwrap()
        }
        Constraint::Or(constraints) => {
            // Or is as selective as its least selective constraint
            assert!(!constraints.is_empty(), "Compiler bug: empty Or constraint");
            constraints.iter().map(estimate_selectivity).min().unwrap()
        }
    }
}

/// Select the best anchor element for a pattern
/// Returns the index of the most selective element
fn select_anchor(pattern: &Pattern) -> usize {
    assert!(
        !pattern.elements.is_empty(),
        "Compiler bug: cannot compile empty pattern"
    );

    let mut best_idx = 0;
    let mut best_selectivity = Selectivity::Low;

    for (idx, element) in pattern.elements.iter().enumerate() {
        let selectivity = estimate_selectivity(&element.constraints);
        if selectivity > best_selectivity {
            best_selectivity = selectivity;
            best_idx = idx;
        }
    }

    best_idx
}

/// Compile a constraint into a sequence of check instructions
fn compile_constraint(constraint: Constraint) -> Vec<Instruction> {
    match constraint {
        Constraint::Any => Vec::new(), // No check needed
        Constraint::Lemma(lemma) => vec![Instruction::CheckLemma(lemma)],
        Constraint::Form(form) => vec![Instruction::CheckForm(form)],
        Constraint::POS(pos) => vec![Instruction::CheckPOS(pos)],
        Constraint::DepRel(deprel) => vec![Instruction::CheckDepRel(deprel)],
        Constraint::And(constraints) => {
            // Compile all constraints sequentially
            constraints
                .into_iter()
                .flat_map(compile_constraint)
                .collect()
        }
        Constraint::Or(constraints) => {
            // For Or, we'd need Choice/alternatives which is complex
            // For now, just compile first constraint
            // TODO: Implement proper Or compilation with Choice in future
            assert!(!constraints.is_empty(), "Compiler bug: empty Or constraint");
            compile_constraint(constraints.into_iter().next().unwrap())
        }
    }
}

/// Compile an edge into navigation instruction(s)
fn compile_edge(
    relation: RelationType,
    label: Option<&str>,
    constraint: Constraint,
) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    // Generate navigation instruction
    match relation {
        RelationType::Child => {
            if constraint.is_any() {
                instructions.push(Instruction::MoveToChild(None));
            } else {
                instructions.push(Instruction::MoveToChild(Some(constraint)));
            }
        }
        RelationType::Parent => {
            instructions.push(Instruction::MoveToParent);
        }
        RelationType::Descendant => {
            instructions.push(Instruction::ScanDescendants(constraint));
        }
        RelationType::Ancestor => {
            instructions.push(Instruction::ScanAncestors(constraint));
        }
        RelationType::Follows => {
            instructions.push(Instruction::ScanSiblings(constraint, true));
        }
        RelationType::Precedes => {
            instructions.push(Instruction::ScanSiblings(constraint, false));
        }
    }

    // Add edge label check if specified
    if let Some(deprel) = label {
        instructions.push(Instruction::CheckDepRel(deprel.to_string()));
    }

    instructions
}

/// Compile a pattern into VM opcodes
/// Returns (opcodes, anchor_index, var_names)
pub fn compile_pattern(pattern: Pattern) -> (Vec<Instruction>, usize, Vec<String>) {
    if pattern.elements.is_empty() {
        return (vec![Instruction::Match], 0, Vec::new());
    }

    let anchor_idx = select_anchor(&pattern);

    // Destructure pattern to take ownership of parts
    let Pattern { elements, edges } = pattern;

    // Extract variable names in position order
    let var_names: Vec<String> = elements.iter().map(|elem| elem.var_name.clone()).collect();

    let mut opcodes = Vec::new();

    // Build a map of element names to indices (moving var_name)
    let name_to_idx: HashMap<String, usize> = elements
        .iter()
        .enumerate()
        .map(|(idx, elem)| (elem.var_name.clone(), idx))
        .collect();

    // Build adjacency list from edges
    let mut edges_from: HashMap<usize, Vec<(usize, PatternEdge)>> = HashMap::new();
    for edge in edges {
        if let (Some(&from_idx), Some(&to_idx)) =
            (name_to_idx.get(&edge.from), name_to_idx.get(&edge.to))
        {
            edges_from.entry(from_idx).or_default().push((to_idx, edge));
        }
    }

    // Start at anchor: verify its constraints and bind it
    let anchor_element = &elements[anchor_idx];
    opcodes.extend(compile_constraint(anchor_element.constraints.clone()));
    opcodes.push(Instruction::Bind(anchor_idx));

    // Track which elements we've visited
    let mut visited = vec![false; elements.len()];
    visited[anchor_idx] = true;

    // BFS traversal from anchor to compile verification of connected nodes
    let mut queue = vec![anchor_idx];

    while let Some(current_idx) = queue.pop() {
        // Check edges from this node
        if let Some(edges_list) = edges_from.get(&current_idx) {
            for (target_idx, edge) in edges_list {
                if visited[*target_idx] {
                    continue;
                }

                // Save state before navigating
                opcodes.push(Instruction::PushState);

                // Navigate to target
                let target_element = &elements[*target_idx];
                let navigation = compile_edge(
                    edge.relation,
                    edge.label.as_deref(),
                    target_element.constraints.clone(),
                );
                opcodes.extend(navigation);

                // Verify target constraints (if not already in navigation)
                if !matches!(
                    edge.relation,
                    RelationType::Child
                        | RelationType::Descendant
                        | RelationType::Ancestor
                        | RelationType::Follows
                        | RelationType::Precedes
                ) {
                    opcodes.extend(compile_constraint(target_element.constraints.clone()));
                }

                // Bind target
                opcodes.push(Instruction::Bind(*target_idx));

                // Mark as visited and add to queue
                visited[*target_idx] = true;
                queue.push(*target_idx);
            }
        }
    }

    // Final match instruction
    opcodes.push(Instruction::Match);

    (opcodes, anchor_idx, var_names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{Constraint, Pattern, PatternEdge, PatternElement, RelationType};

    #[test]
    fn test_selectivity_estimation() {
        assert_eq!(estimate_selectivity(&Constraint::Any), Selectivity::Low);
        assert_eq!(
            estimate_selectivity(&Constraint::Lemma("test".to_string())),
            Selectivity::High
        );
        assert_eq!(
            estimate_selectivity(&Constraint::POS("VERB".to_string())),
            Selectivity::Medium
        );
    }

    #[test]
    fn test_anchor_selection() {
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new("any", Constraint::Any));
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::POS("VERB".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "help",
            Constraint::Lemma("help".to_string()),
        ));

        let anchor = select_anchor(&pattern);
        assert_eq!(anchor, 2); // Should select "help" (most selective)
    }

    #[test]
    fn test_compile_simple_constraint() {
        let constraint = Constraint::Lemma("run".to_string());
        let opcodes = compile_constraint(constraint);
        assert_eq!(opcodes.len(), 1);
        assert_eq!(opcodes[0], Instruction::CheckLemma("run".to_string()));
    }

    #[test]
    fn test_compile_and_constraint() {
        let constraint = Constraint::And(vec![
            Constraint::POS("VERB".to_string()),
            Constraint::Lemma("run".to_string()),
        ]);
        let opcodes = compile_constraint(constraint);
        assert_eq!(opcodes.len(), 2);
        assert_eq!(opcodes[0], Instruction::CheckPOS("VERB".to_string()));
        assert_eq!(opcodes[1], Instruction::CheckLemma("run".to_string()));
    }

    #[test]
    fn test_compile_simple_pattern() {
        // Pattern: single element with POS constraint
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::POS("VERB".to_string()),
        ));

        let (opcodes, anchor, _var_names) = compile_pattern(pattern);
        assert_eq!(anchor, 0);

        // Check exact opcodes: CheckPOS, Bind, Match
        assert_eq!(opcodes.len(), 3);
        assert_eq!(opcodes[0], Instruction::CheckPOS("VERB".to_string()));
        assert_eq!(opcodes[1], Instruction::Bind(0));
        assert_eq!(opcodes[2], Instruction::Match);
    }

    #[test]
    fn test_compile_pattern_with_edge() {
        // Pattern: VERB -[nsubj]-> NOUN
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::POS("VERB".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "noun",
            Constraint::POS("NOUN".to_string()),
        ));
        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Child,
            label: Some("nsubj".to_string()),
        });

        let (opcodes, anchor, _var_names) = compile_pattern(pattern);
        assert_eq!(anchor, 0); // Both have same selectivity, picks first

        // Check exact opcodes:
        // - Check verb POS
        // - Bind verb
        // - Push state
        // - Move to child (with noun constraint)
        // - Check deprel
        // - Bind noun
        // - Match
        assert_eq!(opcodes.len(), 7);
        assert_eq!(opcodes[0], Instruction::CheckPOS("VERB".to_string()));
        assert_eq!(opcodes[1], Instruction::Bind(0));
        assert_eq!(opcodes[2], Instruction::PushState);
        assert_eq!(
            opcodes[3],
            Instruction::MoveToChild(Some(Constraint::POS("NOUN".to_string())))
        );
        assert_eq!(opcodes[4], Instruction::CheckDepRel("nsubj".to_string()));
        assert_eq!(opcodes[5], Instruction::Bind(1));
        assert_eq!(opcodes[6], Instruction::Match);
    }

    #[test]
    fn test_compile_and_execute_simple_pattern() {
        use crate::tree::{Node, Tree};
        use crate::vm::VM;

        // Create a simple tree: "runs" (VERB)
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));

        // Create pattern: match VERB with lemma "run"
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::And(vec![
                Constraint::POS("VERB".to_string()),
                Constraint::Lemma("run".to_string()),
            ]),
        ));

        let (opcodes, _anchor, _var_names) = compile_pattern(pattern);
        let vm = VM::new(opcodes, Vec::new());
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0);
    }

    #[test]
    fn test_compile_and_execute_pattern_with_child() {
        use crate::tree::{Node, Tree};
        use crate::vm::VM;

        // Create tree: "runs" (VERB) -> "dog" (NOUN, nsubj)
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.set_parent(1, 0);

        // Create pattern: VERB -[nsubj]-> NOUN
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::POS("VERB".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "noun",
            Constraint::POS("NOUN".to_string()),
        ));
        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Child,
            label: Some("nsubj".to_string()),
        });

        let (opcodes, _anchor, _var_names) = compile_pattern(pattern);
        let vm = VM::new(opcodes, Vec::new());
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0); // verb
        assert_eq!(match_result.bindings[&1], 1); // noun
    }

    #[test]
    fn test_compile_and_execute_descendant_pattern() {
        use crate::tree::{Node, Tree};
        use crate::vm::VM;

        // Create tree with depth:
        // 0: runs (VERB)
        //   └─ 1: dog (NOUN)
        //        └─ 2: big (ADJ)
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "big", "big", "ADJ", "amod"));
        tree.set_parent(1, 0);
        tree.set_parent(2, 1);

        // Create pattern: VERB ... ADJ (descendant relation)
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::POS("VERB".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "adj",
            Constraint::POS("ADJ".to_string()),
        ));
        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "adj".to_string(),
            relation: RelationType::Descendant,
            label: None,
        });

        let (opcodes, _anchor, _var_names) = compile_pattern(pattern);
        let vm = VM::new(opcodes, Vec::new());
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0); // verb
        assert_eq!(match_result.bindings[&1], 2); // adj
    }

    #[test]
    fn test_compile_pattern_selects_best_anchor() {
        // Pattern with different selectivities
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new("any", Constraint::Any));
        pattern.add_element(PatternElement::new(
            "pos",
            Constraint::POS("VERB".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "lemma",
            Constraint::Lemma("help".to_string()),
        ));

        let (_opcodes, anchor, _var_names) = compile_pattern(pattern);
        assert_eq!(anchor, 2); // Should select "lemma" (most selective)
    }

    #[test]
    fn test_compile_complex_pattern() {
        use crate::tree::{Node, Tree};
        use crate::vm::VM;

        // Create tree: "help" (VERB) -[xcomp]-> "to" (PART) -[obj]-> "write" (VERB)
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "help", "help", "VERB", "root"));
        tree.add_node(Node::new(1, "to", "to", "PART", "xcomp"));
        tree.add_node(Node::new(2, "write", "write", "VERB", "obj"));
        tree.set_parent(1, 0);
        tree.set_parent(2, 1);

        // Create pattern: help -[xcomp]-> to -[obj]-> write
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "help",
            Constraint::Lemma("help".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "to",
            Constraint::Lemma("to".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "yhead",
            Constraint::POS("VERB".to_string()),
        ));

        pattern.add_edge(PatternEdge {
            from: "help".to_string(),
            to: "to".to_string(),
            relation: RelationType::Child,
            label: Some("xcomp".to_string()),
        });
        pattern.add_edge(PatternEdge {
            from: "to".to_string(),
            to: "yhead".to_string(),
            relation: RelationType::Child,
            label: Some("obj".to_string()),
        });

        let (opcodes, anchor, _var_names) = compile_pattern(pattern);
        // Should anchor on "help" or "to" (both lemmas, equally selective)
        assert!(anchor == 0 || anchor == 1);

        let vm = VM::new(opcodes, Vec::new());
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0); // help
        assert_eq!(match_result.bindings[&1], 1); // to
        assert_eq!(match_result.bindings[&2], 2); // write
    }
}
