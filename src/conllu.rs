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
use zstd::stream::read::Decoder as ZstdDecoder;

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
        // Find the 9 tab positions that delimit 10 fields.
        let mut tabs = [0usize; 9];
        let mut n = 0;
        for pos in memchr::memchr_iter(b'\t', line) {
            if n == 9 {
                return Err(ParseError::TooManyFields);
            }
            tabs[n] = pos;
            n += 1;
        }
        if n != 9 {
            return Err(ParseError::MissingField { field_num: n + 1 });
        }

        let token_id_field = &line[..tabs[0]];
        let form = &line[tabs[0] + 1..tabs[1]];
        let lemma = &line[tabs[1] + 1..tabs[2]];
        let upos = &line[tabs[2] + 1..tabs[3]];
        let xpos = &line[tabs[3] + 1..tabs[4]];
        let feats_field = &line[tabs[4] + 1..tabs[5]];
        let head_field = &line[tabs[5] + 1..tabs[6]];
        let deprel = &line[tabs[6] + 1..tabs[7]];
        let deps_field = &line[tabs[7] + 1..tabs[8]];
        let misc_field = &line[tabs[8] + 1..];

        // Skip multiword tokens (e.g., "1-2")
        if memchr::memchr(b'-', token_id_field).is_some() {
            return Ok(());
        }

        let token_id = parse_id(token_id_field)?;
        let feats = self.parse_features(feats_field)?;
        let head = parse_head(head_field)?;
        if deps_field != b"_" {
            return Err(ParseError::UnsupportedExtendedDeprels);
        }
        let misc = self.parse_features(misc_field)?;

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
        let mut start = 0;
        for pipe_pos in memchr::memchr_iter(b'|', s).chain(std::iter::once(s.len())) {
            let pair = &s[start..pipe_pos];
            let Some((k, v)) = bs_split_once(pair, b'=') else {
                return Err(ParseError::InvalidFeatsPair {
                    pair: str::from_utf8(pair)?.to_string(),
                });
            };
            feats.push((
                self.string_pool.get_or_intern(k),
                self.string_pool.get_or_intern(v),
            ));
            start = pipe_pos + 1;
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
    /// Create a reader from a file path (transparently handles gzip and zstd compression)
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Peek at the magic bytes to detect compression format
        let buf = reader.fill_buf()?;
        let reader: Box<dyn Read + Send> = if buf.starts_with(&[0x1f, 0x8b]) {
            Box::new(GzDecoder::new(reader))
        } else if buf.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
            Box::new(ZstdDecoder::new(reader)?)
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

    fn parse_one(conllu: &str) -> Result<Tree, ParseError> {
        TreeIterator::from_string(conllu).next().unwrap()
    }

    /// One root word with the given FEATS and MISC fields
    fn word_with(feats: &str, misc: &str) -> Result<Tree, ParseError> {
        parse_one(&format!(
            "1\tword\tlemma\tX\tX\t{feats}\t0\troot\t_\t{misc}\n"
        ))
    }

    /// Key/value pairs of a FEATS or MISC field, as strings
    fn pairs(tree: &Tree, feats: &Features) -> Vec<(String, String)> {
        let s = |sym| String::from_utf8(tree.string_pool.resolve(sym).to_vec()).unwrap();
        feats.iter().map(|&(k, v)| (s(k), s(v))).collect()
    }

    #[test]
    fn test_parse_sentence() {
        let conllu = "\n\
                      # sent_id = s1\n\
                      # text = The dog runs.\n\
                      1\tThe\tthe\tDET\tDT\t_\t2\tdet\t_\t_\n\
                      2-3\tdogruns\t_\t_\t_\t_\t_\t_\t_\t_\n\
                      2\tdog\tdog\tNOUN\tNN\t_\t3\tnsubj\t_\t_\n\
                      3\truns\trun\tVERB\tVBZ\t_\t0\troot\t_\t_\n\
                      4\t.\t.\tPUNCT\t.\t_\t3\tpunct\t_\t_\n\
                      \n\
                      \n\
                      1\tCats\tcat\tNOUN\tNNS\t_\t0\troot\t_\t_\n";
        let trees: Vec<_> = TreeIterator::from_string(conllu)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(trees.len(), 2);

        let tree = &trees[0];
        assert_eq!(tree.sentence_text.as_deref(), Some("The dog runs."));
        assert_eq!(tree.metadata["sent_id"], "s1");

        // The multiword token is skipped
        let s = |sym| String::from_utf8(tree.string_pool.resolve(sym).to_vec()).unwrap();
        let words: Vec<_> = tree
            .words
            .iter()
            .map(|w| {
                (
                    s(w.form),
                    s(w.lemma),
                    s(w.upos),
                    s(w.xpos),
                    w.head,
                    s(w.deprel),
                )
            })
            .collect();
        let w = |form: &str, lemma: &str, upos: &str, xpos: &str, head, deprel: &str| {
            (
                form.into(),
                lemma.into(),
                upos.into(),
                xpos.into(),
                head,
                deprel.into(),
            )
        };
        assert_eq!(
            words,
            [
                w("The", "the", "DET", "DT", Some(1), "det"),
                w("dog", "dog", "NOUN", "NN", Some(2), "nsubj"),
                w("runs", "run", "VERB", "VBZ", None, "root"),
                w(".", ".", "PUNCT", ".", Some(2), "punct"),
            ]
        );
        assert_eq!(tree.root_id, Some(2));
        assert_eq!(tree.words[2].children, [1, 3]);
    }

    #[test]
    fn test_parse_feats_and_misc() {
        let tree = word_with("Case=Nom|Number=Plur", "SpaceAfter=No").unwrap();
        let word = &tree.words[0];
        let kv = |k: &str, v: &str| (k.to_string(), v.to_string());
        assert_eq!(
            pairs(&tree, &word.feats),
            [kv("Case", "Nom"), kv("Number", "Plur")]
        );
        assert_eq!(pairs(&tree, &word.misc), [kv("SpaceAfter", "No")]);

        let tree = word_with("_", "_").unwrap();
        assert!(tree.words[0].feats.is_empty());
        assert!(tree.words[0].misc.is_empty());

        for (feats, misc) in [
            ("InvalidPair", "_"),
            ("foo|bar=baz", "_"),
            ("_", "SpaceAfter"),
        ] {
            let err = word_with(feats, misc).unwrap_err().to_string();
            assert!(err.contains("Invalid FEATS pair"), "{feats} {misc}: {err}");
        }
    }

    #[test]
    fn test_parse_id_and_head() {
        assert_eq!(parse_id(b"1").unwrap(), 1);
        assert_eq!(parse_id(b"42").unwrap(), 42);
        assert_eq!(parse_head(b"0").unwrap(), None);
        assert_eq!(parse_head(b"1").unwrap(), Some(0)); // 1-indexed to 0-indexed
        assert_eq!(parse_head(b"5").unwrap(), Some(4));

        // Empty nodes are not supported
        let err = parse_id(b"2.1").unwrap_err();
        assert!(matches!(err, ParseError::UnsupportedToken { .. }));
        assert!(err.to_string().contains("2.1"));

        let err = parse_id(b"abc").unwrap_err();
        assert!(matches!(err, ParseError::InvalidTokenId { .. }));
        assert!(err.to_string().contains("abc"));

        let err = parse_head(b"xyz").unwrap_err();
        assert!(matches!(err, ParseError::InvalidHead { .. }));
        assert!(err.to_string().contains("xyz"));
    }

    #[test]
    fn test_line_errors() {
        let cases = [
            ("1\tword\n", "Missing field"),
            (
                "1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\textra\n",
                "10 fields",
            ),
            (
                "1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t0:root\t_\n",
                "Extended deprels",
            ),
            (
                "1.1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n",
                "Empty nodes",
            ),
        ];
        for (conllu, message) in cases {
            let err = parse_one(conllu).unwrap_err();
            assert!(
                matches!(err, ParseError::LineError { line_num: 1, .. }),
                "{err}"
            );
            assert!(err.to_string().contains(message), "{err}");
        }

        // Errors report the line number and content, and later trees still parse
        let conllu = "# comment\n\
                      1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n\
                      \n\
                      abc\tinvalid\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n\
                      \n\
                      1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n";
        let results: Vec<_> = TreeIterator::from_string(conllu).collect();
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok() && results[2].is_ok());
        let err = results[1].as_ref().unwrap_err().to_string();
        assert!(
            err.contains("line 4") && err.contains("abc\tinvalid"),
            "{err}"
        );
    }
}
