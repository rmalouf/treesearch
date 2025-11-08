//! Virtual machine for pattern matching
//!
//! This module implements the bytecode VM that executes compiled patterns
//! against dependency trees.

use crate::tree::{NodeId, Tree, Node};
use crate::pattern::Constraint;
use std::collections::{HashMap, VecDeque, HashSet};

/// VM instructions for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Constraint checking
    CheckLemma(String),
    CheckPOS(String),
    CheckForm(String),
    CheckDepRel(String),

    // Navigation
    MoveToParent,
    MoveToChild(Option<Constraint>),
    MoveLeft,
    MoveRight,

    // Wildcard search (BFS for shortest path)
    ScanDescendants(Constraint),
    ScanAncestors(Constraint),
    ScanSiblings(Constraint, bool), // Constraint, direction (true = right, false = left)

    // Control flow
    Bind(usize),           // Bind current node to pattern variable
    Choice,                // Create backtrack point
    Commit,                // Discard backtrack points
    Match,                 // Success - pattern matched
    Fail,                  // Trigger backtracking

    // State management
    PushState,             // Save current state
    RestoreState,          // Restore saved state
    Jump(isize),           // Jump to instruction offset
}

/// A choice point for backtracking
#[derive(Debug, Clone)]
struct ChoicePoint {
    /// Instruction pointer to resume at
    ip: usize,
    /// Saved bindings from when choice was created
    bindings: HashMap<usize, NodeId>,
    /// Alternative nodes to try (ordered by preference)
    alternatives: Vec<NodeId>,
}

/// VM execution state
#[derive(Debug)]
pub struct VMState {
    /// Current node being examined
    current_node: NodeId,
    /// Variable bindings (pattern position -> node ID)
    bindings: HashMap<usize, NodeId>,
    /// Instruction pointer
    ip: usize,
    /// Backtracking stack
    backtrack_stack: Vec<ChoicePoint>,
    /// State stack for push/restore operations
    state_stack: Vec<(NodeId, HashMap<usize, NodeId>)>,
}

impl VMState {
    fn new(start_node: NodeId) -> Self {
        Self {
            current_node: start_node,
            bindings: HashMap::new(),
            ip: 0,
            backtrack_stack: Vec::new(),
            state_stack: Vec::new(),
        }
    }
}

/// Result of pattern matching
#[derive(Debug, Clone)]
pub struct Match {
    /// Variable bindings (pattern position -> node ID)
    pub bindings: HashMap<usize, NodeId>,
}

/// The pattern matching virtual machine
pub struct VM {
    /// Compiled bytecode
    bytecode: Vec<Instruction>,
}

impl VM {
    /// Create a new VM with the given bytecode
    pub fn new(bytecode: Vec<Instruction>) -> Self {
        Self { bytecode }
    }

    /// Create a choice point with the given alternatives
    /// Alternatives should be ordered by preference (leftmost first)
    fn create_choice_point(
        state: &mut VMState,
        alternatives: Vec<NodeId>,
    ) {
        if alternatives.is_empty() {
            return;
        }

        let choice = ChoicePoint {
            ip: state.ip,
            bindings: state.bindings.clone(),
            alternatives,
        };
        state.backtrack_stack.push(choice);
    }

    /// Order nodes by their position (for leftmost semantics)
    /// Uses the position field from nodes to ensure correct linear ordering
    fn order_alternatives(nodes: Vec<NodeId>, tree: &Tree) -> Vec<NodeId> {
        let mut nodes_with_pos: Vec<(NodeId, usize)> = nodes
            .into_iter()
            .map(|id| {
                let node = tree.get_node(id)
                    .expect("VM bug: node in alternatives does not exist in tree");
                (id, node.position)
            })
            .collect();

        // Sort by position (leftmost first)
        nodes_with_pos.sort_by_key(|(_, pos)| *pos);

        // Extract node IDs
        nodes_with_pos.into_iter().map(|(id, _)| id).collect()
    }

    /// Execute the VM starting from the given node
    pub fn execute(&self, tree: &Tree, start_node: NodeId) -> Option<Match> {
        let mut state = VMState::new(start_node);

        loop {
            if state.ip >= self.bytecode.len() {
                return None; // Ran off end of program
            }

            let instruction = &self.bytecode[state.ip];

            match self.execute_instruction(instruction, &mut state, tree) {
                Ok(true) => {
                    // Match found - take ownership of bindings since we're returning
                    return Some(Match {
                        bindings: std::mem::take(&mut state.bindings),
                    });
                }
                Ok(false) => {
                    // Continue execution
                    state.ip += 1;
                }
                Err(_) => {
                    // Instruction failed, try backtracking
                    if !self.backtrack(&mut state) {
                        return None; // No more alternatives
                    }
                }
            }
        }
    }

    /// Check if a node matches a constraint
    fn check_constraint(node: &Node, constraint: &Constraint, tree: &Tree) -> bool {
        match constraint {
            Constraint::Any => true,
            Constraint::Lemma(lemma) => node.lemma == *lemma,
            Constraint::POS(pos) => node.pos == *pos,
            Constraint::Form(form) => node.form == *form,
            Constraint::DepRel(deprel) => node.deprel == *deprel,
            Constraint::And(constraints) => {
                constraints.iter().all(|c| Self::check_constraint(node, c, tree))
            }
            Constraint::Or(constraints) => {
                constraints.iter().any(|c| Self::check_constraint(node, c, tree))
            }
        }
    }

    /// Scan descendants using BFS to find shortest path matches
    /// Returns all nodes at the shortest depth that match the constraint
    fn scan_descendants(
        start_node: NodeId,
        constraint: &Constraint,
        tree: &Tree,
        max_depth: usize,
    ) -> Vec<NodeId> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut matches = Vec::new();
        let mut first_match_depth = None;

        // Start with children of the current node at depth 1
        if let Some(node) = tree.get_node(start_node) {
            for &child_id in &node.children {
                queue.push_back((child_id, 1));
            }
        }

        visited.insert(start_node);

        while let Some((node_id, depth)) = queue.pop_front() {
            // If we've found matches and we're at a deeper level, stop
            if let Some(match_depth) = first_match_depth {
                if depth > match_depth {
                    break;
                }
            }

            // Check depth limit
            if depth > max_depth {
                continue;
            }

            // Avoid cycles
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id);

            // Check if this node matches
            if let Some(node) = tree.get_node(node_id) {
                if Self::check_constraint(node, constraint, tree) {
                    matches.push(node_id);
                    if first_match_depth.is_none() {
                        first_match_depth = Some(depth);
                    }
                }

                // Add children to queue for next level
                // (only if we haven't found matches yet, or we're at the match depth)
                if first_match_depth.is_none() || first_match_depth == Some(depth) {
                    for &child_id in &node.children {
                        if !visited.contains(&child_id) {
                            queue.push_back((child_id, depth + 1));
                        }
                    }
                }
            }
        }

        matches
    }

    /// Scan ancestors to find matching nodes
    /// Walks up the parent chain and returns the first match
    /// (ancestors are naturally ordered by distance, so first match is closest)
    fn scan_ancestors(
        start_node: NodeId,
        constraint: &Constraint,
        tree: &Tree,
        max_depth: usize,
    ) -> Vec<NodeId> {
        let mut current_id = start_node;
        let mut depth = 0;

        // Walk up the parent chain
        while let Some(node) = tree.get_node(current_id) {
            if let Some(parent_id) = node.parent {
                depth += 1;

                if depth > max_depth {
                    break;
                }

                if let Some(parent_node) = tree.get_node(parent_id) {
                    if Self::check_constraint(parent_node, constraint, tree) {
                        // For ancestors, return only the first (closest) match
                        // No backtracking needed for ancestor search
                        return vec![parent_id];
                    }
                    current_id = parent_id;
                } else {
                    break;
                }
            } else {
                break; // Reached root
            }
        }

        Vec::new()
    }

    /// Scan siblings to find matching nodes
    /// Direction: true = right (forward), false = left (backward)
    /// Returns matches in order of proximity (closest first)
    fn scan_siblings(
        start_node: NodeId,
        constraint: &Constraint,
        tree: &Tree,
        direction: bool,
    ) -> Vec<NodeId> {
        // Get parent and find position among siblings
        let parent_id = match tree.get_node(start_node).and_then(|n| n.parent) {
            Some(id) => id,
            None => return Vec::new(),
        };
        let parent = match tree.get_node(parent_id) {
            Some(p) => p,
            None => return Vec::new(),
        };
        let start_pos = match parent.children.iter().position(|&id| id == start_node) {
            Some(p) => p,
            None => return Vec::new(),
        };

        let siblings = if direction {
            &parent.children[start_pos + 1..]
        } else {
            &parent.children[..start_pos]
        };

        let iter: Box<dyn Iterator<Item = &NodeId>> = if direction {
            Box::new(siblings.iter())
        } else {
            Box::new(siblings.iter().rev())
        };

        iter.filter(|&&id| Self::check_constraint(&tree.nodes[id], constraint, tree))
            .copied()
            .collect()
    }

    /// Execute a single instruction
    fn execute_instruction(
        &self,
        instruction: &Instruction,
        state: &mut VMState,
        tree: &Tree,
    ) -> Result<bool, ()> {
        match instruction {
            Instruction::Match => Ok(true),
            Instruction::Fail => Err(()),

            Instruction::Bind(pos) => {
                state.bindings.insert(*pos, state.current_node);
                Ok(false)
            }

            Instruction::CheckLemma(lemma) => {
                let node = tree.get_node(state.current_node)
                    .expect("VM bug: current_node does not exist");
                if node.lemma == *lemma {
                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::CheckPOS(pos) => {
                let node = tree.get_node(state.current_node)
                    .expect("VM bug: current_node does not exist");
                if node.pos == *pos {
                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::CheckForm(form) => {
                let node = tree.get_node(state.current_node)
                    .expect("VM bug: current_node does not exist");
                if node.form == *form {
                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::CheckDepRel(deprel) => {
                let node = tree.get_node(state.current_node)
                    .expect("VM bug: current_node does not exist");
                if node.deprel == *deprel {
                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::MoveToParent => {
                if let Some(parent) = tree.parent(state.current_node) {
                    state.current_node = parent.id;
                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::PushState => {
                state.state_stack.push((state.current_node, state.bindings.clone()));
                Ok(false)
            }

            Instruction::RestoreState => {
                if let Some((node, bindings)) = state.state_stack.pop() {
                    state.current_node = node;
                    state.bindings = bindings;
                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::MoveToChild(constraint_opt) => {
                if let Some(node) = tree.get_node(state.current_node) {
                    // Get all matching children
                    let matching_children: Vec<NodeId> = node.children.iter()
                        .filter_map(|&child_id| {
                            tree.get_node(child_id).and_then(|child| {
                                let matches = if let Some(constraint) = constraint_opt {
                                    Self::check_constraint(child, constraint, tree)
                                } else {
                                    true // No constraint means any child matches
                                };
                                if matches { Some(child_id) } else { None }
                            })
                        })
                        .collect();

                    if matching_children.is_empty() {
                        return Err(());
                    }

                    // Order by leftmost position
                    let ordered = Self::order_alternatives(matching_children, tree);

                    // Use first match
                    state.current_node = ordered[0];

                    // Create choice point if there are alternatives
                    if ordered.len() > 1 {
                        Self::create_choice_point(state, ordered[1..].to_vec());
                    }

                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::MoveLeft => {
                // Move to left sibling (previous sibling in parent's children list)
                if let Some(current) = tree.get_node(state.current_node) {
                    if let Some(parent_id) = current.parent {
                        if let Some(parent) = tree.get_node(parent_id) {
                            if let Some(pos) = parent.children.iter().position(|&id| id == state.current_node) {
                                if pos > 0 {
                                    state.current_node = parent.children[pos - 1];
                                    return Ok(false);
                                }
                            }
                        }
                    }
                }
                Err(())
            }

            Instruction::MoveRight => {
                // Move to right sibling (next sibling in parent's children list)
                if let Some(current) = tree.get_node(state.current_node) {
                    if let Some(parent_id) = current.parent {
                        if let Some(parent) = tree.get_node(parent_id) {
                            if let Some(pos) = parent.children.iter().position(|&id| id == state.current_node) {
                                if pos + 1 < parent.children.len() {
                                    state.current_node = parent.children[pos + 1];
                                    return Ok(false);
                                }
                            }
                        }
                    }
                }
                Err(())
            }

            Instruction::Jump(offset) => {
                // Offset can be negative (backwards jump) or positive (forwards jump)
                let new_ip = (state.ip as isize) + offset;
                if new_ip >= 0 && (new_ip as usize) < self.bytecode.len() {
                    state.ip = new_ip as usize;
                    Ok(false)
                } else {
                    Err(())
                }
            }

            Instruction::Choice => {
                // Choice creates a backtrack point with alternatives
                // For now, this is a placeholder - proper implementation needs alternatives
                // This will be fully implemented in Task 3
                Ok(false)
            }

            Instruction::Commit => {
                // Discard all choice points (cut operation)
                state.backtrack_stack.clear();
                Ok(false)
            }

            Instruction::ScanDescendants(constraint) => {
                const MAX_DEPTH: usize = 7; // Default depth limit
                let matches = Self::scan_descendants(state.current_node, constraint, tree, MAX_DEPTH);

                if matches.is_empty() {
                    return Err(());
                }

                // Order by leftmost position
                let ordered = Self::order_alternatives(matches, tree);

                // Use first match
                state.current_node = ordered[0];

                // Create choice point if there are alternatives
                if ordered.len() > 1 {
                    Self::create_choice_point(state, ordered[1..].to_vec());
                }

                Ok(false)
            }

            Instruction::ScanAncestors(constraint) => {
                const MAX_DEPTH: usize = 7; // Default depth limit
                let matches = Self::scan_ancestors(state.current_node, constraint, tree, MAX_DEPTH);

                if matches.is_empty() {
                    return Err(());
                }

                // Ancestors are already ordered by proximity, just use first
                state.current_node = matches[0];

                // Ancestors typically only return one match (closest)
                // but if we change that in the future, handle alternatives
                if matches.len() > 1 {
                    Self::create_choice_point(state, matches[1..].to_vec());
                }

                Ok(false)
            }

            Instruction::ScanSiblings(constraint, direction) => {
                let matches = Self::scan_siblings(state.current_node, constraint, tree, *direction);

                if matches.is_empty() {
                    return Err(());
                }

                // Siblings are already ordered by proximity
                state.current_node = matches[0];

                // Create choice point if there are alternatives
                if matches.len() > 1 {
                    Self::create_choice_point(state, matches[1..].to_vec());
                }

                Ok(false)
            }
        }
    }

    /// Attempt to backtrack to a previous choice point
    fn backtrack(&self, state: &mut VMState) -> bool {
        if let Some(mut choice) = state.backtrack_stack.pop() {
            if let Some(next_alternative) = choice.alternatives.pop() {
                // Try next alternative
                // Set IP to after the instruction that created the choice
                // (the main loop will execute from this point)
                state.ip = choice.ip + 1;
                state.current_node = next_alternative;
                state.bindings = choice.bindings.clone();

                // Put choice point back if more alternatives remain
                if !choice.alternatives.is_empty() {
                    state.backtrack_stack.push(choice);
                }

                true
            } else {
                // No more alternatives, try next choice point
                self.backtrack(state)
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Tree, Node};

    /// Helper to create a simple test tree:
    /// 0: runs (VERB, root)
    ///   ├─ 1: dog (NOUN, nsubj)
    ///   └─ 2: quickly (ADV, advmod)
    fn create_test_tree() -> Tree {
        let mut tree = Tree::new();
        let root = Node::new(0, "runs", "run", "VERB", "root");
        let child1 = Node::new(1, "dog", "dog", "NOUN", "nsubj");
        let child2 = Node::new(2, "quickly", "quickly", "ADV", "advmod");

        tree.add_node(root);
        tree.add_node(child1);
        tree.add_node(child2);
        tree.set_parent(1, 0);
        tree.set_parent(2, 0);

        tree
    }

    #[test]
    fn test_simple_match() {
        let mut tree = Tree::new();
        let root = Node::new(0, "runs", "run", "VERB", "root");
        tree.add_node(root);

        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
    }

    #[test]
    fn test_check_form() {
        let mut tree = Tree::new();
        let root = Node::new(0, "runs", "run", "VERB", "root");
        tree.add_node(root);

        let bytecode = vec![
            Instruction::CheckForm("runs".to_string()),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);
        assert!(result.is_some());

        // Test failure case
        let bytecode_fail = vec![
            Instruction::CheckForm("walked".to_string()),
            Instruction::Match,
        ];
        let vm_fail = VM::new(bytecode_fail);
        let result_fail = vm_fail.execute(&tree, 0);
        assert!(result_fail.is_none());
    }

    #[test]
    fn test_check_deprel() {
        let mut tree = Tree::new();
        let root = Node::new(0, "runs", "run", "VERB", "root");
        tree.add_node(root);

        let bytecode = vec![
            Instruction::CheckDepRel("root".to_string()),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);
        assert!(result.is_some());

        // Test failure case
        let bytecode_fail = vec![
            Instruction::CheckDepRel("nsubj".to_string()),
            Instruction::Match,
        ];
        let vm_fail = VM::new(bytecode_fail);
        let result_fail = vm_fail.execute(&tree, 0);
        assert!(result_fail.is_none());
    }

    #[test]
    fn test_move_child_no_constraint() {
        let tree = create_test_tree();

        // Move from root (0) to any child
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),  // At root
            Instruction::MoveToChild(None),                // Move to first child
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 1); // Should be at child 1 (dog)
    }

    #[test]
    fn test_move_child_with_constraint() {
        let tree = create_test_tree();

        // Move from root to child with POS=NOUN
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(Some(Constraint::POS("NOUN".to_string()))),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 1); // Should be at child 1 (dog/NOUN)
    }

    #[test]
    fn test_move_child_constraint_no_match() {
        let tree = create_test_tree();

        // Try to move to child with POS=PRON (doesn't exist)
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(Some(Constraint::POS("PRON".to_string()))),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_none()); // Should fail - no PRON child
    }

    #[test]
    fn test_move_parent() {
        let tree = create_test_tree();

        // Start at child, move to parent
        let bytecode = vec![
            Instruction::CheckPOS("NOUN".to_string()),  // At child 1 (dog)
            Instruction::MoveToParent,                     // Move to parent (runs)
            Instruction::CheckPOS("VERB".to_string()),   // Verify we're at parent
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 1); // Start at node 1 (dog)

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0); // Should be at parent (runs)
    }

    #[test]
    fn test_move_left_right() {
        let tree = create_test_tree();

        // Start at child 2, move left to child 1
        let bytecode = vec![
            Instruction::CheckPOS("ADV".to_string()),   // At child 2 (quickly)
            Instruction::MoveLeft,                       // Move to child 1 (dog)
            Instruction::CheckPOS("NOUN".to_string()),   // Verify
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 2); // Start at node 2 (quickly)

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 1);

        // Now test MoveRight: start at child 1, move right to child 2
        let bytecode2 = vec![
            Instruction::CheckPOS("NOUN".to_string()),  // At child 1 (dog)
            Instruction::MoveRight,                      // Move to child 2 (quickly)
            Instruction::CheckPOS("ADV".to_string()),    // Verify
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm2 = VM::new(bytecode2);
        let result2 = vm2.execute(&tree, 1); // Start at node 1 (dog)

        assert!(result2.is_some());
        let match_result2 = result2.unwrap();
        assert_eq!(match_result2.bindings[&0], 2);
    }

    #[test]
    fn test_move_left_at_boundary() {
        let tree = create_test_tree();

        // Try to move left from first child (should fail)
        let bytecode = vec![
            Instruction::CheckPOS("NOUN".to_string()),  // At child 1 (dog)
            Instruction::MoveLeft,                       // Try to move left (no left sibling)
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 1);

        assert!(result.is_none());
    }

    #[test]
    fn test_move_right_at_boundary() {
        let tree = create_test_tree();

        // Try to move right from last child (should fail)
        let bytecode = vec![
            Instruction::CheckPOS("ADV".to_string()),   // At child 2 (quickly)
            Instruction::MoveRight,                      // Try to move right (no right sibling)
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 2);

        assert!(result.is_none());
    }

    #[test]
    fn test_jump() {
        let mut tree = Tree::new();
        let root = Node::new(0, "runs", "run", "VERB", "root");
        tree.add_node(root);

        // Use Jump to skip over a failing instruction
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),  // 0: Check passes
            Instruction::Jump(2),                        // 1: Jump forward 2 (to instruction 3)
            Instruction::CheckForm("invalid".to_string()), // 2: Skipped (would fail)
            Instruction::Bind(0),                        // 3: Land here
            Instruction::Match,                          // 4: Success
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
    }

    #[test]
    fn test_commit() {
        let mut tree = Tree::new();
        let root = Node::new(0, "runs", "run", "VERB", "root");
        tree.add_node(root);

        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::Commit,  // Clear backtrack stack
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
    }

    #[test]
    fn test_push_restore_state() {
        let tree = create_test_tree();

        // Push state, move to child, then restore back to original
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),  // At root (0)
            Instruction::PushState,                      // Save state (at root)
            Instruction::MoveToChild(None),                // Move to child 1
            Instruction::CheckPOS("NOUN".to_string()),   // Verify at child
            Instruction::RestoreState,                   // Restore to root
            Instruction::CheckPOS("VERB".to_string()),   // Verify back at root
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0); // Should be back at root
    }

    #[test]
    fn test_compound_constraint_and() {
        let tree = create_test_tree();

        // Test And constraint: must be both NOUN and lemma "dog"
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(Some(Constraint::And(vec![
                Constraint::POS("NOUN".to_string()),
                Constraint::Lemma("dog".to_string()),
            ]))),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 1);
    }

    #[test]
    fn test_compound_constraint_or() {
        let tree = create_test_tree();

        // Test Or constraint: must be either NOUN or ADV (will match first child with NOUN)
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(Some(Constraint::Or(vec![
                Constraint::POS("PRON".to_string()),  // Doesn't match
                Constraint::POS("NOUN".to_string()),  // Matches child 1
            ]))),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 1);
    }

    /// Helper to create a deeper test tree for wildcard searches:
    /// 0: runs (VERB, root)
    ///   ├─ 1: dog (NOUN, nsubj)
    ///   │    └─ 3: big (ADJ, amod)
    ///   └─ 2: quickly (ADV, advmod)
    ///        └─ 4: very (ADV, advmod)
    ///             └─ 5: much (ADV, advmod)
    ///                  └─ 6: too (ADV, advmod)
    ///                       └─ 7: extremely (ADV, advmod)
    ///                            └─ 8: incredibly (ADV, advmod)
    fn create_deep_tree() -> Tree {
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "quickly", "quickly", "ADV", "advmod"));
        tree.add_node(Node::new(3, "big", "big", "ADJ", "amod"));
        tree.add_node(Node::new(4, "very", "very", "ADV", "advmod"));
        tree.add_node(Node::new(5, "much", "much", "ADV", "advmod"));
        tree.add_node(Node::new(6, "too", "too", "ADV", "advmod"));
        tree.add_node(Node::new(7, "extremely", "extremely", "ADV", "advmod"));
        tree.add_node(Node::new(8, "incredibly", "incredibly", "ADV", "advmod"));

        tree.set_parent(1, 0); // dog -> runs
        tree.set_parent(2, 0); // quickly -> runs
        tree.set_parent(3, 1); // big -> dog
        tree.set_parent(4, 2); // very -> quickly
        tree.set_parent(5, 4); // much -> very
        tree.set_parent(6, 5); // too -> much
        tree.set_parent(7, 6); // extremely -> too
        tree.set_parent(8, 7); // incredibly -> extremely

        tree
    }

    #[test]
    fn test_scan_descendants_shortest_path() {
        let tree = create_deep_tree();

        // From root, scan for ADJ - should find node 3 (big) at depth 2
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),  // At root
            Instruction::ScanDescendants(Constraint::POS("ADJ".to_string())),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 3); // Should find "big" (ADJ)
    }

    #[test]
    fn test_scan_descendants_depth_limit() {
        let tree = create_deep_tree();

        // From root, scan for node 8 (incredibly) which is at depth 6
        // With max_depth=7 it should be found
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::ScanDescendants(Constraint::Lemma("incredibly".to_string())),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 8);
    }

    #[test]
    fn test_scan_descendants_no_match() {
        let tree = create_deep_tree();

        // Try to find a PRON which doesn't exist
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::ScanDescendants(Constraint::POS("PRON".to_string())),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_none()); // Should fail
    }

    #[test]
    fn test_scan_descendants_bfs_order() {
        let tree = create_deep_tree();

        // From root, scan for ADV
        // Should find node 2 (quickly) at depth 1, not node 4+ which are deeper
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::ScanDescendants(Constraint::POS("ADV".to_string())),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 2); // Should find "quickly" at depth 1
    }

    #[test]
    fn test_scan_ancestors() {
        let tree = create_deep_tree();

        // From node 3 (big/ADJ), scan ancestors for VERB (should find root)
        let bytecode = vec![
            Instruction::CheckPOS("ADJ".to_string()),  // At node 3 (big)
            Instruction::ScanAncestors(Constraint::POS("VERB".to_string())),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 3); // Start at node 3

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0); // Should find root (runs/VERB)
    }

    #[test]
    fn test_scan_ancestors_closest_match() {
        let tree = create_deep_tree();

        // From node 8 (incredibly), scan ancestors for ADV
        // Should find node 7 (extremely), the closest ADV ancestor
        let bytecode = vec![
            Instruction::CheckLemma("incredibly".to_string()),  // At node 8
            Instruction::ScanAncestors(Constraint::POS("ADV".to_string())),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 8);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 7); // Should find "extremely" (closest ADV)
    }

    #[test]
    fn test_scan_ancestors_no_match() {
        let tree = create_deep_tree();

        // From node 3 (big/ADJ), scan ancestors for PRON (doesn't exist)
        let bytecode = vec![
            Instruction::CheckPOS("ADJ".to_string()),
            Instruction::ScanAncestors(Constraint::POS("PRON".to_string())),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 3);

        assert!(result.is_none());
    }

    #[test]
    fn test_scan_siblings_right() {
        let tree = create_test_tree();

        // From node 1 (dog/NOUN), scan right for ADV
        let bytecode = vec![
            Instruction::CheckPOS("NOUN".to_string()),  // At node 1 (dog)
            Instruction::ScanSiblings(Constraint::POS("ADV".to_string()), true), // Scan right
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 1);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 2); // Should find node 2 (quickly/ADV)
    }

    #[test]
    fn test_scan_siblings_left() {
        let tree = create_test_tree();

        // From node 2 (quickly/ADV), scan left for NOUN
        let bytecode = vec![
            Instruction::CheckPOS("ADV".to_string()),  // At node 2 (quickly)
            Instruction::ScanSiblings(Constraint::POS("NOUN".to_string()), false), // Scan left
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 2);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 1); // Should find node 1 (dog/NOUN)
    }

    #[test]
    fn test_scan_siblings_no_match() {
        let tree = create_test_tree();

        // From node 1 (dog), scan right for PRON (doesn't exist)
        let bytecode = vec![
            Instruction::CheckPOS("NOUN".to_string()),
            Instruction::ScanSiblings(Constraint::POS("PRON".to_string()), true),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 1);

        assert!(result.is_none());
    }

    #[test]
    fn test_scan_siblings_no_parent() {
        let tree = create_test_tree();

        // From root (no parent), scan siblings should fail
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),  // At root
            Instruction::ScanSiblings(Constraint::POS("NOUN".to_string()), true),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_none()); // Should fail - root has no siblings
    }

    #[test]
    fn test_wildcard_pattern_combination() {
        let tree = create_deep_tree();

        // Complex pattern: VERB ... ADJ (find ADJ descendant of VERB)
        // Then from that ADJ, find NOUN ancestor
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),  // At root (runs)
            Instruction::Bind(0),
            Instruction::ScanDescendants(Constraint::POS("ADJ".to_string())), // Find big
            Instruction::Bind(1),
            Instruction::ScanAncestors(Constraint::POS("NOUN".to_string())), // Find dog
            Instruction::Bind(2),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 0); // VERB = runs
        assert_eq!(match_result.bindings[&1], 3); // ADJ = big
        assert_eq!(match_result.bindings[&2], 1); // NOUN = dog
    }

    /// Helper to create a tree with multiple matching children for backtracking tests:
    /// 0: runs (VERB, root)
    ///   ├─ 1: the (DET, det)
    ///   ├─ 2: big (ADJ, amod)
    ///   ├─ 3: dog (NOUN, nsubj)
    ///   └─ 4: quickly (ADV, advmod)
    fn create_backtrack_tree() -> Tree {
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "the", "the", "DET", "det"));
        tree.add_node(Node::new(2, "big", "big", "ADJ", "amod"));
        tree.add_node(Node::new(3, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(4, "quickly", "quickly", "ADV", "advmod"));

        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 0);
        tree.set_parent(4, 0);

        tree
    }

    #[test]
    fn test_backtrack_with_multiple_children() {
        let tree = create_backtrack_tree();

        // Try to find a child that is a NOUN (not first child)
        // First child is DET, so it should try DET, fail the check, backtrack, and try NOUN
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),  // At root
            Instruction::MoveToChild(None),                // Move to first child (creates choice points)
            Instruction::CheckPOS("NOUN".to_string()),   // Check if it's a NOUN
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        // Should have backtracked through DET(1), ADJ(2), and found NOUN(3)
        assert_eq!(match_result.bindings[&0], 3);
    }

    #[test]
    fn test_backtrack_succeeds_on_second_alternative() {
        let tree = create_backtrack_tree();

        // Look for ADJ child (should be second alternative after DET)
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(None),
            Instruction::CheckPOS("ADJ".to_string()),  // Will fail on DET, succeed on ADJ
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 2); // Should find node 2 (big/ADJ)
    }

    #[test]
    fn test_backtrack_exhausts_all_alternatives_fails() {
        let tree = create_backtrack_tree();

        // Try to find a PRON child (doesn't exist)
        // Should try all children and fail
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(None),
            Instruction::CheckPOS("PRON".to_string()),  // Will fail on all children
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_none()); // Should fail after exhausting all alternatives
    }

    #[test]
    fn test_backtrack_with_constraint_creates_fewer_alternatives() {
        let tree = create_backtrack_tree();

        // Move to child with constraint - should only create choice points for matching children
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(Some(Constraint::POS("ADJ".to_string()))),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 2); // Should find ADJ directly
    }

    #[test]
    fn test_commit_prevents_backtracking() {
        let tree = create_backtrack_tree();

        // Move to first child, then commit (clearing backtrack stack)
        // Then try to match NOUN - should fail because we can't backtrack
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(None),                // Creates choice points
            Instruction::Commit,                         // Clear backtrack stack
            Instruction::CheckPOS("NOUN".to_string()),   // Will fail on DET (first child)
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_none()); // Should fail - can't backtrack after commit
    }

    #[test]
    fn test_nested_backtracking() {
        // Create a tree where we need to backtrack at multiple levels
        // 0: root (VERB)
        //   ├─ 1: child1 (NOUN)
        //   │    ├─ 3: grandchild1 (DET)
        //   │    └─ 4: grandchild2 (ADJ)
        //   └─ 2: child2 (NOUN)
        //        └─ 5: grandchild3 (ADJ)
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "root", "root", "VERB", "root"));
        tree.add_node(Node::new(1, "child1", "child1", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "child2", "child2", "NOUN", "obj"));
        tree.add_node(Node::new(3, "gc1", "gc1", "DET", "det"));
        tree.add_node(Node::new(4, "gc2", "gc2", "ADJ", "amod"));
        tree.add_node(Node::new(5, "gc3", "gc3", "ADJ", "amod"));

        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 1);
        tree.set_parent(4, 1);
        tree.set_parent(5, 2);

        // Pattern: VERB -> NOUN -> ADJ, but check that grandchild lemma is "gc3"
        // Should try: child1->gc1 (fail), child1->gc2 (fail on lemma), child2->gc5 (success)
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(Some(Constraint::POS("NOUN".to_string()))), // Try child1 first
            Instruction::MoveToChild(Some(Constraint::POS("ADJ".to_string()))),  // Try ADJ grandchild
            Instruction::CheckLemma("gc3".to_string()),                         // Check specific lemma
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 5); // Should find gc3 after backtracking
    }

    #[test]
    fn test_backtrack_with_scan_descendants() {
        // Create tree with multiple ADJ nodes at same depth
        // 0: runs (VERB)
        //   ├─ 1: dog (NOUN)
        //   │    ├─ 3: big (ADJ)
        //   │    └─ 4: small (ADJ)
        //   └─ 2: cat (NOUN)
        //        └─ 5: quick (ADJ)
        let mut tree = Tree::new();
        tree.add_node(Node::new(0, "runs", "run", "VERB", "root"));
        tree.add_node(Node::new(1, "dog", "dog", "NOUN", "nsubj"));
        tree.add_node(Node::new(2, "cat", "cat", "NOUN", "obj"));
        tree.add_node(Node::new(3, "big", "big", "ADJ", "amod"));
        tree.add_node(Node::new(4, "small", "small", "ADJ", "amod"));
        tree.add_node(Node::new(5, "quick", "quick", "ADJ", "amod"));

        tree.set_parent(1, 0);
        tree.set_parent(2, 0);
        tree.set_parent(3, 1);
        tree.set_parent(4, 1);
        tree.set_parent(5, 2);

        // Scan for ADJ descendants, but require specific lemma
        // Should try nodes in order: 3, 4, 5 (BFS, then leftmost at same depth)
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::ScanDescendants(Constraint::POS("ADJ".to_string())),
            Instruction::CheckLemma("quick".to_string()),  // Only matches node 5
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.bindings[&0], 5); // Should find "quick" after backtracking
    }

    #[test]
    fn test_leftmost_semantics_by_position() {
        // Test that leftmost semantics uses position field, not node ID
        // Create a tree where NodeIds (vector indices) don't match positions
        let mut tree = Tree::new();

        // Add root first - NodeId 0, position 2 (rightmost)
        let mut root = Node::new(0, "root", "root", "VERB", "root");
        root.position = 2;
        tree.add_node(root);

        // Add first NOUN - NodeId 1, position 1 (middle)
        let mut noun1 = Node::new(1, "second", "second", "NOUN", "nsubj");
        noun1.position = 1;
        tree.add_node(noun1);

        // Add second NOUN - NodeId 2, position 0 (leftmost!)
        let mut noun2 = Node::new(2, "first", "first", "NOUN", "nsubj");
        noun2.position = 0;
        tree.add_node(noun2);

        // Both NOUNs are children of root
        tree.set_parent(1, 0); // NodeId 1 (position 1) -> root
        tree.set_parent(2, 0); // NodeId 2 (position 0) -> root

        // Query: find VERB with NOUN child
        // Should match the LEFTMOST NOUN by position (NodeId 2, position 0)
        // NOT NodeId 1 (which has lower NodeId but higher position)
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::Bind(0),
            Instruction::MoveToChild(Some(Constraint::POS("NOUN".to_string()))),
            Instruction::Bind(1),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0).unwrap(); // Start at root (NodeId 0)

        assert_eq!(result.bindings[&0], 0); // VERB = NodeId 0 (root)
        assert_eq!(result.bindings[&1], 2); // NOUN = NodeId 2 (position 0, leftmost!)
    }

    #[test]
    fn test_leftmost_semantics() {
        let tree = create_backtrack_tree();

        // Multiple children match NOUN constraint - should get leftmost (lowest ID)
        // In our tree: children are 1(DET), 2(ADJ), 3(NOUN), 4(ADV)
        // Only node 3 is NOUN, so no backtracking needed, but test ordering works
        let bytecode = vec![
            Instruction::CheckPOS("VERB".to_string()),
            Instruction::MoveToChild(Some(Constraint::POS("NOUN".to_string()))),
            Instruction::Bind(0),
            Instruction::Match,
        ];

        let vm = VM::new(bytecode);
        let result = vm.execute(&tree, 0);

        assert!(result.is_some());
        assert_eq!(result.unwrap().bindings[&0], 3);
    }
}
