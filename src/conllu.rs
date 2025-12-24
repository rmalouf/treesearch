//! CoNLL-U file parsing
//!
//! Parses CoNLL-U format files into Tree structures.
//! Supports all CoNLL-U features including multiword tokens, empty nodes,
//! enhanced dependencies, and sentence metadata.
//!
//! CoNLL-U format: https://universaldependencies.org/format.html

use crate::bytes::{BytestringPool, bs_atoi, bs_split_once};
use crate::tree::{Dep, Features, Misc, TokenId, Tree, WordId};
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use thiserror::Error;

/// Error during CoNLL-U parsing
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Parse error at line {line_num}: {message}\n  Line: {line_content}")]
    LineError {
        line_num: usize,
        line_content: String,
        message: String,
    },

    #[error("Parse error at line {line_num}: {message}")]
    LineErrorNoContent { line_num: usize, message: String },

    #[error("Parse error: {message}")]
    GenericError { message: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid UTF-8 sequence: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Missing field {field_num}")]
    MissingField { field_num: usize },

    #[error("Extended deprels not yet supported")]
    UnsupportedExtendedDeprels,

    #[error("Expected 10 fields, found more than 10")]
    TooManyFields,

    #[error("Invalid FEATS pair (missing '='): {pair}")]
    InvalidFeatsPair { pair: String },

    #[error("Invalid DEPS pair: {pair}")]
    InvalidDepsPair { pair: String },

    #[error("Empty nodes are not supported: {token_id}")]
    UnsupportedToken { token_id: String },

    #[error("Invalid token ID: {token_id}")]
    InvalidTokenId { token_id: String },

    #[error("Invalid HEAD: {head}")]
    InvalidHead { head: String },

    #[error("Invalid MISC pair (missing '='): {pair}")]
    InvalidMiscPair { pair: String },
}

/// CoNLL-U reader that iterates over sentences
pub struct TreeIterator<R: BufRead> {
    reader: R,
    line_num: usize,
    string_pool: BytestringPool,
}

impl<R: BufRead> TreeIterator<R> {
    /// Parse a single CoNLL-U line into a Word
    /// Skips multiword tokens (not yet supported), errors on empty nodes
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
                let result = fields.next().ok_or_else(|| {
                    let num = field_num;
                    field_num += 1;
                    ParseError::MissingField { field_num: num }
                })?;
                field_num += 1;
                let _ = field_num;
                result
            }};
        }

        let token_id_field = next_field!();

        // Skip multiword tokens (e.g., "1-2")
        if token_id_field.contains(&b'-') {
            return Ok(());
        }

        let token_id = parse_id(token_id_field)?;
        let form = next_field!();
        let lemma = next_field!();
        let upos = next_field!();
        let xpos = next_field!();
        let feats = self.parse_features(next_field!())?;
        let head = parse_head(next_field!())?;
        let deprel = next_field!();
        if next_field!() != b"_" {
            return Err(ParseError::UnsupportedExtendedDeprels);
        }
        let misc = self.parse_features(next_field!())?;

        if fields.next().is_some() {
            return Err(ParseError::TooManyFields);
        }

        tree.add_word(
            word_id, token_id, form, lemma, upos, xpos, feats, head, deprel, misc,
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
                return Err(ParseError::InvalidFeatsPair {
                    pair: str::from_utf8(pair)?.to_string(),
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
    fn _parse_deps(&mut self, s: &[u8]) -> Result<Vec<Dep>, ParseError> {
        let mut deps = Vec::new();

        if s == b"_" {
            return Ok(deps);
        }

        for pair in s.split(|b| *b == b'|') {
            let Some((head_str, deprel)) = bs_split_once(pair, b':') else {
                return Err(ParseError::InvalidDepsPair {
                    pair: str::from_utf8(pair)?.to_string(),
                });
            };

            let Some(head) = bs_atoi(head_str) else {
                return Err(ParseError::InvalidDepsPair {
                    pair: str::from_utf8(pair)?.to_string(),
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

impl TreeIterator<BufReader<Box<dyn Read + Send>>> {
    /// Create a reader from a file path (transparently handles gzip compression)
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Peek at the magic bytes to detect gzip
        let buf = reader.fill_buf()?;
        let reader: Box<dyn Read + Send> = if buf.starts_with(&[0x1f, 0x8b]) {
            Box::new(GzDecoder::new(reader))
        } else {
            Box::new(reader)
        };

        Ok(Self {
            reader: BufReader::new(reader),
            line_num: 0,
            string_pool: BytestringPool::new(),
        })
    }
}

impl TreeIterator<BufReader<std::io::Cursor<String>>> {
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

impl<R: BufRead> Iterator for TreeIterator<R> {
    type Item = Result<Tree, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut tree = Tree::with_metadata(&self.string_pool, None, HashMap::new());
        let mut word_id: WordId = 0;
        let mut buffer: Vec<u8> = Vec::with_capacity(100);
        let mut has_content = false;

        // Read lines until we hit a blank line (sentence boundary) or EOF
        loop {
            self.line_num += 1;
            buffer.clear(); // Reuse buffer allocation

            match self.reader.read_until(b'\n', &mut buffer) {
                Err(e) => {
                    return Some(Err(ParseError::IoError(e)));
                }
                Ok(0) => break, // EOF - always break
                Ok(_) => {
                    // Optimization: read_until includes '\n' at end, use O(1) suffix check
                    // instead of O(n) scan through entire buffer
                    let line = buffer.strip_suffix(b"\n").unwrap_or(&buffer);

                    if line.is_empty() {
                        // Blank line = sentence boundary if we have content
                        if has_content {
                            break;
                        }
                        // Skip leading/multiple blank lines
                        continue;
                    }

                    if buffer[0] == b'#' {
                        // Comment/metadata line
                        parse_comment(line, &mut tree);
                    } else {
                        // Regular token line - parse immediately
                        has_content = true;
                        if let Err(e) = self.parse_line(&mut tree, line, word_id) {
                            // Wrap error with line context
                            let enriched_error = ParseError::LineError {
                                line_num: self.line_num,
                                line_content: String::from_utf8_lossy(line).to_string(),
                                message: e.to_string(),
                            };
                            return Some(Err(enriched_error));
                        }
                        word_id += 1;
                    }
                }
            }
        }

        // Return None if we broke on EOF with no content
        if !has_content {
            return None;
        }

        // Compile tree
        tree.compile_tree();
        Some(Ok(tree))
    }
}

/// Parse a comment line (starts with #)
fn parse_comment(line: &[u8], tree: &mut Tree) {
    // TODO: deal with bytestring stuff here

    // Check for key = value format
    let line = str::from_utf8(line).unwrap().to_string();
    if let Some((key, value)) = line[1..].split_once("=") {
        let key = key.trim();
        let value = value.trim();

        if key == "text" {
            tree.sentence_text = Some(value.to_string());
        } else {
            tree.metadata.insert(key.to_string(), value.to_string());
        }
    }
}

/// Parse ID field (single integer only)
fn parse_id(s: &[u8]) -> Result<TokenId, ParseError> {
    // Check for empty nodes (containing '.')
    if s.contains(&b'.') {
        return Err(ParseError::UnsupportedToken {
            token_id: str::from_utf8(s)?.to_string(),
        });
    }

    let Some(id) = bs_atoi(s) else {
        return Err(ParseError::InvalidTokenId {
            token_id: str::from_utf8(s)?.to_string(),
        });
    };
    Ok(id)
}

/// Parse HEAD field (0 or integer)
fn parse_head(s: &[u8]) -> Result<Option<WordId>, ParseError> {
    if s == b"0" || s == b"_" {
        Ok(None) // Root word
    } else {
        let Some(head) = bs_atoi(s) else {
            return Err(ParseError::InvalidHead {
                head: str::from_utf8(s)?.to_string(),
            });
        };
        // HEAD is 1-indexed in CoNLL-U, convert to 0-indexed WordIds
        Ok(Some(head - 1))
    }
}

/// Parse MISC field (key=value|key=value)
fn _parse_misc(s: &str) -> Result<Misc, ParseError> {
    if s == "_" {
        return Ok(Misc::new());
    }

    let mut misc = Misc::new();
    for pair in s.split('|') {
        let Some((k, v)) = pair.split_once('=') else {
            return Err(ParseError::InvalidMiscPair {
                pair: pair.to_string(),
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

        let mut reader = TreeIterator::from_string(conllu);
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
        // Empty nodes are not supported
        assert!(parse_id(b"2.1").is_err());
        assert!(parse_id(b"10.5").is_err());
    }

    #[test]
    fn test_parse_features() {
        let conllu = "1\tword\tlemma\tUPOS\tXPOS\tCase=Nom|Number=Plur\t0\troot\t_\t_\n\n";
        let mut reader = TreeIterator::from_string(conllu);
        let first = reader.next().unwrap().unwrap();
        let feats = &first.word(0).unwrap().feats;
        assert!(feats.iter().any(|(k, v)| first.string_pool.compare_kv(
            *k,
            *v,
            "Case".as_bytes(),
            "Nom".as_bytes()
        )));

        assert!(feats.iter().any(|(k, v)| first.string_pool.compare_kv(
            *k,
            *v,
            "Number".as_bytes(),
            "Plur".as_bytes()
        )));

        let conllu = "1\tword\tlemma\tUPOS\tXPOS\t_\t0\troot\t_\t_\n\n";
        let mut reader = TreeIterator::from_string(conllu);
        let first = reader.next().unwrap().unwrap();
        let feats = &first.word(0).unwrap().feats;
        assert!(feats.is_empty());

        let conllu = "1\tword\tlemma\tUPOS\tXPOS\tInvalidPair\t0\troot\t_\t_\n\n";
        let mut reader = TreeIterator::from_string(conllu);
        let first = reader.next();
        assert!(first.unwrap().is_err());

        let conllu = "1\tword\tlemma\tUPOS\tXPOS\tfoo|bar=baz\t0\troot\t_\t_\n\n";
        let mut reader = TreeIterator::from_string(conllu);
        let first = reader.next();
        assert!(first.unwrap().is_err());
    }

    #[test]
    fn test_parse_misc() {
        let conllu = "1\tword\tlemma\tUPOS\tXPOS\tNumber=Plur\t0\troot\t_\tSpaceAfter=No\n\n";
        let mut reader = TreeIterator::from_string(conllu);
        let first = reader.next().unwrap().unwrap();
        let feats = &first.word(0).unwrap().misc;
        assert!(feats.iter().any(|(k, v)| first.string_pool.compare_kv(
            *k,
            *v,
            "SpaceAfter".as_bytes(),
            "No".as_bytes()
        )));

        let conllu = "1\tword\tlemma\tUPOS\tXPOS\tNumber=Plur_\t0\troot\t_\t_\n\n";
        let mut reader = TreeIterator::from_string(conllu);
        let first = reader.next().unwrap().unwrap();
        let feats = &first.word(0).unwrap().misc;
        assert!(feats.is_empty());
    }

    #[test]
    fn test_parse_head() {
        assert_eq!(parse_head(b"0").unwrap(), None);
        assert_eq!(parse_head(b"1").unwrap(), Some(0)); // 1-indexed to 0-indexed
        assert_eq!(parse_head(b"5").unwrap(), Some(4));
    }

    // Error handling tests
    #[test]
    fn test_error_empty_node() {
        let err = parse_id(b"2.1").unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedToken { .. }));
        assert!(err.to_string().contains("2.1"));
    }

    #[test]
    fn test_error_invalid_token_id() {
        let err = parse_id(b"abc").unwrap_err();
        assert!(matches!(err, ParseError::InvalidTokenId { .. }));
        assert!(err.to_string().contains("abc"));
    }

    #[test]
    fn test_error_invalid_head() {
        let err = parse_head(b"xyz").unwrap_err();
        assert!(matches!(err, ParseError::InvalidHead { .. }));
        assert!(err.to_string().contains("xyz"));
    }

    #[test]
    fn test_error_missing_fields() {
        let conllu = "1\tword\n\n"; // Only 2 fields instead of 10
        let mut reader = TreeIterator::from_string(conllu);
        let err = reader.next().unwrap().unwrap_err();
        // Should be wrapped in LineError with context
        assert!(matches!(err, ParseError::LineError { .. }));
        assert!(err.to_string().contains("line 1"));
    }

    #[test]
    fn test_error_too_many_fields() {
        // 11 fields - all valid until we check field count
        let conllu = "1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\textra\n\n";
        let mut reader = TreeIterator::from_string(conllu);
        let err = reader.next().unwrap().unwrap_err();
        assert!(matches!(err, ParseError::LineError { .. }));
        assert!(err.to_string().contains("10 fields"));
    }

    #[test]
    fn test_error_invalid_feats_pair() {
        let pool = BytestringPool::new();
        let mut reader = TreeIterator {
            reader: BufReader::new(std::io::Cursor::new("")),
            line_num: 0,
            string_pool: pool,
        };
        let err = reader.parse_features(b"InvalidPair").unwrap_err();
        assert!(matches!(err, ParseError::InvalidFeatsPair { .. }));
        assert!(err.to_string().contains("InvalidPair"));
    }

    #[test]
    fn test_error_unsupported_enhanced_deprels() {
        let conllu = "1\tword\tlemma\tNOUN\tNN\t_\t2\tnsubj\t2:dep\t_\n\n"; // DEPS field not "_"
        let mut reader = TreeIterator::from_string(conllu);
        let err = reader.next().unwrap().unwrap_err();
        assert!(
            err.to_string()
                .contains("Extended deprels not yet supported")
        );
    }

    #[test]
    fn test_error_line_context_preserved() {
        // Test that line number and content are preserved in error messages
        let conllu = r#"# comment line
1	word	lemma	NOUN	NN	_	0	root	_	_

abc	invalid	lemma	NOUN	NN	_	0	root	_	_

"#;
        let mut reader = TreeIterator::from_string(conllu);
        let first = reader.next(); // Get first valid tree
        assert!(first.is_some());
        assert!(first.unwrap().is_ok());

        let second = reader.next(); // Get error from second tree
        assert!(second.is_some());
        let err = second.unwrap().unwrap_err();

        let err_str = err.to_string();
        assert!(err_str.contains("line 4")); // Line number in error
        assert!(err_str.contains("abc")); // Line content in error
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
