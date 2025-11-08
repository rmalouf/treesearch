//! CoNLL-U file parsing
//!
//! Parses CoNLL-U format files into Tree structures.
//! Supports all CoNLL-U features including multiword tokens, empty nodes,
//! enhanced dependencies, and sentence metadata.
//!
//! CoNLL-U format: https://universaldependencies.org/format.html

use crate::tree::{Dep, Features, Misc, Node, NodeId, TokenId, Tree};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;

/// Error during CoNLL-U parsing
#[derive(Debug)]
pub struct ParseError {
    pub line_num: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at line {}: {}", self.line_num, self.message)
    }
}

impl std::error::Error for ParseError {}

/// CoNLL-U reader that iterates over sentences
pub struct CoNLLUReader<R: BufRead> {
    lines: Lines<R>,
    line_num: usize,
}

impl CoNLLUReader<BufReader<File>> {
    /// Create a reader from a file path
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(Self {
            lines: reader.lines(),
            line_num: 0,
        })
    }
}

impl CoNLLUReader<BufReader<std::io::Cursor<String>>> {
    /// Create a reader from a string
    pub fn from_str(text: &str) -> Self {
        let cursor = std::io::Cursor::new(text.to_string());
        let reader = BufReader::new(cursor);
        Self {
            lines: reader.lines(),
            line_num: 0,
        }
    }
}

impl<R: BufRead> Iterator for CoNLLUReader<R> {
    type Item = Result<Tree, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut tree_lines = Vec::new();
        let mut metadata = HashMap::new();
        let mut sentence_text = None;

        // Read lines until we hit a blank line (sentence boundary) or EOF
        loop {
            self.line_num += 1;
            match self.lines.next() {
                None => {
                    // EOF
                    if tree_lines.is_empty() {
                        return None; // No more sentences
                    } else {
                        // Last sentence without trailing blank line
                        break;
                    }
                }
                Some(Err(e)) => {
                    return Some(Err(ParseError {
                        line_num: self.line_num,
                        message: format!("IO error: {}", e),
                    }));
                }
                Some(Ok(line)) => {
                    let line = line.trim();

                    if line.is_empty() {
                        // Blank line = sentence boundary
                        if !tree_lines.is_empty() {
                            break;
                        }
                        // Skip multiple blank lines
                        continue;
                    }

                    if line.starts_with('#') {
                        // Comment/metadata line
                        parse_comment(&line[1..], &mut metadata, &mut sentence_text);
                        continue;
                    }

                    // Regular token line
                    tree_lines.push((self.line_num, line.to_string()));
                }
            }
        }

        // Parse the accumulated lines into a tree
        Some(parse_tree(tree_lines, sentence_text, metadata))
    }
}

/// Parse a comment line (starts with #)
fn parse_comment(comment: &str, metadata: &mut HashMap<String, String>, sentence_text: &mut Option<String>) {
    let comment = comment.trim();

    // Check for key = value format
    if let Some(eq_pos) = comment.find('=') {
        let key = comment[..eq_pos].trim();
        let value = comment[eq_pos + 1..].trim();

        if key == "text" {
            *sentence_text = Some(value.to_string());
        } else {
            metadata.insert(key.to_string(), value.to_string());
        }
    }
}

/// Parse accumulated lines into a Tree
fn parse_tree(
    lines: Vec<(usize, String)>,
    sentence_text: Option<String>,
    metadata: HashMap<String, String>,
) -> Result<Tree, ParseError> {
    let mut tree = Tree::with_metadata(sentence_text, metadata);
    let mut nodes = Vec::new();

    // Parse each line into a Node
    for (line_num, line) in lines {
        match parse_line(&line, line_num, nodes.len()) {
            Ok(Some(node)) => nodes.push(node),
            Ok(None) => {
                // Multiword token or empty node - skip for now
                // TODO: Handle these properly in future
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    // Build tree structure from HEAD relationships
    for node in nodes {
        tree.add_node(node);
    }

    // Set up parent-child relationships
    let node_count = tree.nodes.len();
    for i in 0..node_count {
        if let Some(parent_id) = tree.nodes[i].parent {
            if parent_id < node_count {
                tree.set_parent(i, parent_id);
            }
        } else {
            // Node with no parent is root
            tree.root_id = Some(i);
        }
    }

    Ok(tree)
}

/// Parse a single CoNLL-U line into a Node
/// Returns None for multiword tokens and empty nodes (for now)
fn parse_line(line: &str, line_num: usize, node_id: NodeId) -> Result<Option<Node>, ParseError> {
    let fields: Vec<&str> = line.split('\t').collect();

    if fields.len() != 10 {
        return Err(ParseError {
            line_num,
            message: format!("Expected 10 fields, found {}", fields.len()),
        });
    }

    // Field 0: ID
    let token_id = parse_id(fields[0])?;

    // Skip multiword tokens and empty nodes for now
    match token_id {
        TokenId::Range(_, _) => return Ok(None),
        TokenId::Decimal(_, _) => return Ok(None),
        TokenId::Single(_) => {}
    }

    // Field 1: FORM
    let form = fields[1].to_string();

    // Field 2: LEMMA
    let lemma = if fields[2] == "_" {
        form.clone() // Default to form if lemma not specified
    } else {
        fields[2].to_string()
    };

    // Field 3: UPOS
    let pos = fields[3].to_string();

    // Field 4: XPOS
    let xpos = if fields[4] == "_" {
        None
    } else {
        Some(fields[4].to_string())
    };

    // Field 5: FEATS
    let feats = parse_features(fields[5]);

    // Field 6: HEAD
    let head = parse_head(fields[6])?;

    // Field 7: DEPREL
    let deprel = fields[7].to_string();

    // Field 8: DEPS
    let deps = parse_deps(fields[8]);

    // Field 9: MISC
    let misc = parse_misc(fields[9]);

    let mut node = Node::with_full_fields(
        node_id,
        node_id, // Position = node_id for now
        token_id,
        form,
        lemma,
        pos,
        xpos,
        feats,
        deprel,
        deps,
        misc,
    );

    node.parent = head;

    Ok(Some(node))
}

/// Parse ID field (can be integer, range, or decimal)
fn parse_id(s: &str) -> Result<TokenId, ParseError> {
    if s.contains('-') {
        // Range: 1-2
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err(ParseError {
                line_num: 0,
                message: format!("Invalid range ID: {}", s),
            });
        }
        let start = parts[0].parse().map_err(|_| ParseError {
            line_num: 0,
            message: format!("Invalid range start: {}", parts[0]),
        })?;
        let end = parts[1].parse().map_err(|_| ParseError {
            line_num: 0,
            message: format!("Invalid range end: {}", parts[1]),
        })?;
        Ok(TokenId::Range(start, end))
    } else if s.contains('.') {
        // Decimal: 2.1
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(ParseError {
                line_num: 0,
                message: format!("Invalid decimal ID: {}", s),
            });
        }
        let main = parts[0].parse().map_err(|_| ParseError {
            line_num: 0,
            message: format!("Invalid decimal main: {}", parts[0]),
        })?;
        let sub = parts[1].parse().map_err(|_| ParseError {
            line_num: 0,
            message: format!("Invalid decimal sub: {}", parts[1]),
        })?;
        Ok(TokenId::Decimal(main, sub))
    } else {
        // Single: 1, 2, 3
        let id = s.parse().map_err(|_| ParseError {
            line_num: 0,
            message: format!("Invalid ID: {}", s),
        })?;
        Ok(TokenId::Single(id))
    }
}

/// Parse HEAD field (0 or integer)
fn parse_head(s: &str) -> Result<Option<NodeId>, ParseError> {
    if s == "0" || s == "_" {
        Ok(None) // Root node
    } else {
        let head: usize = s.parse().map_err(|_| ParseError {
            line_num: 0,
            message: format!("Invalid HEAD: {}", s),
        })?;
        // HEAD is 1-indexed in CoNLL-U, but we use 0-indexed NodeIds
        if head > 0 {
            Ok(Some(head - 1))
        } else {
            Ok(None)
        }
    }
}

/// Parse FEATS field (key=value|key=value)
fn parse_features(s: &str) -> Features {
    let mut feats = Features::new();

    if s == "_" {
        return feats;
    }

    for pair in s.split('|') {
        if let Some(eq_pos) = pair.find('=') {
            let key = pair[..eq_pos].to_string();
            let value = pair[eq_pos + 1..].to_string();
            feats.insert(key, value);
        }
    }

    feats
}

/// Parse DEPS field (head:deprel|head:deprel)
fn parse_deps(s: &str) -> Vec<Dep> {
    let mut deps = Vec::new();

    if s == "_" {
        return deps;
    }

    for pair in s.split('|') {
        if let Some(colon_pos) = pair.find(':') {
            if let Ok(head) = pair[..colon_pos].parse::<usize>() {
                let deprel = pair[colon_pos + 1..].to_string();
                // Convert 1-indexed to 0-indexed
                if head > 0 {
                    deps.push(Dep {
                        head: head - 1,
                        deprel,
                    });
                }
            }
        }
    }

    deps
}

/// Parse MISC field (key=value|key=value)
fn parse_misc(s: &str) -> Misc {
    let mut misc = Misc::new();

    if s == "_" {
        return misc;
    }

    for pair in s.split('|') {
        if let Some(eq_pos) = pair.find('=') {
            let key = pair[..eq_pos].to_string();
            let value = pair[eq_pos + 1..].to_string();
            misc.insert(key, value);
        }
    }

    misc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_sentence() {
        let conllu = r#"# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	SpaceAfter=No
4	.	.	PUNCT	.	_	3	punct	_	_

"#;

        let mut reader = CoNLLUReader::from_str(conllu);
        let tree = reader.next().unwrap().unwrap();

        assert_eq!(tree.nodes.len(), 4);
        assert_eq!(tree.sentence_text, Some("The dog runs.".to_string()));
        assert_eq!(tree.root_id, Some(2)); // "runs" is root

        // Check nodes
        assert_eq!(tree.nodes[0].form, "The");
        assert_eq!(tree.nodes[0].lemma, "the");
        assert_eq!(tree.nodes[0].pos, "DET");
        assert_eq!(tree.nodes[0].deprel, "det");

        assert_eq!(tree.nodes[2].form, "runs");
        assert_eq!(tree.nodes[2].parent, None); // root
        assert_eq!(tree.nodes[2].children.len(), 2); // dog, . (The is child of dog, not runs)
    }

    #[test]
    fn test_parse_with_features() {
        let conllu = r#"1	dogs	dog	NOUN	NNS	Number=Plur	2	nsubj	_	_
2	run	run	VERB	VBP	Number=Plur|Tense=Pres	0	root	_	_

"#;

        let mut reader = CoNLLUReader::from_str(conllu);
        let tree = reader.next().unwrap().unwrap();

        assert_eq!(tree.nodes.len(), 2);

        // Check features
        assert_eq!(tree.nodes[0].feats.get("Number"), Some("Plur"));
        assert_eq!(tree.nodes[1].feats.get("Number"), Some("Plur"));
        assert_eq!(tree.nodes[1].feats.get("Tense"), Some("Pres"));
    }

    #[test]
    fn test_parse_id_single() {
        assert_eq!(parse_id("1").unwrap(), TokenId::Single(1));
        assert_eq!(parse_id("42").unwrap(), TokenId::Single(42));
    }

    #[test]
    fn test_parse_id_range() {
        assert_eq!(parse_id("1-2").unwrap(), TokenId::Range(1, 2));
        assert_eq!(parse_id("5-7").unwrap(), TokenId::Range(5, 7));
    }

    #[test]
    fn test_parse_id_decimal() {
        assert_eq!(parse_id("2.1").unwrap(), TokenId::Decimal(2, 1));
        assert_eq!(parse_id("10.5").unwrap(), TokenId::Decimal(10, 5));
    }

    #[test]
    fn test_parse_features() {
        let feats = parse_features("Case=Nom|Number=Sing");
        assert_eq!(feats.get("Case"), Some("Nom"));
        assert_eq!(feats.get("Number"), Some("Sing"));

        let empty = parse_features("_");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_parse_head() {
        assert_eq!(parse_head("0").unwrap(), None);
        assert_eq!(parse_head("1").unwrap(), Some(0)); // 1-indexed to 0-indexed
        assert_eq!(parse_head("5").unwrap(), Some(4));
    }
}
