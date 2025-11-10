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
    pub line_num: Option<usize>,
    pub line_content: Option<String>,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.line_num, &self.line_content) {
            (Some(num), Some(content)) => {
                write!(f, "Parse error at line {}: {}\n  Line: {}", num, self.message, content)
            }
            (Some(num), None) => {
                write!(f, "Parse error at line {}: {}", num, self.message)
            }
            (None, Some(content)) => {
                write!(f, "Parse error: {}\n  Line: {}", self.message, content)
            }
            (None, None) => {
                write!(f, "Parse error: {}", self.message)
            }
        }
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
    pub fn from_string(text: &str) -> Self {
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
                None => break, // EOF - always break
                Some(Err(e)) => {
                    return Some(Err(ParseError {
                        line_num: Some(self.line_num),
                        line_content: None,
                        message: format!("IO error: {}", e),
                    }));
                }
                Some(Ok(line)) => {
                    let line = line.trim();

                    if line.is_empty() {
                        // Blank line = sentence boundary if we have content
                        if !tree_lines.is_empty() {
                            break;
                        }
                        // Skip leading/multiple blank lines
                        continue;
                    }

                    if let Some(comment) = line.strip_prefix('#') {
                        // Comment/metadata line
                        parse_comment(comment, &mut metadata, &mut sentence_text);
                        continue;
                    }

                    // Regular token line
                    tree_lines.push((self.line_num, line.to_string()));
                }
            }
        }

        // Return None if we broke on EOF with no content
        if tree_lines.is_empty() {
            return None;
        }

        // Parse the accumulated lines into a tree
        Some(parse_tree(tree_lines, sentence_text, metadata))
    }
}

/// Parse a comment line (starts with #)
fn parse_comment(
    comment: &str,
    metadata: &mut HashMap<String, String>,
    sentence_text: &mut Option<String>,
) {
    let comment = comment.trim();

    // Check for key = value format
    if let Some((key, value)) = comment.split_once('=') {
        let key = key.trim();
        let value = value.trim();

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
        match parse_line(&line, nodes.len()) {
            Ok(node) => nodes.push(node),
            Err(mut e) => {
                e.line_num = Some(line_num);
                e.line_content = Some(line);
                return Err(e);
            }
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
/// Errors on multiword tokens and empty nodes (not yet supported)
fn parse_line(line: &str, node_id: NodeId) -> Result<Node, ParseError> {
    let fields: Vec<&str> = line.split('\t').collect();

    if fields.len() != 10 {
        return Err(ParseError {
            line_num: None,
            line_content: None,
            message: format!("Expected 10 fields, found {}", fields.len()),
        });
    }

    // Field 0: ID (1-based token number)
    let token_id = parse_id(fields[0])?;

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
    let feats = parse_features(fields[5])?;

    // Field 6: HEAD
    let head = parse_head(fields[6])?;

    // Field 7: DEPREL
    let deprel = fields[7].to_string();

    // Field 8: DEPS
    let deps = parse_deps(fields[8])?;

    // Field 9: MISC
    let misc = parse_misc(fields[9])?;

    let mut node = Node::with_full_fields(
        node_id, node_id, // Position = node_id for now
        token_id, form, lemma, pos, xpos, feats, deprel, deps, misc,
    );

    node.parent = head;

    Ok(node)
}

/// Parse ID field (single integer only)
fn parse_id(s: &str) -> Result<TokenId, ParseError> {
    if s.contains('-') {
        return Err(ParseError {
            line_num: None,
            line_content: None,
            message: format!("Multiword tokens (e.g., {}) are not supported", s),
        });
    }
    if s.contains('.') {
        return Err(ParseError {
            line_num: None,
            line_content: None,
            message: format!("Empty nodes (e.g., {}) are not supported", s),
        });
    }

    let Ok(id) = s.parse() else {
        return Err(ParseError {
            line_num: None,
            line_content: None,
            message: format!("Invalid token ID: {}", s),
        });
    };
    Ok(id)
}

/// Parse HEAD field (0 or integer)
fn parse_head(s: &str) -> Result<Option<NodeId>, ParseError> {
    if s == "0" || s == "_" {
        Ok(None) // Root node
    } else {
        let Ok(head) = s.parse::<usize>() else {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: format!("Invalid HEAD: {}", s),
            });
        };
        // HEAD is 1-indexed in CoNLL-U, convert to 0-indexed NodeIds
        Ok(Some(head - 1))
    }
}

/// Parse FEATS field (key=value|key=value)
fn parse_features(s: &str) -> Result<Features, ParseError> {
    if s == "_" {
        return Ok(Features::new());
    }

    let mut feats = Features::new();
    for pair in s.split('|') {
        let Some((k, v)) = pair.split_once('=') else {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: format!("Invalid FEATS pair (missing '='): {}", pair),
            });
        };
        feats.insert(k.to_string(), v.to_string());
    }
    Ok(feats)
}

/// Parse DEPS field (head:deprel|head:deprel)
fn parse_deps(s: &str) -> Result<Vec<Dep>, ParseError> {
    let mut deps = Vec::new();

    if s == "_" {
        return Ok(deps);
    }

    for pair in s.split('|') {
        let Some((head_str, deprel)) = pair.split_once(':') else {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: format!("Invalid DEPS pair: {}", pair),
            });
        };

        let Ok(head) = head_str.parse::<usize>() else {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: format!("Invalid DEPS pair: {}", pair),
            });
        };

        // Convert 1-indexed to 0-indexed; 0 means root (None)
        let head_id = if head == 0 { None } else { Some(head - 1) };
        deps.push(Dep {
            head: head_id,
            deprel: deprel.to_string(),
        });
    }

    Ok(deps)
}

/// Parse MISC field (key=value|key=value)
fn parse_misc(s: &str) -> Result<Misc, ParseError> {
    if s == "_" {
        return Ok(Misc::new());
    }

    let mut misc = Misc::new();
    for pair in s.split('|') {
        let Some((k, v)) = pair.split_once('=') else {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: format!("Invalid MISC pair (missing '='): {}", pair),
            });
        };
        misc.insert(k.to_string(), v.to_string());
    }
    Ok(misc)
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

        let mut reader = CoNLLUReader::from_string(conllu);
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

        let mut reader = CoNLLUReader::from_string(conllu);
        let tree = reader.next().unwrap().unwrap();

        assert_eq!(tree.nodes.len(), 2);

        // Check features
        assert_eq!(tree.nodes[0].feats.get("Number"), Some(&"Plur".to_string()));
        assert_eq!(tree.nodes[1].feats.get("Number"), Some(&"Plur".to_string()));
        assert_eq!(tree.nodes[1].feats.get("Tense"), Some(&"Pres".to_string()));
    }

    #[test]
    fn test_parse_id_single() {
        assert_eq!(parse_id("1").unwrap(), 1);
        assert_eq!(parse_id("42").unwrap(), 42);
    }

    #[test]
    fn test_parse_id_range() {
        // Multiword tokens are not supported
        assert!(parse_id("1-2").is_err());
        assert!(parse_id("5-7").is_err());
    }

    #[test]
    fn test_parse_id_decimal() {
        // Empty nodes are not supported
        assert!(parse_id("2.1").is_err());
        assert!(parse_id("10.5").is_err());
    }

    #[test]
    fn test_parse_features() {
        let feats = parse_features("Case=Nom|Number=Sing").unwrap();
        assert_eq!(feats.get("Case"), Some(&"Nom".to_string()));
        assert_eq!(feats.get("Number"), Some(&"Sing".to_string()));

        let empty = parse_features("_").unwrap();
        assert!(empty.is_empty());

        // Test error case
        assert!(parse_features("InvalidPair").is_err());
        assert!(parse_features("foo|bar=baz").is_err());
    }

    #[test]
    fn test_parse_head() {
        assert_eq!(parse_head("0").unwrap(), None);
        assert_eq!(parse_head("1").unwrap(), Some(0)); // 1-indexed to 0-indexed
        assert_eq!(parse_head("5").unwrap(), Some(4));
    }

    #[test]
    fn test_parse_deps() {
        let deps = parse_deps("2:nsubj|3:obj").unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].head, Some(1)); // 2 -> 1 (0-indexed)
        assert_eq!(deps[0].deprel, "nsubj");
        assert_eq!(deps[1].head, Some(2)); // 3 -> 2 (0-indexed)
        assert_eq!(deps[1].deprel, "obj");

        // Test root attachment
        let deps = parse_deps("0:root").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].head, None); // 0 -> None
        assert_eq!(deps[0].deprel, "root");

        let empty = parse_deps("_").unwrap();
        assert!(empty.is_empty());

        // Test error cases
        assert!(parse_deps("InvalidPair").is_err()); // Missing ':'
        assert!(parse_deps("foo:bar").is_err()); // Non-numeric head
        assert!(parse_deps("1:nsubj|invalid").is_err()); // One valid, one invalid
    }
}
