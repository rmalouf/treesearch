//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Solve CSP to find ALL matches (exhaustive search)
//! 3. Yield matches
//!
//! TODO: Implement CSP solver

use crate::parser::parse_query;
use crate::pattern::{Constraint, Pattern, VarId};
use crate::tree::Word;
use crate::tree::{Tree, WordId};
use std::collections::HashMap;

/// Error during search
#[derive(Debug)]
pub enum SearchError {
    ParseError(crate::parser::ParseError),
}

impl From<crate::parser::ParseError> for SearchError {
    fn from(e: crate::parser::ParseError) -> Self {
        SearchError::ParseError(e)
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for SearchError {}

/// A match is a binding from pattern variable IDs (VarId) to tree word IDs (WordId)
pub type Match = HashMap<VarId, WordId>;

/// Check if a tree word satisfies a pattern variable's constraint
fn satisfies_constraint(word: &Word, constraint: &Constraint) -> bool {
    match constraint {
        Constraint::Lemma(lemma) => word.lemma == *lemma,
        Constraint::POS(pos) => word.pos == *pos,
        Constraint::Form(form) => word.form == *form,
        Constraint::DepRel(deprel) => word.deprel == *deprel,
        Constraint::And(constraints) => constraints
            .iter()
            .all(|constraint| satisfies_constraint(word, constraint)),
        Constraint::Or(constraints) => constraints
            .iter()
            .any(|constraint| satisfies_constraint(word, constraint)),
        Constraint::Any => true, // No filtering
    }
}

/* WORKING VERSION - CSP solver sketch with correct terminology

/// Enumerate all matches
pub fn enumerate<'a>(&self, tree: &'a Tree, pattern: Pattern) -> Vec<Match>
{
    // Initial candidate domains (variable-word consistency)
    let mut domains: Vec<Vec<WordId>> = vec![Vec::new(); pattern.n_vars];
    for (var_id, var) in pattern.vars.iter().enumerate() {
        for (word_id, word) in tree.words.iter().enumerate() {
            // TODO: check required_parents and required_children
            if satisfies_constraint(word, &var.constraints) {
                domains[var_id].push(word_id);
            }
        }
        if domains[var_id].is_empty() {
            return Vec::new(); // no solution possible
        }
    }

    // DFS with forward-checking
    let mut assign: Vec<Option<WordId>> = vec![None; pattern.n_vars];
    self.dfs(tree, pattern, &mut assign, &domains)
}

fn dfs(&self,
          tree: &Tree,
          pattern: &Pattern,
          assign: &mut [Option<WordId>],
          domains: &[Vec<WordId>],
)
{
    // 1) Pick an unassigned variable with Minimum Remaining Values (MRV)
    let mut mrv_var: Option<VarId> = None;
    let mut best_rv = usize::MAX;
    for var_id in 0..pattern.n_vars {
        if assign[var_id].is_some() { continue; }
        let rv = domains[var_id].len();
        if rv < best_rv {
            best_rv = rv;
            mrv_var = Some(var_id);
        }
    }

    // No more variables to assign
    if mrv_var.is_none() {
        return vec![assign.clone()];
    }

    let next_var = mrv_var.unwrap();

    // 2) Try each candidate word for this variable
    'candidates: for &word_id in &domains[next_var] {

        // Check arc consistency with already-assigned neighbors (early prune)
        for &edge_idx in &pattern.out_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_idx];
            if let Some(target_word_id) = assign[edge_constraint.to] {
                if !edge_holds(tree, edge_constraint, word_id, target_word_id) {
                    continue 'candidates;
                }
            }
        }
        for &edge_idx in &pattern.in_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_idx];
            if let Some(source_word_id) = assign[edge_constraint.from] {
                if !edge_holds(tree, edge_constraint, source_word_id, word_id) {
                    continue 'candidates;
                }
            }
        }

        // 3) Create next state: assign var := word_id, mark used, forward-check neighbors
        let mut assign2 = assign.to_vec();
        assign2[next_var] = Some(word_id);

        let mut used2 = used.to_vec();
        used2[word_id] = true;

        // Forward-check: build pruned domains
        let mut dom2: Vec<Vec<WordId>> = domains.to_vec();
        // AllDiff: remove word_id from all other unassigned vars
        for var_id in 0..pattern.n_vars {
            if var_id == next_var || assign2[var_id].is_some() { continue; }
            dom2[var_id].retain(|&cand| cand != word_id);
            if dom2[var_id].is_empty() { continue 'candidates; }
        }

        // Propagate along edge constraints touching next_var
        // next_var as source
        for &edge_idx in &pattern.out_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_idx];
            let target_var = edge_constraint.to;
            if assign2[target_var].is_some() { continue; }
            dom2[target_var].retain(|&w| edge_holds(tree, edge_constraint, word_id, w) && !used2[w]);
            if dom2[target_var].is_empty() { continue 'candidates; }
        }
        // next_var as target
        for &edge_idx in &pattern.in_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_idx];
            let source_var = edge_constraint.from;
            if assign2[source_var].is_some() { continue; }
            dom2[source_var].retain(|&w| edge_holds(tree, edge_constraint, w, word_id) && !used2[w]);
            if dom2[source_var].is_empty() { continue 'candidates; }
        }

        // Recurse
        self.dfs(tree, pattern, &mut assign2, &dom2, &used2, on_solution);
    }
}


*/

/* ALTERNATIVE VERSION - with degree compatibility check

/// Enumerate all matches
pub fn enumerate<'a>(&self, tree: &'a Tree, pattern: Pattern) -> Vec<Match>
    {
        // Initial candidate domains (variable-word consistency)
        let mut domains: Vec<Vec<WordId>> = vec![Vec::new(); pattern.n_vars];
        for var_id in 0..pattern.n_vars {
            for (word_id, word) in tree.words.iter().enumerate() {
                if satisfies_constraint(word, &pattern.vars[var_id].constraints) {
                    if !Self::degree_compatible(&pattern, tree, var_id, word_id) { continue; }
                    domains[var_id].push(word_id);
                }
            }
            if domains[var_id].is_empty() {
                return Vec::new(); // no solution possible
            }
        }

        // DFS with forward-checking & AllDiff
        let mut assign: Vec<Option<WordId>> = vec![None; pattern.n_vars];
        let used = vec![false; tree.words.len()]; // AllDiff marker
        self.dfs(&pattern, tree, &mut assign, &domains, &used, &mut on_solution)
    }

    /// Check if word has the required parent/child deprels for this variable
    fn degree_compatible(pattern: &Pattern, tree: &Tree, var_id: VarId, word_id: WordId) -> bool {
        for deprel in &pattern.required_children[var_id] {
            if !tree.has_child_with_deprel(word_id, deprel) { return false; }
        }
        for deprel in &pattern.required_parents[var_id] {
            if !tree.has_parent_with_deprel(word_id, deprel) { return false; }
        }
        true
    }

    fn dfs<F>(&self,
              pattern: &Pattern,
              tree: &Tree,
              assign: &mut [Option<WordId>],
              domains: &[Vec<WordId>],
              used: &[bool],
              on_solution: &mut F)
    where
        F: FnMut(&[WordId]),
    {
        // 1) Pick an unassigned variable with MRV (smallest domain)
        let mut next_var: Option<VarId> = None;
        let mut best_size = usize::MAX;
        for var_id in 0..pattern.n_vars {
            if assign[var_id].is_some() { continue; }
            let sz = domains[var_id].len();
            if sz < best_size {
                best_size = sz;
                next_var = Some(var_id);
            }
        }

        // Done?
        if next_var.is_none() {
            // Collect solution
            let mut sol = vec![0usize; pattern.n_vars];
            for var_id in 0..pattern.n_vars {
                sol[var_id] = assign[var_id].expect("all assigned");
            }
            on_solution(&sol);
            return;
        }

        let var_id = next_var.unwrap();

        // 2) Try each candidate word for this variable
        'candidates: for &word_id in &domains[var_id] {
            if used[word_id] { continue; } // AllDiff

            // Check consistency with already-assigned neighbors (early prune)
            for &edge_idx in &pattern.out_edges[var_id] {
                let edge_constraint = &pattern.edge_constraints[edge_idx];
                if let Some(target_word_id) = assign[edge_constraint.to] {
                    if !edge_holds(tree, edge_constraint, word_id, target_word_id) {
                        continue 'candidates;
                    }
                }
            }
            for &edge_idx in &pattern.in_edges[var_id] {
                let edge_constraint = &pattern.edge_constraints[edge_idx];
                if let Some(source_word_id) = assign[edge_constraint.from] {
                    if !edge_holds(tree, edge_constraint, source_word_id, word_id) {
                        continue 'candidates;
                    }
                }
            }

            // 3) Create next state: assign var := word_id, mark used, forward-check neighbors
            let mut assign2 = assign.to_vec();
            assign2[var_id] = Some(word_id);

            let mut used2 = used.to_vec();
            used2[word_id] = true;

            // Forward-check: build pruned domains
            let mut dom2: Vec<Vec<WordId>> = domains.to_vec();
            // AllDiff: remove word_id from all other unassigned vars
            for other_var in 0..pattern.n_vars {
                if other_var == var_id || assign2[other_var].is_some() { continue; }
                dom2[other_var].retain(|&cand| cand != word_id);
                if dom2[other_var].is_empty() { continue 'candidates; }
            }

            // Propagate along edge constraints touching var_id
            // var_id as source
            for &edge_idx in &pattern.out_edges[var_id] {
                let edge_constraint = &pattern.edge_constraints[edge_idx];
                let target_var = edge_constraint.to;
                if assign2[target_var].is_some() { continue; }
                dom2[target_var].retain(|&w| edge_holds(tree, edge_constraint, word_id, w) && !used2[w]);
                if dom2[target_var].is_empty() { continue 'candidates; }
            }
            // var_id as target
            for &edge_idx in &pattern.in_edges[var_id] {
                let edge_constraint = &pattern.edge_constraints[edge_idx];
                let source_var = edge_constraint.from;
                if assign2[source_var].is_some() { continue; }
                dom2[source_var].retain(|&w| edge_holds(tree, edge_constraint, w, word_id) && !used2[w]);
                if dom2[source_var].is_empty() { continue 'candidates; }
            }

            // Recurse
            self.dfs(pattern, tree, &mut assign2, &dom2, &used2, on_solution);
        }
    }
}

/// Check if an edge constraint holds between source and target words in the tree
#[inline]
fn edge_holds(tree: &Tree, edge_constraint: &EdgeConstraint, source_word_id: WordId, target_word_id: WordId) -> bool {
    // TODO: implement based on RelationType and optional label
    // This is a placeholder - needs to check tree structure for the required relation
    true
}
*/

/// Search a tree with a pre-compiled pattern
///
/// Returns an iterator over all matches in the tree.
pub fn search<'a>(_tree: &'a Tree, _pattern: Pattern) -> impl Iterator<Item = Match> + 'a {
    // Placeholder - will be reimplemented as CSP solver
    std::iter::empty()
}

/// Search a tree with a query string
///
/// Parses the query and then searches the tree.
pub fn search_query<'a>(
    tree: &'a Tree,
    query: &str,
) -> Result<impl Iterator<Item = Match> + 'a, SearchError> {
    let pattern = parse_query(query)?;
    Ok(search(tree, pattern))
}

// Tests will be rewritten once CSP solver is implemented
#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
