//! Virtual machine for pattern matching
//!
//! This module implements the bytecode VM that executes compiled patterns
//! against dependency trees.

use crate::tree::{Node, NodeId, Tree};
use crate::pattern::Constraint;
use std::collections::HashMap;

/// VM instructions for pattern matching
#[derive(Debug, Clone)]
pub enum Instruction {
    // Constraint checking
    CheckLemma(String),
    CheckPOS(String),
    CheckForm(String),
    CheckDepRel(String),

    // Navigation
    MoveParent,
    MoveChild(Option<Constraint>),
    MoveLeft,
    MoveRight,

    // Wildcard search (BFS for shortest path)
    ScanDescendants(Constraint),
    ScanAncestors(Constraint),
    ScanSiblings(Constraint),

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
    ip: usize,
    node_id: NodeId,
    bindings: HashMap<usize, NodeId>,
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
                    // Match found
                    return Some(Match {
                        bindings: state.bindings.clone(),
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
                if let Some(node) = tree.get_node(state.current_node) {
                    if node.lemma == *lemma {
                        Ok(false)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }

            Instruction::CheckPOS(pos) => {
                if let Some(node) = tree.get_node(state.current_node) {
                    if node.pos == *pos {
                        Ok(false)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }

            Instruction::MoveParent => {
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

            // Placeholder implementations for other instructions
            _ => {
                // TODO: Implement remaining instructions
                Err(())
            }
        }
    }

    /// Attempt to backtrack to a previous choice point
    fn backtrack(&self, state: &mut VMState) -> bool {
        if let Some(mut choice) = state.backtrack_stack.pop() {
            if let Some(next_alternative) = choice.alternatives.pop() {
                // Try next alternative
                state.ip = choice.ip;
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
    use crate::tree::Tree;

    #[test]
    fn test_simple_match() {
        let mut tree = Tree::new();
        let root = crate::tree::Node::new(0, "runs", "run", "VERB", "root");
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
}
