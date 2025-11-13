//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Solve CSP to find ALL matches (exhaustive search)
//! 3. Yield matches
//!
//! TODO: Implement CSP solver

use crate::RelationType;
use crate::parser::parse_query;
use crate::pattern::{Constraint, Pattern};
use crate::tree::Word;
use crate::tree::{Tree, WordId};

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
pub type Match = Vec<WordId>;

/// Check if a tree word satisfies a pattern variable's constraint
fn satisfies_var_constraint(word: &Word, constraint: &Constraint) -> bool {
    match constraint {
        Constraint::Lemma(lemma) => word.lemma == *lemma,
        Constraint::POS(pos) => word.pos == *pos,
        Constraint::Form(form) => word.form == *form,
        Constraint::DepRel(deprel) => word.deprel == *deprel,
        Constraint::And(constraints) => constraints
            .iter()
            .all(|constraint| satisfies_var_constraint(word, constraint)),
        Constraint::Or(constraints) => constraints
            .iter()
            .any(|constraint| satisfies_var_constraint(word, constraint)),
        Constraint::Any => true, // No filtering
    }
}

fn satisfies_arc_constraint(
    tree: &Tree,
    from_word_id: WordId,
    to_word_id: WordId,
    relation: &RelationType,
) -> bool {
    match relation {
        RelationType::Child => tree.check_rel(from_word_id, to_word_id),
        RelationType::Parent => tree.check_rel(to_word_id, from_word_id),
        RelationType::Precedes => from_word_id < to_word_id,
        RelationType::Follows => to_word_id < from_word_id,
        _ => panic!("Unsupported relation"),
    }
}

/// Enumerate all matches
pub fn enumerate(tree: &Tree, pattern: &Pattern) -> Vec<Match> {
    // Initial candidate domains (node consistency)
    let mut domains: Vec<Vec<WordId>> = vec![Vec::new(); pattern.n_vars];
    for (var_id, var) in pattern.vars.iter().enumerate() {
        for (word_id, word) in tree.words.iter().enumerate() {
            if satisfies_var_constraint(word, &var.constraints) {
                domains[var_id].push(word_id);
            }
        }
        if domains[var_id].is_empty() {
            return Vec::new(); // no solution possible
        }
    }

    // DFS with forward-checking
    let mut assign: Vec<Option<WordId>> = vec![None; pattern.n_vars];
    dfs(tree, pattern, &mut assign, &domains)
}

fn dfs(
    tree: &Tree,
    pattern: &Pattern,
    assign: &mut [Option<WordId>],
    domains: &[Vec<WordId>]
) -> Vec<Match> {
    // No more variables to assign!
    if assign.iter().all(|word_id| word_id.is_some()) {
        let solution = assign.iter().map(|&opt| opt.unwrap()).collect();
        return vec![solution];
    }

    // 1) Pick an unassigned variable with Minimum Remaining Values (MRV)
    let next_var = (0..pattern.n_vars)
        .filter(|&var_id| assign[var_id].is_none())
        .min_by_key(|&var_id| domains[var_id].len())
        .expect("No MRV var found");

    let mut solutions: Vec<Match> = Vec::new();

    // 2) Try each candidate word for this variable
    'candidates: for &word_id in &domains[next_var] {
        // Check arc consistency with already-assigned neighbors (early prune)
        for &edge_id in &pattern.out_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_id];
            let target_var_id = pattern.var_names[&edge_constraint.to];
            if assign[target_var_id].is_some_and(|target_word_id|
                !satisfies_arc_constraint(tree, target_word_id, word_id, &edge_constraint.relation)
            ) {
                continue 'candidates;
            }
        }
        for &edge_id in &pattern.in_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_id];
            let source_var_id = pattern.var_names[&edge_constraint.from];
            if assign[source_var_id].is_some_and(|source_word_id|
                !satisfies_arc_constraint(tree, source_word_id, word_id, &edge_constraint.relation)
            ) {
                continue 'candidates;
            }
        }

        // 3) Create next state: assign var := word_id, mark used, forward-check neighbors
        let mut new_assign = assign.to_vec();
        new_assign[next_var] = Some(word_id);

        // Forward-check: build pruned domains
        // Propagate along edge constraints touching next_var
        let mut new_domains: Vec<Vec<WordId>> = domains.to_vec();

        for &edge_idx in &pattern.out_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_idx];
            let target_var_id = pattern.var_names[&edge_constraint.to];
            if new_assign[target_var_id].is_some() {
                continue;
            }
            new_domains[target_var_id].retain(|&w| {
                satisfies_arc_constraint(tree, word_id, w, &edge_constraint.relation)
            });
            if new_domains[target_var_id].is_empty() {
                continue 'candidates;
            }
        }

        for &edge_idx in &pattern.in_edges[next_var] {
            let edge_constraint = &pattern.edge_constraints[edge_idx];
            let source_var_id = pattern.var_names[&edge_constraint.from];
            if new_assign[source_var_id].is_some() {
                continue;
            }
            new_domains[source_var_id].retain(|&w| {
                satisfies_arc_constraint(tree, w, word_id, &edge_constraint.relation)
            });
            if new_domains[source_var_id].is_empty() {
                continue 'candidates;
            }
        }

        // Recurse
        solutions.extend(dfs(tree, pattern, &mut new_assign, &new_domains));
    }
    solutions
}

/// Search a tree with a pre-compiled pattern
///
/// Returns an iterator over all matches in the tree.
pub fn search(tree: &Tree, pattern: Pattern) -> impl Iterator<Item = Match> {
    // Placeholder - will be reimplemented as CSP solver
    enumerate(tree, &pattern).into_iter()
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
