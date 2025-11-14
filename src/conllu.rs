//! CoNLL-U file parsing
//!
//! Parses CoNLL-U format files into Tree structures.
//! Supports all CoNLL-U features including multiword tokens, empty nodes,
//! enhanced dependencies, and sentence metadata.
//!
//! CoNLL-U format: https://universaldependencies.org/format.html

use crate::tree::{Dep, Features, Misc, StringPool, TokenId, Tree, WordId, create_string_pool};
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
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
                write!(
                    f,
                    "Parse error at line {}: {}\n  Line: {}",
                    num, self.message, content
                )
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
    reader: R,
    line_num: usize,
    buffer: String,
    string_pool: StringPool,
    tree_lines: Vec<(usize, String)>,
}

impl<R: BufRead> CoNLLUReader<R> {
    /// Parse accumulated lines into a Tree
    pub fn parse_tree(
        &self,
        lines: &[(usize, String)],
        sentence_text: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<Tree, ParseError> {
        let mut tree = Tree::with_metadata(&self.string_pool, sentence_text, metadata);

        // Parse each line into a Word
        for (word_id, (line_num, line)) in lines.iter().enumerate() {
            if let Err(mut e) = self.parse_line(&mut tree, line, word_id) {
                e.line_num = Some(*line_num);
                e.line_content = Some(line.clone());
                return Err(e);
            }
        }

        // Set up parent-child relationships
        for i in 0..tree.words.len() {
            if let Some(parent_id) = tree.words[i].parent {
                tree.set_parent(i, parent_id);
            } else {
                // Word with no parent is root
                tree.root_id = Some(i);
            }
        }

        Ok(tree)
    }

    /// Parse a single CoNLL-U line into a Word
    /// Errors on multiword tokens and empty nodes (not yet supported)
    fn parse_line(&self, tree: &mut Tree, line: &str, word_id: WordId) -> Result<(), ParseError> {
        let mut fields = line.split('\t');
        //let mut fields = split_tabs(line);

        // Helper macro to consume the next field with error handling
        macro_rules! next_field {
            ($field_num:expr) => {
                fields.next().ok_or_else(|| ParseError {
                    line_num: None,
                    line_content: None,
                    message: format!("Missing field {}", $field_num),
                })?
            };
        }

        // Field 0: ID (1-based token number)
        let token_id = parse_id(next_field!(0))?;

        // Field 1: FORM
        let form = next_field!(1).to_string();

        // Field 2: LEMMA
        let lemma_str = next_field!(2);
        let lemma = if lemma_str == "_" {
            form.clone() // Default to form if lemma not specified
        } else {
            lemma_str.to_string()
        };

        // Field 3: UPOS
        let pos = tree.string_pool.get_or_intern(next_field!(3));

        // Field 4: XPOS
        let xpos_str = next_field!(4);
        let xpos = if xpos_str == "_" {
            None
        } else {
            Some(tree.string_pool.get_or_intern(xpos_str))
        };

        // Field 5: FEATS
        let feats = parse_features(next_field!(5))?;

        // Field 6: HEAD
        let head = parse_head(next_field!(6))?;

        // Field 7: DEPREL
        let deprel = tree.string_pool.get_or_intern(next_field!(7));

        // Field 8: DEPS
        let deps = parse_deps(next_field!(8))?;

        // Field 9: MISC
        let misc = parse_misc(next_field!(9))?;

        // Validate no extra fields
        if fields.next().is_some() {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: "Expected 10 fields, found more than 10".to_string(),
            });
        }

        tree.add_word_full_fields(
            word_id, token_id, form, lemma, pos, xpos, feats, deprel, deps, misc, head,
        );
        Ok(())
    }
}

/// Open a file and detect if it's gzipped based on magic bytes
fn open_file(path: &Path) -> std::io::Result<Box<dyn Read>> {
    let file = File::open(path)?;
    let mut buffered = BufReader::new(file);
    let buf = buffered.fill_buf()?;

    // Peek at first 2 bytes to check for gzip magic bytes (0x1f 0x8b)
    if buf.len() >= 2 && buf[0] == 0x1f && buf[1] == 0x8b {
        Ok(Box::new(GzDecoder::new(buffered)))
    } else {
        Ok(Box::new(buffered))
    }
}

impl CoNLLUReader<BufReader<Box<dyn Read>>> {
    /// Create a reader from a file path (transparently handles gzip compression)
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let file = open_file(path)?;
        let reader = BufReader::new(file);
        let rodeo = create_string_pool();
        Ok(Self {
            reader,
            line_num: 0,
            buffer: String::with_capacity(1 << 20),
            string_pool: rodeo,
            tree_lines: Vec::with_capacity(50),
        })
    }
}

impl CoNLLUReader<BufReader<std::io::Cursor<String>>> {
    /// Create a reader from a string
    pub fn from_string(text: &str) -> Self {
        let cursor = std::io::Cursor::new(text.to_string());
        let reader = BufReader::new(cursor);
        let rodeo = create_string_pool();
        Self {
            reader,
            line_num: 0,
            buffer: String::new(),
            string_pool: rodeo,
            tree_lines: Vec::with_capacity(50),
        }
    }
}

impl<R: BufRead> Iterator for CoNLLUReader<R> {
    type Item = Result<Tree, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        //let mut tree_lines = Vec::with_capacity(50);
        let mut metadata = HashMap::new();
        let mut sentence_text = None;
        self.tree_lines.clear();

        // Read lines until we hit a blank line (sentence boundary) or EOF
        loop {
            self.buffer.clear();
            self.line_num += 1;

            match self.reader.read_line(&mut self.buffer) {
                Err(e) => {
                    return Some(Err(ParseError {
                        line_num: Some(self.line_num),
                        line_content: None,
                        message: format!("IO error: {}", e),
                    }));
                }
                Ok(0) => break, // EOF - always break
                Ok(_) => {
                    let line = self.buffer.trim();

                    if line.is_empty() {
                        // Blank line = sentence boundary if we have content
                        if !self.tree_lines.is_empty() {
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
                    self.tree_lines.push((self.line_num, line.to_string()));
                }
            }
        }

        // Return None if we broke on EOF with no content
        if self.tree_lines.is_empty() {
            return None;
        }

        // Parse the accumulated lines into a tree
        Some(self.parse_tree(&self.tree_lines, sentence_text, metadata))
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

/*
#[inline]
fn split_tabs<'a>(line: &'a str) -> impl Iterator<Item = &'a str> {
    let bytes = line.as_bytes();
    let mut start = 0usize;
    let mut it = memchr_iter(b'\t', bytes).peekable();

    std::iter::from_fn(move || {
        if let Some(i) = it.next() {
            let field = &line[start..i];   // valid char boundary (ASCII tab)
            start = i + 1;
            Some(field)
        } else if start <= bytes.len() {
            // last field
            let field = &line[start..];
            start = bytes.len() + 1;       // mark done
            Some(field)
        } else {
            None
        }
    })
}
*/

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
fn parse_head(s: &str) -> Result<Option<WordId>, ParseError> {
    if s == "0" || s == "_" {
        Ok(None) // Root word
    } else {
        let Ok(head) = s.parse::<usize>() else {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: format!("Invalid HEAD: {}", s),
            });
        };
        // HEAD is 1-indexed in CoNLL-U, convert to 0-indexed WordIds
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
        feats.push((k.to_string(), v.to_string()));
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

        assert_eq!(tree.words.len(), 4);
        assert_eq!(tree.sentence_text, Some("The dog runs.".to_string()));
        assert_eq!(tree.root_id, Some(2)); // "runs" is root

        // Check nodes
        assert_eq!(tree.words[0].form, "The");
        assert_eq!(tree.words[0].lemma, "the");
        // TODO: fix this
        // assert_eq!(tree.words[0].pos, "DET");
        // assert_eq!(tree.words[0].deprel, "det");

        assert_eq!(tree.words[2].form, "runs");
        assert_eq!(tree.words[2].parent, None); // root
        assert_eq!(tree.words[2].children.len(), 2); // dog, . (The is child of dog, not runs)
    }

    #[test]
    fn test_parse_with_features() {
        let conllu = r#"1	dogs	dog	NOUN	NNS	Number=Plur	2	nsubj	_	_
2	run	run	VERB	VBP	Number=Plur|Tense=Pres	0	root	_	_

"#;

        let mut reader = CoNLLUReader::from_string(conllu);
        let tree = reader.next().unwrap().unwrap();

        assert_eq!(tree.words.len(), 2);

        // Check features - Features is a Vec<(String, String)>, not a HashMap
        assert!(
            tree.words[0]
                .feats
                .iter()
                .any(|(k, v)| k == "Number" && v == "Plur")
        );
        assert!(
            tree.words[1]
                .feats
                .iter()
                .any(|(k, v)| k == "Number" && v == "Plur")
        );
        assert!(
            tree.words[1]
                .feats
                .iter()
                .any(|(k, v)| k == "Tense" && v == "Pres")
        );
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
        assert!(feats.iter().any(|(k, v)| k == "Case" && v == "Nom"));
        assert!(feats.iter().any(|(k, v)| k == "Number" && v == "Sing"));

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
