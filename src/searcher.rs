//! End-to-end tree search using constraint satisfaction
//!
//! The search pipeline:
//! 1. Parse query string into Pattern
//! 2. Solve CSP to find ALL matches (exhaustive search)
//! 3. Yield matches
//!
//! TODO: Implement CSP solver

use crate::parser::parse_query;
use crate::pattern::{Constraint, Pattern};
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

/// A match is a binding from pattern variable indices to tree word IDs
pub type Match = HashMap<usize, WordId>;

fn satisfies_node_constraint(word: &Word, constraint: &Constraint) -> bool {
    match constraint {
        Constraint::Lemma(lemma) => word.lemma == *lemma,
        Constraint::POS(pos) => word.pos == *pos,
        Constraint::Form(form) => word.form == *form,
        Constraint::DepRel(deprel) => word.deprel == *deprel,
        Constraint::And(constraints) => constraints
            .iter()
            .all(|constraint| satisfies_node_constraint(word, constraint)),
        Constraint::Or(constraints) => constraints
            .iter()
            .any(|constraint| satisfies_node_constraint(word, constraint)),
        Constraint::Any => true, // No filtering
    }
}

/* WORKING VERSION

/// Enumerate all matches
pub fn enumerate<'a>(&self, tree: &'a Tree, pattern: Pattern) -> Vec<Match>
{
    // Initial candidate domains (node consistency)
    let mut domains: Vec<Vec<usize>> = vec![Vec::new(); pattern.n_vars];
    for (node_id, node) in pattern.nodes.iter().enumerate() {
        for (word_id, word) in tree.words.iter().enumerate() {
            // TODO: check required_parents and required_children
            if satisfies_node_constraint(word, &node.constraints) {
                domains[node_id].push(word_id);
            }
        }
        if domains[node_id].is_empty() {
            return Vec::new(); // no solution possible
        }
    }

    // DFS with forward-checking
    let mut assign: Vec<Option<usize>> = vec![None; pattern.n_nodes];
    self.dfs(tree, pattern,&mut assign, &domains)
}

fn dfs(&self,
          tree: &Tree,
          pattern: &Pattern,
          assign: &mut [Option<usize>],
          domains: &[Vec<usize>],
)
{
    // 1) Pick an unassigned node with Minimum Remaining Values (MRV)
    let mut mrv_node: Option<usize> = None;
    let mut best_rv = usize::MAX;
    for node_id in 0..pattern.n_nodes {
        if assign[node_id].is_some() { continue; }
        let rv = domains[node_id].len();
        if rv < best_rv {
            best_rv = rv;
            mrv_node = Some(node_id);
        }
    }

    // No more nodes to assign
    if mrv_node.is_none() {
        return vec![assign.clone()];
    }

    let next_node = mrv_node.unwrap();

    // 2) Try each candidate value `word_id` for node `next_node`
    'candidates: for &word_id in &domains[next_node] {

        // Check arc consistency with already-assigned neighbors (early prune)
        for &e_id in &pattern.out_edge[next_node] {
            let e = pattern.edges[e_id];
            if let Some(n) = assign[e.to] {
                if !edge_holds(g, e, a, b) { continue 'candidates; }
            }
        }
        for &e_id in &pattern.in_edge[next_node] {
            let e = p.edges[e_id];
            if let Some(n) = assign[e.from] {
                if !edge_holds(g, e, b, a) { continue 'candidates; }
            }
        }

        // 3) Create next state: assign v:=a, mark used, and forward-check neighbors
        let mut assign2 = assign.to_vec();
        assign2[v] = Some(a);

        let mut used2 = used.to_vec();
        used2[a] = true;

        // Forward-check: build pruned domains'
        let mut dom2: Vec<Vec<usize>> = domains.to_vec();
        // AllDiff: remove `a` from all other unassigned vars
        for u in 0..p.k {
            if u == v || assign2[u].is_some() { continue; }
            dom2[u].retain(|&cand| cand != a);
            if dom2[u].is_empty() { continue 'candidates; }
        }

        // Propagate along pattern edges touching v
        // v as src
        for &ei in &p.out_by_var[v] {
            let e = p.edges[ei];
            let j = e.dst;
            if assign2[j].is_some() { continue; }
            dom2[j].retain(|&b| edge_holds(g, e, a, b) && !used2[b]);
            if dom2[j].is_empty() { continue 'candidates; }
        }
        // v as dst
        for &ei in &p.in_by_var[v] {
            let e = p.edges[ei];
            let i = e.src;
            if assign2[i].is_some() { continue; }
            dom2[i].retain(|&b| edge_holds(g, e, b, a) && !used2[b]);
            if dom2[i].is_empty() { continue 'candidates; }
        }

        // Recurse
        self.dfs(p, g, &mut assign2, &dom2, &used2, on_solution);
    }
}


*/

/* CHATGPT VERSION


/// Enumerate all matches
pub fn enumerate<'a>(&self, tree: &'a Tree, pattern: Pattern)
    {
        // Initial candidate domains (node consistency)
        let mut domains: Vec<Vec<usize>> = vec![Vec::new(); pattern.n_vars];
        for var_index in 0..pattern.n_vars {
            for (node_index, node) in tree.nodes.iter().enumerate() {
                if satisfies_node_constraint(node, )
                if !Self::degree_compatible(p, g, v, a) { continue; }
                domains[v].push(a);
            }
            if domains[v].is_empty() {
                return; // no solution possible
            }
        }

        // 2) DFS with forward-checking & AllDiff
        let mut assign: Vec<Option<usize>> = vec![None; p.k];
        let used = vec![false; g.n]; // AllDiff marker
        self.dfs(p, g, &mut assign, &domains, &used, &mut on_solution);
    }

    /// Necessary (cheap) filter: does node `a` have the required parent/child labels for var `v`?
    fn degree_compatible(p: &Pattern, g: &SentGraph, v: usize, a: usize) -> bool {
        for &r in &p.req_child[v]  { if !g.has_child_with_rel(a, r)  { return false; } }
        for &r in &p.req_parent[v] { if !g.has_parent_with_rel(a, r) { return false; } }
        true
    }

    fn dfs<F>(&self,
              p: &Pattern,
              g: &SentGraph,
              assign: &mut [Option<usize>],
              domains: &[Vec<usize>],
              used: &[bool],
              on_solution: &mut F)
    where
        F: FnMut(&[usize]),
    {
        // 1) Pick an unassigned var with MRV (smallest domain)
        let mut next_v: Option<usize> = None;
        let mut best_size = usize::MAX;
        for v in 0..p.k {
            if assign[v].is_some() { continue; }
            let sz = domains[v].len();
            if sz < best_size {
                best_size = sz;
                next_v = Some(v);
            }
        }

        // Done?
        if next_v.is_none() {
            // collect solution
            let mut sol = vec![0usize; p.k];
            for v in 0..p.k {
                sol[v] = assign[v].expect("all assigned");
            }
            on_solution(&sol);
            return;
        }

        let v = next_v.unwrap();

        // 2) Try each candidate value `a` for var `v`
        'candidates: for &a in &domains[v] {
            if used[a] { continue; } // AllDiff

            // Check consistency with already-assigned neighbors (early prune)
            for &ei in &p.out_by_var[v] {
                let e = p.edges[ei];
                if let Some(b) = assign[e.dst] {
                    if !edge_holds(g, e, a, b) { continue 'candidates; }
                }
            }
            for &ei in &p.in_by_var[v] {
                let e = p.edges[ei];
                if let Some(b) = assign[e.src] {
                    if !edge_holds(g, e, b, a) { continue 'candidates; }
                }
            }

            // 3) Create next state: assign v:=a, mark used, and forward-check neighbors
            let mut assign2 = assign.to_vec();
            assign2[v] = Some(a);

            let mut used2 = used.to_vec();
            used2[a] = true;

            // Forward-check: build pruned domains'
            let mut dom2: Vec<Vec<usize>> = domains.to_vec();
            // AllDiff: remove `a` from all other unassigned vars
            for u in 0..p.k {
                if u == v || assign2[u].is_some() { continue; }
                dom2[u].retain(|&cand| cand != a);
                if dom2[u].is_empty() { continue 'candidates; }
            }

            // Propagate along pattern edges touching v
            // v as src
            for &ei in &p.out_by_var[v] {
                let e = p.edges[ei];
                let j = e.dst;
                if assign2[j].is_some() { continue; }
                dom2[j].retain(|&b| edge_holds(g, e, a, b) && !used2[b]);
                if dom2[j].is_empty() { continue 'candidates; }
            }
            // v as dst
            for &ei in &p.in_by_var[v] {
                let e = p.edges[ei];
                let i = e.src;
                if assign2[i].is_some() { continue; }
                dom2[i].retain(|&b| edge_holds(g, e, b, a) && !used2[b]);
                if dom2[i].is_empty() { continue 'candidates; }
            }

            // Recurse
            self.dfs(p, g, &mut assign2, &dom2, &used2, on_solution);
        }
    }
}

/// Check if edge constraint holds between (src=a, dst=b).
#[inline]
fn edge_holds(g: &SentGraph, e: Edge, a: usize, b: usize) -> bool {
    match e.dir {
        Dir::Child  => g.children[a].iter().any(|&(c, r)| c == b && r == e.rel),
        Dir::Parent => g.parent[a].is_some_and(|(p, r)| p == b && r == e.rel),
    }
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
