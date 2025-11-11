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
            assert!(!constraints.is_empty(), "Compiler bug: empty And constraint");
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

/// Reverse a relation type (for backward edge traversal)
fn reverse_relation(relation: RelationType) -> RelationType {
    match relation {
        RelationType::Child => RelationType::Parent,
        RelationType::Parent => RelationType::Child,
        RelationType::Descendant => RelationType::Ancestor,
        RelationType::Ancestor => RelationType::Descendant,
        RelationType::Follows => RelationType::Precedes,
        RelationType::Precedes => RelationType::Follows,
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
///
/// # Current Limitations
///
/// The compiler has a known limitation when the anchor node has edges in certain
/// combinations that require navigating back using a Child relation. Specifically:
///
/// 1. **Parent edge + Child edge from anchor**: If the anchor has both a Parent edge
///    (to its parent) and a Child edge (to a child), the compiler cannot navigate
///    back from the parent to continue to the child.
///
/// 2. **Multiple Parent edges from anchor**: If the anchor has multiple Parent edges,
///    the compiler cannot navigate back after visiting the first parent.
///
/// These cases will panic with "Cannot navigate back from child to parent".
///
/// **Workaround**: These patterns still work if the anchor is selected differently
/// (e.g., by adjusting selectivity so a node with only Child edges becomes the anchor).
///
/// See tests: `test_anchor_with_parent_edge_and_child_edge_panics` and
/// `test_anchor_with_multiple_parent_edges_panics`
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

    // Build adjacency list from edges (both forward and backward)
    let mut edges_from: HashMap<usize, Vec<(usize, PatternEdge, bool)>> = HashMap::new();
    for edge in edges {
        if let (Some(&from_idx), Some(&to_idx)) =
            (name_to_idx.get(&edge.from), name_to_idx.get(&edge.to))
        {
            // Forward edge: from -> to
            edges_from
                .entry(from_idx)
                .or_default()
                .push((to_idx, edge.clone(), false));
            // Backward edge: to -> from (reversed)
            edges_from
                .entry(to_idx)
                .or_default()
                .push((from_idx, edge, true));
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
        // Collect unvisited edges from this node
        let unvisited_edges: Vec<_> = edges_from
            .get(&current_idx)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|(target_idx, _, _)| !visited[*target_idx])
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for (i, &(target_idx, edge, is_reversed)) in unvisited_edges.iter().enumerate() {
            // Determine the actual relation to use (reverse if needed)
            let actual_relation = if *is_reversed {
                reverse_relation(edge.relation)
            } else {
                edge.relation
            };

            // Always save state before navigating (needed for backtracking)
            opcodes.push(Instruction::PushState);

            // Navigate to target
            let target_element = &elements[*target_idx];
            let navigation = compile_edge(
                actual_relation,
                edge.label.as_deref(),
                target_element.constraints.clone(),
            );
            opcodes.extend(navigation);

            // Verify target constraints (if not already in navigation)
            if !matches!(
                actual_relation,
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

            // Navigate back to current node for next edge (except for last edge)
            // This ensures bindings are preserved while position is reset
            if i < unvisited_edges.len() - 1 {
                // Navigate back using the reverse relation
                let return_relation = reverse_relation(actual_relation);
                match return_relation {
                    RelationType::Parent => opcodes.push(Instruction::MoveToParent),
                    RelationType::Child => {
                        // LIMITATION: Cannot navigate back when return requires Child relation.
                        // This happens when:
                        // 1. Anchor has Parent edge (navigates to parent), then needs to
                        //    navigate back (Child) to process another edge
                        // 2. Multiple Parent edges from anchor
                        // See compile_pattern() docs and panic detection tests for details.
                        panic!(
                            "Compiler limitation: Cannot navigate back from child to parent in current implementation. \
                             This occurs when the anchor has multiple edges that require returning via Child relation. \
                             Try adjusting pattern to select a different anchor."
                        );
                    }
                    _ => {
                        // For scan operations, we need restore
                        opcodes.push(Instruction::RestoreState);
                    }
                }
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
    use crate::tree::{Node, Tree};
    use crate::vm::VM;

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
        let mut result = vm.execute(&tree, 0);

        let match_result = result.next().expect("Should have a match");
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
        let mut result = vm.execute(&tree, 0);

        let match_result = result.next().expect("Should have a match");
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
        let mut result = vm.execute(&tree, 0);

        let match_result = result.next().expect("Should have a match");
        assert_eq!(match_result.bindings[&0], 0); // verb
        assert_eq!(match_result.bindings[&1], 2); // adj
    }

    #[test]
    fn test_unconstrained_deprel_matches_any() {
        // Test that unconstrained edges (label: None) match any deprel
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "parent",
            Constraint::POS("VERB".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "child",
            Constraint::POS("NOUN".to_string()),
        ));
        pattern.add_edge(PatternEdge {
            from: "parent".to_string(),
            to: "child".to_string(),
            relation: RelationType::Child,
            label: None, // Unconstrained - should match any deprel
        });

        let (opcodes, _anchor, _var_names) = compile_pattern(pattern);

        // Verify no CheckDepRel instruction is generated
        assert!(
            !opcodes
                .iter()
                .any(|op| matches!(op, Instruction::CheckDepRel(_)))
        );

        // Test tree 1: with nsubj relation
        let mut tree1 = Tree::new();
        tree1.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree1.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree1.set_parent(1, 0);

        let vm1 = VM::new(opcodes.clone(), Vec::new());
        let mut result1 = vm1.execute(&tree1, 0);
        assert!(result1.next().is_some(), "Should match tree with nsubj");

        // Test tree 2: with obj relation (different deprel)
        let mut tree2 = Tree::new();
        tree2.add_node(Node::new(0, "sees", "see", "VERB", "root"));
        tree2.add_node(Node::new(1, "cat", "cat", "NOUN", "obj"));
        tree2.set_parent(1, 0);

        let vm2 = VM::new(opcodes.clone(), Vec::new());
        let mut result2 = vm2.execute(&tree2, 0);
        assert!(result2.next().is_some(), "Should match tree with obj");

        // Test tree 3: with obl relation (yet another deprel)
        let mut tree3 = Tree::new();
        tree3.add_node(Node::new(0, "goes", "go", "VERB", "root"));
        tree3.add_node(Node::new(1, "store", "store", "NOUN", "obl"));
        tree3.set_parent(1, 0);

        let vm3 = VM::new(opcodes, Vec::new());
        let mut result3 = vm3.execute(&tree3, 0);
        assert!(result3.next().is_some(), "Should match tree with obl");
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
        let mut result = vm.execute(&tree, 0);

        let match_result = result.next().expect("Should have a match");
        assert_eq!(match_result.bindings[&0], 0); // help
        assert_eq!(match_result.bindings[&1], 1); // to
        assert_eq!(match_result.bindings[&2], 2); // write
    }

    #[test]
    #[should_panic(expected = "Compiler limitation: Cannot navigate back from child to parent")]
    fn test_anchor_with_parent_edge_and_child_edge_panics() {
        // This test detects a limitation: when anchor has both a parent edge
        // and a child edge, the compiler can't navigate back from the parent
        // Pattern: VERB (anchor) <- NOUN (parent), VERB -> ADV (child)
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::Lemma("run".to_string()), // High selectivity - will be anchor
        ));
        pattern.add_element(PatternElement::new(
            "noun",
            Constraint::POS("NOUN".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "adv",
            Constraint::POS("ADV".to_string()),
        ));

        // Edge 1: VERB has parent NOUN (Parent relation from verb to noun)
        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Parent,
            label: Some("nsubj".to_string()),
        });
        // Edge 2: VERB has child ADV (Child relation from verb to adv)
        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "adv".to_string(),
            relation: RelationType::Child,
            label: Some("advmod".to_string()),
        });

        // This will panic when trying to navigate back from parent to child
        compile_pattern(pattern);
    }

    #[test]
    #[should_panic(expected = "Compiler limitation: Cannot navigate back from child to parent")]
    fn test_anchor_with_multiple_parent_edges_panics() {
        // This test detects another case: anchor with multiple parent edges
        // Pattern: ADJ (parent1) <- NOUN (anchor) -> VERB (parent2)
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "noun",
            Constraint::Lemma("dog".to_string()), // High selectivity - will be anchor
        ));
        pattern.add_element(PatternElement::new(
            "adj",
            Constraint::POS("ADJ".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::POS("VERB".to_string()),
        ));

        // Edge 1: NOUN has parent ADJ
        pattern.add_edge(PatternEdge {
            from: "noun".to_string(),
            to: "adj".to_string(),
            relation: RelationType::Parent,
            label: None,
        });
        // Edge 2: NOUN has parent VERB (second parent edge)
        pattern.add_edge(PatternEdge {
            from: "noun".to_string(),
            to: "verb".to_string(),
            relation: RelationType::Parent,
            label: None,
        });

        // This will panic when trying to navigate back after first parent edge
        compile_pattern(pattern);
    }

    #[test]
    fn test_anchor_in_middle_follows_backward_edges() {
        // This test demonstrates the bug: when anchor is in the middle,
        // compiler must follow edges both forward AND backward
        use crate::tree::{Node, Tree};
        use crate::vm::VM;

        // Create tree: "dog" (NOUN) -[nsubj]-> "runs" (VERB) -[obj]-> "fast" (ADV)
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "fast", "fast", "ADV", "advmod"));
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);

        // Pattern: NOUN -[nsubj]-> VERB -[advmod]-> ADV
        // The verb has HIGH selectivity, so it will be chosen as anchor
        // Compiler must navigate BOTH to NOUN (backward) and to ADV (forward)
        let mut pattern = Pattern::new();
        pattern.add_element(PatternElement::new(
            "noun",
            Constraint::POS("NOUN".to_string()),
        ));
        pattern.add_element(PatternElement::new(
            "verb",
            Constraint::Lemma("run".to_string()), // High selectivity - will be anchor
        ));
        pattern.add_element(PatternElement::new(
            "adv",
            Constraint::POS("ADV".to_string()),
        ));

        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "noun".to_string(),
            relation: RelationType::Child,
            label: Some("nsubj".to_string()),
        });
        pattern.add_edge(PatternEdge {
            from: "verb".to_string(),
            to: "adv".to_string(),
            relation: RelationType::Child,
            label: Some("advmod".to_string()),
        });

        let (opcodes, anchor, _var_names) = compile_pattern(pattern);
        assert_eq!(anchor, 1); // Should anchor on "verb" (most selective)

        let vm = VM::new(opcodes, Vec::new());
        let mut result = vm.execute(&tree, 0);

        let match_result = result.next().expect("Should have a match");
        assert_eq!(match_result.bindings[&0], 1); // noun -> node 1
        assert_eq!(match_result.bindings[&1], 0); // verb (anchor) -> node 0
        assert_eq!(match_result.bindings[&2], 2); // adv -> node 2
    }
}
