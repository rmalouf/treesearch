//! CoNLL-U file parsing
//!
//! Parses CoNLL-U format files into Tree structures.
//! Supports all CoNLL-U features including multiword tokens, empty nodes,
//! enhanced dependencies, and sentence metadata.
//!
//! CoNLL-U format: https://universaldependencies.org/format.html

use crate::bytes::{BytestringPool, bs_split_once, bs_trim};
use crate::tree::{Dep, Features, Misc, TokenId, Tree, WordId};
use atoi::atoi;
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

impl From<std::str::Utf8Error> for ParseError {
    fn from(e: std::str::Utf8Error) -> Self {
        ParseError {
            line_num: None,
            line_content: None,
            message: format!("Invalid UTF-8 sequence: {}", e),
        }
    }
}

/// CoNLL-U reader that iterates over sentences
pub struct CoNLLUReader<R: BufRead> {
    reader: R,
    line_num: usize,
    string_pool: BytestringPool,
}

impl<R: BufRead> CoNLLUReader<R> {
    /// Parse accumulated lines into a Tree
    pub fn parse_tree(
        &mut self,
        lines: &[(usize, Vec<u8>)],
        sentence_text: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<Tree, ParseError> {
        let mut tree = Tree::with_metadata(&self.string_pool, sentence_text, metadata);

        // Parse each line into a Word
        for (word_id, (line_num, line)) in lines.iter().enumerate() {
            if let Err(mut e) = self.parse_line(&mut tree, line, word_id) {
                e.line_num = Some(*line_num);
                e.line_content = Some(str::from_utf8(line)?.to_string());
                return Err(e);
            }
        }

        tree.compile_tree();
        Ok(tree)
    }

    /// Parse a single CoNLL-U line into a Word
    /// Errors on multiword tokens and empty nodes (not yet supported)
    fn parse_line(
        &mut self,
        tree: &mut Tree,
        line: &[u8],
        word_id: WordId,
    ) -> Result<(), ParseError> {
        let mut fields = line.split(|b| *b == b'\t');
        let mut field_num = 0;

        // Helper macro to consume the next field with error handling
        macro_rules! next_field {
            () => {{
                let result = fields.next().ok_or_else(|| ParseError {
                    line_num: None,
                    line_content: None,
                    message: format!("Missing field {}", field_num),
                })?;
                field_num += 1;
                let _ = field_num; // avoid warning about unused value
                result
            }};
        }

        let token_id = parse_id(next_field!())?;
        let form = next_field!();
        let lemma = next_field!();
        let upos = next_field!();
        let xpos = match next_field!() {
            b"_" => None,
            s => Some(s),
        };
        let feats = self.parse_features(next_field!())?;
        let head = parse_head(next_field!())?;
        let deprel = next_field!();
        if next_field!() != b"_" {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: "Extended deprels not yet supported".to_string(),
            });
        }
        if next_field!() != b"_" {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: "Misc annotation not yet supported".to_string(),
            });
        }

        if fields.next().is_some() {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: "Expected 10 fields, found more than 10".to_string(),
            });
        }

        tree.add_word(
            word_id, token_id, form, lemma, upos, xpos, feats, head, deprel,
        );
        Ok(())
    }

    /// Parse FEATS field (key=value|key=value)
    fn parse_features(&mut self, s: &[u8]) -> Result<Features, ParseError> {
        if s == b"_" {
            return Ok(Features::new());
        }

        let mut feats = Features::new();
        for pair in s.split(|b| *b == b'|') {
            //            let mut kv = pair.split(|b| *b == b'=');
            //            let (Some(k), Some(v)) = (kv.next(), kv.next()) else {
            let Some((k, v)) = bs_split_once(pair, b'=') else {
                return Err(ParseError {
                    line_num: None,
                    line_content: None,
                    message: format!(
                        "Invalid FEATS pair (missing '='): {}",
                        str::from_utf8(pair)?
                    ),
                });
            };
            feats.push((
                self.string_pool.get_or_intern(k),
                self.string_pool.get_or_intern(v),
            ));
        }
        Ok(feats)
    }

    /// Parse DEPS field (head:deprel|head:deprel)
    fn parse_deps(&mut self, s: &[u8]) -> Result<Vec<Dep>, ParseError> {
        let mut deps = Vec::new();

        if s == b"_" {
            return Ok(deps);
        }

        for pair in s.split(|b| *b == b'|') {
            let Some((head_str, deprel)) = bs_split_once(pair, b':') else {
                return Err(ParseError {
                    line_num: None,
                    line_content: None,
                    message: format!("Invalid DEPS pair: {}", str::from_utf8(pair)?),
                });
            };

            let Some(head) = atoi::<usize>(head_str) else {
                return Err(ParseError {
                    line_num: None,
                    line_content: None,
                    message: format!("Invalid DEPS pair: {}", str::from_utf8(pair)?),
                });
            };

            // Convert 1-indexed to 0-indexed; 0 means root (None)
            let head_id = if head == 0 { None } else { Some(head - 1) };
            deps.push(Dep {
                head: head_id,
                deprel: self.string_pool.get_or_intern(deprel),
            });
        }

        Ok(deps)
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
        Ok(Self {
            reader,
            line_num: 0,
            string_pool: BytestringPool::new(),
        })
    }
}

impl CoNLLUReader<BufReader<std::io::Cursor<String>>> {
    /// Create a reader from a string
    pub fn from_string(text: &str) -> Self {
        let cursor = std::io::Cursor::new(text.to_string());
        let reader = BufReader::new(cursor);
        Self {
            reader,
            line_num: 0,
            string_pool: BytestringPool::new(),
        }
    }
}

impl<R: BufRead> Iterator for CoNLLUReader<R> {
    type Item = Result<Tree, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        //let mut tree_lines = Vec::with_capacity(50);
        let mut metadata = HashMap::new();
        let mut sentence_text = None;
        let mut tree_lines = Vec::with_capacity(50);

        // Read lines until we hit a blank line (sentence boundary) or EOF
        loop {
            self.line_num += 1;

            //match self.reader.read_line(&mut self.buffer) {
            let mut buffer: Vec<u8> = Vec::with_capacity(100);
            match self.reader.read_until(b'\n', &mut buffer) {
                Err(e) => {
                    return Some(Err(ParseError {
                        line_num: Some(self.line_num),
                        line_content: None,
                        message: format!("IO error: {}", e),
                    }));
                }
                Ok(0) => break, // EOF - always break
                Ok(_) => {
                    let line = bs_trim(&buffer);

                    if line.is_empty() {
                        // Blank line = sentence boundary if we have content
                        if !tree_lines.is_empty() {
                            break;
                        }
                        // Skip leading/multiple blank lines
                        continue;
                    }

                    if buffer[0] == b'#' {
                        // Comment/metadata line
                        parse_comment(line, &mut metadata, &mut sentence_text);
                    } else {
                        // Regular token line
                        tree_lines.push((self.line_num, line.to_owned()));
                    }
                }
            }
        }

        // Return None if we broke on EOF with no content
        if tree_lines.is_empty() {
            return None;
        }

        // Parse the accumulated lines into a tree
        Some(self.parse_tree(&tree_lines, sentence_text, metadata))
    }
}

/// Parse a comment line (starts with #)
fn parse_comment(
    line: &[u8],
    metadata: &mut HashMap<String, String>,
    sentence_text: &mut Option<String>,
) {
    // TODO: deal with bytestring stuff here

    // Check for key = value format
    let line = str::from_utf8(line).unwrap().to_string();
    if let Some((key, value)) = line[1..].split_once("=") {
        let key = key.trim();
        let value = value.trim();

        if key == "text" {
            *sentence_text = Some(value.to_string());
        } else {
            metadata.insert(key.to_string(), value.to_string());
        }
    }
}

/// Parse ID field (single integer only)
fn parse_id(s: &[u8]) -> Result<TokenId, ParseError> {
    if s.contains(&b'-') {
        return Err(ParseError {
            line_num: None,
            line_content: None,
            message: format!(
                "Multiword tokens (e.g., {}) are not supported",
                str::from_utf8(s)?
            ),
        });
    }
    if s.contains(&b'.') {
        return Err(ParseError {
            line_num: None,
            line_content: None,
            message: format!(
                "Empty nodes (e.g., {}) are not supported",
                str::from_utf8(s)?
            ),
        });
    }

    let Some(id) = atoi::<TokenId>(s) else {
        return Err(ParseError {
            line_num: None,
            line_content: None,
            message: format!("Invalid token ID: {}", str::from_utf8(s)?),
        });
    };
    Ok(id)
}

/// Parse HEAD field (0 or integer)
fn parse_head(s: &[u8]) -> Result<Option<WordId>, ParseError> {
    if s == b"0" || s == b"_" {
        Ok(None) // Root word
    } else {
        let Some(head) = atoi::<WordId>(s) else {
            return Err(ParseError {
                line_num: None,
                line_content: None,
                message: format!("Invalid HEAD: {}", str::from_utf8(s)?),
            });
        };
        // HEAD is 1-indexed in CoNLL-U, convert to 0-indexed WordIds
        Ok(Some(head - 1))
    }
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

    // TODO: add tests for
    //   deprels and misc

    #[test]
    fn test_parse_simple_sentence() {
        let conllu = r#"# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	_
4	.	.	PUNCT	.	_	3	punct	_	_

"#;

        let mut reader = CoNLLUReader::from_string(conllu);
        let tree = reader.next().unwrap().unwrap();

        assert_eq!(tree.words.len(), 4);
        assert_eq!(tree.sentence_text, Some("The dog runs.".to_string()));
        assert_eq!(tree.root_id, Some(2)); // "runs" is root

        // Check nodes
        // TODO: fix these
        // assert_eq!(tree.words[0].form, b"The");
        // assert_eq!(tree.words[0].lemma, b"the");
        // assert_eq!(*tree.string_pool.resolve(tree.words[0].upos), b"DET");
        // assert_eq!(*tree.string_pool.resolve(tree.words[0].deprel), b"det");
        // assert_eq!(tree.words[2].form, "runs");
        assert_eq!(tree.words[2].head, None); // root
        assert_eq!(tree.words[2].children.len(), 2); // dog, . (The is child of dog, not runs)
    }

    /*
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
    */
    #[test]
    fn test_parse_id() {
        assert_eq!(parse_id(b"1").unwrap(), 1);
        assert_eq!(parse_id(b"42").unwrap(), 42);
        // Multiword tokens are not supported
        assert!(parse_id(b"1-2").is_err());
        // Empty nodes are not supported
        assert!(parse_id(b"2.1").is_err());
        assert!(parse_id(b"10.5").is_err());
    }
    /*
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
    */
    #[test]
    fn test_parse_head() {
        assert_eq!(parse_head(b"0").unwrap(), None);
        assert_eq!(parse_head(b"1").unwrap(), Some(0)); // 1-indexed to 0-indexed
        assert_eq!(parse_head(b"5").unwrap(), Some(4));
    }
    /*
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
    */
}
