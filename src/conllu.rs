//! CoNLL-U file parsing
//!
//! Parses CoNLL-U format files into Tree structures.
//! Multiword tokens, empty nodes and enhanced dependencies (DEPS) are checked
//! for well-formed IDs where relevant and otherwise ignored: trees hold the
//! basic dependency tree and sentence metadata.
//!
//! CoNLL-U format: https://universaldependencies.org/format.html

use crate::bytes::{BytestringPool, bs_atoi, bs_split_once};
use crate::tree::{Dep, Features, TokenId, Tree, WordId};
use flate2::read::MultiGzDecoder;
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

    #[error("Expected 10 fields, found more than 10")]
    TooManyFields,

    #[error("Invalid DEPS pair: {pair}")]
    InvalidDepsPair { pair: String },

    #[error("Empty node {token_id} out of place (expected {after}.N)")]
    MisplacedEmptyNode { token_id: String, after: TokenId },

    #[error("Invalid token ID: {token_id}")]
    InvalidTokenId { token_id: String },

    #[error("Invalid HEAD: {head}")]
    InvalidHead { head: String },

    #[error("Empty {field} field")]
    EmptyField { field: &'static str },

    #[error("Expected word ID {expected}, found {found}")]
    UnexpectedId { expected: TokenId, found: String },

    #[error("Comment line inside a sentence (comments must precede the word lines)")]
    CommentInSentence,

    #[error("Second root word (first root at line {first})")]
    MultipleRoots { first: usize },
}

const FIELD_NAMES: [&str; 10] = [
    "ID", "FORM", "LEMMA", "UPOS", "XPOS", "FEATS", "HEAD", "DEPREL", "DEPS", "MISC",
];

/// CoNLL-U reader that iterates over sentences
pub struct TreeIterator<R: BufRead> {
    reader: R,
    line_num: usize,
    string_pool: BytestringPool,
}

impl<R: BufRead> TreeIterator<R> {
    /// Parse a word line into `tree`; multiword-token and empty-node lines are checked and
    /// skipped, and DEPS is ignored
    fn parse_line(&mut self, tree: &mut Tree, line: &[u8]) -> Result<(), ParseError> {
        // Split into exactly 10 tab-separated fields
        let mut fields: [&[u8]; 10] = [&[]; 10];
        let mut n = 0;
        let mut start = 0;
        for end in memchr::memchr_iter(b'\t', line).chain([line.len()]) {
            if n == 10 {
                return Err(ParseError::TooManyFields);
            }
            fields[n] = &line[start..end];
            n += 1;
            start = end + 1;
        }
        if n != 10 {
            return Err(ParseError::MissingField { field_num: n + 1 });
        }
        // DEPS (ignored) and MISC may be empty
        if let Some(i) = fields[..8].iter().position(|f| f.is_empty()) {
            return Err(ParseError::EmptyField {
                field: FIELD_NAMES[i],
            });
        }
        let [
            id,
            form,
            lemma,
            upos,
            xpos,
            feats,
            head,
            deprel,
            _deps,
            misc,
        ] = fields;

        let word_id = tree.words.len();
        let next_word_id = word_id + 1;

        // Multiword token (e.g., "1-2"): must start at the next word ID
        if let Some((first, last)) = bs_split_once(id, b'-') {
            return match (bs_atoi(first), bs_atoi(last)) {
                (Some(first), Some(last)) if first < last => {
                    if first == next_word_id {
                        Ok(())
                    } else {
                        Err(ParseError::UnexpectedId {
                            expected: next_word_id,
                            found: String::from_utf8_lossy(id).into_owned(),
                        })
                    }
                }
                _ => Err(ParseError::InvalidTokenId {
                    token_id: String::from_utf8_lossy(id).into_owned(),
                }),
            };
        }

        // Empty node (e.g., "5.1"): must follow the last word read
        if let Some((after, n)) = bs_split_once(id, b'.') {
            return match (bs_atoi(after), bs_atoi(n)) {
                (Some(after), Some(n)) if n >= 1 => {
                    if after == word_id {
                        Ok(())
                    } else {
                        Err(ParseError::MisplacedEmptyNode {
                            token_id: String::from_utf8_lossy(id).into_owned(),
                            after: word_id,
                        })
                    }
                }
                _ => Err(ParseError::InvalidTokenId {
                    token_id: String::from_utf8_lossy(id).into_owned(),
                }),
            };
        }

        let Some(token_id) = bs_atoi(id) else {
            return Err(ParseError::InvalidTokenId {
                token_id: String::from_utf8_lossy(id).into_owned(),
            });
        };
        if token_id != next_word_id {
            return Err(ParseError::UnexpectedId {
                expected: next_word_id,
                found: String::from_utf8_lossy(id).into_owned(),
            });
        }
        let feats = self.parse_features(feats);
        let head = parse_head(head)?;
        let misc = self.parse_features(misc);

        tree.add_word(
            word_id, token_id, form, lemma, upos, xpos, feats, head, deprel, misc,
        );
        Ok(())
    }

    /// After a line error, skip to the end of the sentence so the next one parses cleanly
    fn skip_sentence(&mut self) {
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            match self.reader.read_until(b'\n', &mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    self.line_num += 1;
                    if buffer == b"\n" {
                        return;
                    }
                }
            }
        }
    }

    /// Parse a FEATS or MISC field (key=value|key=value); an item without `=` gets an empty
    /// value, and empty items are skipped
    fn parse_features(&mut self, s: &[u8]) -> Features {
        if s == b"_" {
            return Features::new();
        }
        s.split(|&b| b == b'|')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (k, v) = bs_split_once(pair, b'=').unwrap_or((pair, b""));
                (
                    self.string_pool.get_or_intern(k),
                    self.string_pool.get_or_intern(v),
                )
            })
            .collect()
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
                    pair: String::from_utf8_lossy(pair).into_owned(),
                });
            };

            let Some(head) = bs_atoi(head_str) else {
                return Err(ParseError::InvalidDepsPair {
                    pair: String::from_utf8_lossy(pair).into_owned(),
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
            Box::new(MultiGzDecoder::new(reader))
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
        let mut buffer: Vec<u8> = Vec::with_capacity(100);
        let mut has_content = false;
        let mut first_comment: Option<usize> = None;
        // Largest HEAD seen and its line, checked against the sentence length at the end
        let mut max_head: Option<(WordId, usize)> = None;
        let mut first_word_line: Option<usize> = None;
        let mut root_line: Option<usize> = None;

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
                    let mut line = buffer.strip_suffix(b"\n").unwrap_or(&buffer);
                    line = line.strip_suffix(b"\r").unwrap_or(line);
                    if self.line_num == 1 {
                        line = line.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(line);
                    }
                    if line.is_empty() {
                        // Blank line = sentence boundary if we have content or comments
                        if has_content || first_comment.is_some() {
                            break;
                        }
                        // Skip leading/multiple blank lines
                        continue;
                    }

                    let result = if line[0] == b'#' {
                        if has_content {
                            Err(ParseError::CommentInSentence)
                        } else {
                            first_comment.get_or_insert(self.line_num);
                            parse_comment(line, &mut tree)
                        }
                    } else {
                        has_content = true;
                        first_word_line.get_or_insert(self.line_num);
                        let n_words = tree.words.len();
                        self.parse_line(&mut tree, line).and_then(|()| {
                            // Multiword-token lines add no word
                            if tree.words.len() == n_words {
                                return Ok(());
                            }
                            match tree.words[n_words].head {
                                None => match root_line {
                                    Some(first) => {
                                        return Err(ParseError::MultipleRoots { first });
                                    }
                                    None => root_line = Some(self.line_num),
                                },
                                Some(head) if max_head.is_none_or(|(max, _)| head > max) => {
                                    max_head = Some((head, self.line_num));
                                }
                                Some(_) => {}
                            }
                            Ok(())
                        })
                    };
                    if let Err(e) = result {
                        let error = ParseError::LineError {
                            line_num: self.line_num,
                            line_content: String::from_utf8_lossy(line).into_owned(),
                            message: e.to_string(),
                        };
                        self.skip_sentence();
                        return Some(Err(error));
                    }
                }
            }
        }

        // At EOF with nothing read, we're done; comments alone are an empty sentence
        if !has_content {
            return first_comment.map(|line_num| {
                Err(ParseError::LineErrorNoContent {
                    line_num,
                    message: "Comment lines with no word lines following".to_string(),
                })
            });
        }

        let len = tree.words.len();
        if let Some((head, line_num)) = max_head.filter(|&(head, _)| head >= len) {
            return Some(Err(ParseError::LineErrorNoContent {
                line_num,
                message: format!(
                    "HEAD {} is out of range (sentence has {len} words)",
                    head + 1
                ),
            }));
        }

        let sentence_error = |message: String| {
            Some(Err(ParseError::LineErrorNoContent {
                line_num: first_word_line.unwrap_or(self.line_num),
                message,
            }))
        };
        if root_line.is_none() {
            return sentence_error("No root word (no word has HEAD 0)".to_string());
        }

        tree.compile_tree();
        // With one root and in-range heads, a word the root can't reach is on a cycle
        let mut reached = vec![false; len];
        let mut stack = Vec::from_iter(tree.root_id);
        while let Some(id) = stack.pop() {
            reached[id] = true;
            stack.extend(&tree.words[id].children);
        }
        if let Some(id) = reached.iter().position(|&r| !r) {
            return sentence_error(format!(
                "HEAD links form a cycle (word {} does not reach the root)",
                tree.words[id].token_id
            ));
        }
        Some(Ok(tree))
    }
}

/// Parse a comment line (starts with #): `# key = value`, or `# key` with an empty value
fn parse_comment(line: &[u8], tree: &mut Tree) -> Result<(), ParseError> {
    let line = str::from_utf8(&line[1..])?;
    let (key, value) = line.split_once('=').unwrap_or((line, ""));
    let (key, value) = (key.trim(), value.trim());
    if key == "text" {
        tree.sentence_text = Some(value.to_string());
    } else if !key.is_empty() {
        tree.metadata.insert(key.to_string(), value.to_string());
    }
    Ok(())
}

/// Parse HEAD field (0 for the root, else a 1-based word ID)
fn parse_head(s: &[u8]) -> Result<Option<WordId>, ParseError> {
    match bs_atoi(s) {
        Some(0) => Ok(None),
        // HEAD is 1-indexed in CoNLL-U, convert to 0-indexed WordIds
        Some(head) => Ok(Some(head - 1)),
        None => Err(ParseError::InvalidHead {
            head: String::from_utf8_lossy(s).into_owned(),
        }),
    }
}

/// Test helper: one tree from rows of "form lemma upos xpos head deprel"
/// (HEAD is 1-based as in CoNLL-U, 0 for the root)
#[cfg(test)]
pub(crate) fn tree_from_rows(rows: &[&str]) -> Tree {
    let conllu: String = rows
        .iter()
        .zip(1..)
        .map(|(row, id)| {
            let [form, lemma, upos, xpos, head, deprel] =
                row.split_whitespace().collect::<Vec<_>>()[..]
            else {
                panic!("bad row: {row}");
            };
            format!("{id}\t{form}\t{lemma}\t{upos}\t{xpos}\t_\t{head}\t{deprel}\t_\t_\n")
        })
        .collect();
    TreeIterator::from_string(&conllu).next().unwrap().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(conllu: &[u8]) -> Result<Tree, ParseError> {
        let mut trees = TreeIterator {
            reader: conllu,
            line_num: 0,
            string_pool: BytestringPool::new(),
        };
        trees.next().unwrap()
    }

    /// One root word with the given FEATS and MISC fields
    fn word_with(feats: &str, misc: &str) -> Result<Tree, ParseError> {
        parse_one(format!("1\tword\tlemma\tX\tX\t{feats}\t0\troot\t_\t{misc}\n").as_bytes())
    }

    /// Key/value pairs of a FEATS or MISC field, as strings
    fn pairs(tree: &Tree, feats: &Features) -> Vec<(String, String)> {
        let s = |sym| String::from_utf8(tree.string_pool.resolve(sym).to_vec()).unwrap();
        feats.iter().map(|&(k, v)| (s(k), s(v))).collect()
    }

    #[test]
    fn test_parse_sentence() {
        let conllu = "\n\
                      # newdoc\n\
                      #\n\
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
        // `# key` with no value is kept; a bare `#` is not
        let metadata = [("newdoc", ""), ("sent_id", "s1")];
        assert_eq!(
            tree.metadata,
            metadata.map(|(k, v)| (k.to_string(), v.to_string())).into()
        );

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
        assert_eq!(
            tree.words.iter().map(|w| w.id).collect::<Vec<_>>(),
            [0, 1, 2, 3]
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

        // Items without `=` get an empty value; empty items are skipped
        for (field, expected) in [
            ("_", vec![]),
            ("", vec![]),
            (
                "SpaceAfter|Gloss=dog",
                vec![kv("SpaceAfter", ""), kv("Gloss", "dog")],
            ),
            ("|a=1||b=2|", vec![kv("a", "1"), kv("b", "2")]),
        ] {
            let tree = word_with("_", field).unwrap();
            assert_eq!(
                pairs(&tree, &tree.words[0].misc),
                expected,
                "MISC {field:?}"
            );
            if !field.is_empty() {
                let tree = word_with(field, "_").unwrap();
                assert_eq!(
                    pairs(&tree, &tree.words[0].feats),
                    expected,
                    "FEATS {field:?}"
                );
            }
        }
    }

    #[test]
    fn test_line_errors() {
        let cases: [(&[u8], usize, &str); 25] = [
            (b"1\tword\n", 1, "Missing field 3"),
            (b"1\tw\tl\tX\tX\t_\t0\troot\t_\n", 1, "Missing field 10"),
            (b"1\tw\tl\tX\tX\t_\t0\troot\t_\t_\textra\n", 1, "10 fields"),
            (
                b"1.1\tw\tl\tX\tX\t_\t_\t_\t_\t_\n",
                1,
                "Empty node 1.1 out of place (expected 0.N)",
            ),
            (
                b"0.0\tw\tl\tX\tX\t_\t_\t_\t_\t_\n",
                1,
                "Invalid token ID: 0.0",
            ),
            (
                b"0.x\tw\tl\tX\tX\t_\t_\t_\t_\t_\n",
                1,
                "Invalid token ID: 0.x",
            ),
            (
                b"abc\tw\tl\tX\tX\t_\t0\troot\t_\t_\n",
                1,
                "Invalid token ID: abc",
            ),
            (
                b"1\tw\tl\tX\tX\t_\txyz\troot\t_\t_\n",
                1,
                "Invalid HEAD: xyz",
            ),
            (b"\tw\tl\tX\tX\t_\t0\troot\t_\t_\n", 1, "Empty ID field"),
            (b"1\t\tl\tX\tX\t_\t0\troot\t_\t_\n", 1, "Empty FORM field"),
            (b"1\tw\tl\tX\tX\t_\t\troot\t_\t_\n", 1, "Empty HEAD field"),
            (b"1\tw\tl\tX\tX\t\t0\troot\t_\t_\n", 1, "Empty FEATS field"),
            (
                b"2\tw\tl\tX\tX\t_\t0\troot\t_\t_\n",
                1,
                "Expected word ID 1, found 2",
            ),
            (
                b"2-3\tw\t_\t_\t_\t_\t_\t_\t_\t_\n",
                1,
                "Expected word ID 1, found 2-3",
            ),
            (
                b"1-1\tw\t_\t_\t_\t_\t_\t_\t_\t_\n",
                1,
                "Invalid token ID: 1-1",
            ),
            (
                b"abc-\tw\t_\t_\t_\t_\t_\t_\t_\t_\n",
                1,
                "Invalid token ID: abc-",
            ),
            (
                b"1\tw\tl\tX\tX\t_\t2\tdep\t_\t_\n",
                1,
                "HEAD 2 is out of range (sentence has 1 words)",
            ),
            (
                b"# text = \xff\n1\tw\tl\tX\tX\t_\t0\troot\t_\t_\n",
                1,
                "invalid utf-8",
            ),
            (
                b"1\tw\tl\tX\tX\t_\t0\troot\t_\t_\n# x = y\n",
                2,
                "Comment line inside a sentence",
            ),
            (
                b"# sent_id = a\n\n1\tw\tl\tX\tX\t_\t0\troot\t_\t_\n",
                1,
                "Comment lines with no word lines",
            ),
            (b"\n# newdoc\n", 2, "Comment lines with no word lines"),
            (b"1\tw\tl\tX\tX\t_\t_\troot\t_\t_\n", 1, "Invalid HEAD: _"),
            (
                b"1\ta\tl\tX\tX\t_\t0\troot\t_\t_\n2\tb\tl\tX\tX\t_\t0\troot\t_\t_\n",
                2,
                "Second root word (first root at line 1)",
            ),
            (
                b"# x = y\n1\ta\tl\tX\tX\t_\t2\tdep\t_\t_\n2\tb\tl\tX\tX\t_\t1\tdep\t_\t_\n",
                2,
                "No root word",
            ),
            (
                b"1\ta\tl\tX\tX\t_\t0\troot\t_\t_\n2\tb\tl\tX\tX\t_\t3\tdep\t_\t_\n3\tc\tl\tX\tX\t_\t2\tdep\t_\t_\n",
                1,
                "HEAD links form a cycle (word 2 does not reach the root)",
            ),
        ];
        for (conllu, line_num, message) in cases {
            let err = parse_one(conllu).unwrap_err().to_string();
            let location = format!("Parse error at line {line_num}:");
            assert!(err.starts_with(&location), "{err}");
            assert!(err.contains(message), "{err}");
        }

        // Errors report the line number and content, and the rest of the bad
        // sentence is skipped so later trees still parse
        let conllu = "# comment\n\
                      1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n\
                      \n\
                      1\tword\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n\
                      abc\tinvalid\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n\
                      3\tword\tlemma\tNOUN\tNN\t_\t1\tdep\t_\t_\n\
                      \n\
                      1\tnext\tlemma\tNOUN\tNN\t_\t0\troot\t_\t_\n";
        let results: Vec<_> = TreeIterator::from_string(conllu).collect();
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok() && results[2].is_ok());
        let err = results[1].as_ref().unwrap_err().to_string();
        assert!(
            err.contains("line 5") && err.contains("abc\tinvalid"),
            "{err}"
        );
        let next = results[2].as_ref().unwrap();
        assert_eq!(*next.string_pool.resolve(next.words[0].form), *b"next");
    }

    #[test]
    fn test_enhanced_layer_ignored() {
        // Empty nodes and DEPS are skipped; the basic tree is unchanged
        let conllu = "0.1\tsaid\tsay\tVERB\t_\t_\t_\t_\t2:parataxis\t_\n\
                      1\tSue\tSue\tPROPN\t_\t_\t2\tnsubj\t2:nsubj\t_\n\
                      2\tlikes\tlike\tVERB\t_\t_\t0\troot\t0:root\t_\n\
                      3\tcoffee\tcoffee\tNOUN\t_\t_\t2\tobj\t2:obj\t_\n\
                      4\tand\tand\tCCONJ\t_\t_\t5\tcc\t5.1:cc\t_\n\
                      5\tBill\tBill\tPROPN\t_\t_\t2\tconj\t5.1:nsubj\t_\n\
                      5.1\tlikes\tlike\tVERB\t_\t_\t_\t_\t2:conj\tCopyOf=2\n\
                      6\ttea\ttea\tNOUN\t_\t_\t5\torphan\t5.1:obj\t_\n";
        let tree = parse_one(conllu.as_bytes()).unwrap();
        let heads: Vec<_> = tree.words.iter().map(|w| w.head).collect();
        assert_eq!(heads, [Some(1), None, Some(1), Some(4), Some(1), Some(4)]);
    }

    #[test]
    fn test_lenient_input() {
        // A leading UTF-8 BOM is skipped, HEAD 00 is the root like HEAD 0, and DEPS may be
        // empty
        for conllu in [
            &b"\xEF\xBB\xBF1\tw\tl\tX\tX\t_\t0\troot\t_\t_\n"[..],
            b"1\tw\tl\tX\tX\t_\t00\troot\t_\t_\n",
            b"1\tw\tl\tX\tX\t_\t0\troot\t\t_\n",
        ] {
            let tree = parse_one(conllu).unwrap();
            assert_eq!(tree.root_id, Some(0));
        }
    }

    #[test]
    fn test_crlf_line_endings() {
        let conllu = "# x = y\r\n\
                      1\tw\tl\tX\tX\t_\t0\troot\t_\tGloss=w\r\n\
                      \r\n\
                      1\tv\tl\tX\tX\t_\t0\troot\t_\t_\r\n";
        let trees: Vec<_> = TreeIterator::from_string(conllu)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].metadata, [("x".into(), "y".into())].into());
        let kv = (String::from("Gloss"), String::from("w"));
        assert_eq!(pairs(&trees[0], &trees[0].words[0].misc), [kv]);
    }
}
