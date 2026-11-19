"""Tests for treesearch Python bindings.

These tests cover the Python API surface: types, conversions, error handling, and
Python idioms. Query semantics are tested in the Rust test suite.
"""

import gzip
from pathlib import Path

import pytest
import treesearch

SAMPLE = """# sent_id = 1
# text = He helped us to win.
# source = test
1	He	he	PRON	PRP	Case=Nom|Number=Sing	2	nsubj	_	_
2	helped	help	VERB	VBD	_	0	root	_	_
3	us	we	PRON	_	_	2	obj	_	_
4	to	to	PART	TO	_	5	mark	_	_
5	win	win	VERB	VB	_	2	xcomp	_	SpaceAfter=No
6	.	.	PUNCT	.	_	2	punct	_	_

"""

# Two trees, one verb each
TWO_TREES = """# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	_

# text = Cats sleep.
1	Cats	cat	NOUN	NNS	_	2	nsubj	_	_
2	sleep	sleep	VERB	VBP	_	0	root	_	_

"""

VERBS = 'MATCH { V [upos="VERB"]; }'


@pytest.fixture
def tree():
    return next(treesearch.from_string(SAMPLE).trees())


@pytest.fixture
def corpus_dir(tmp_path):
    """Directory with c.conllu, a.conllu, b.conllu, each holding TWO_TREES
    with the file name prefixed to the sentence text."""
    for name in "cab":
        text = TWO_TREES.replace("# text = ", f"# text = {name} ")
        (tmp_path / f"{name}.conllu").write_text(text)
    return tmp_path


def first_letters(trees):
    return [t.sentence_text[0] for t in trees]


# ==============================================================================
# Queries
# ==============================================================================


def test_compile_query():
    assert "Pattern" in repr(treesearch.compile_query(VERBS))
    treesearch.compile_query("MATCH { }")


@pytest.mark.parametrize(
    "query", ["INVALID SYNTAX [[[", "", "MATCH", "MATCH { A []; B []; A <0< B; }"]
)
def test_invalid_query_raises_valueerror(query):
    with pytest.raises(ValueError, match="Query parse error"):
        treesearch.compile_query(query)


# ==============================================================================
# Tree and Word
# ==============================================================================


def test_tree(tree):
    assert tree.sentence_text == "He helped us to win."
    assert tree.metadata == {"sent_id": "1", "source": "test"}
    assert len(tree) == 6
    assert repr(tree).startswith("<Tree len=6 words='He helped us")
    assert [w.id for w in tree] == list(range(6))

    no_text = next(treesearch.from_string("1\tx\tx\tX\t_\t_\t0\troot\t_\t_\n").trees())
    assert no_text.sentence_text is None
    assert no_text.metadata == {}


def test_tree_indexing(tree):
    assert tree[0].form == "He"
    assert tree[-1].id == 5
    assert tree.word(1).form == "helped"
    for bad in [6, -7]:
        with pytest.raises(IndexError):
            tree[bad]
    # tree.word() takes only word ids
    with pytest.raises(IndexError):
        tree.word(-1)
    with pytest.raises(IndexError, match="word index out of range: 999"):
        tree.word(999)


def test_word_properties(tree):
    he, helped, us = tree[0], tree[1], tree[2]
    assert (helped.id, helped.token_id) == (1, 2)  # token_id is 1-based as in CoNLL-U
    assert (helped.form, helped.lemma, helped.upos, helped.deprel) == (
        "helped",
        "help",
        "VERB",
        "root",
    )
    assert helped.xpos == "VBD"
    assert us.xpos is None  # "_"
    assert (he.head, helped.head) == (1, None)
    assert he.feats == {"Case": "Nom", "Number": "Sing"}
    assert helped.feats == {}
    assert tree[4].misc == {"SpaceAfter": "No"}
    assert repr(helped) == "<Word id=1 form='helped' lemma='help' upos='VERB' deprel='root'>"

    unicode = next(treesearch.from_string("1\t日本語\t日本語\tNOUN\t_\t_\t0\troot\t_\t_\n").trees())
    assert unicode[0].form == "日本語"


def test_word_equality(tree):
    """Words are equal (and hash alike) when they are the same word of the same tree."""
    verb = tree.word(1)
    assert tree.word(0).parent() == verb
    assert tree.word(0) != verb
    assert verb != 1
    assert len({verb, tree[1], tree.word(0)}) == 2

    other = next(treesearch.from_string(SAMPLE).trees())
    assert other[1] != verb


def test_word_navigation(tree):
    helped, win = tree[1], tree[4]
    assert tree[0].parent() == helped
    assert helped.parent() is None

    assert helped.children_ids == [0, 2, 4, 5]
    assert [c.id for c in helped.children()] == helped.children_ids
    assert [c.form for c in helped.children_by_deprel("nsubj")] == ["He"]
    assert helped.children_by_deprel("nonexistent") == []

    assert {w.form for w in helped.descendants()} == {"He", "us", "to", "win", "."}
    assert sorted(helped.descendant_ids) == [0, 2, 3, 4, 5]
    assert [w.form for w in win.descendants()] == ["to"]
    assert tree[0].descendants() == []


# ==============================================================================
# Treebanks
# ==============================================================================


def test_from_string():
    tb = treesearch.from_string(TWO_TREES)
    # Treebanks can be iterated more than once
    for _ in range(2):
        assert [len(t) for t in tb.trees()] == [3, 2]
    assert list(treesearch.from_string("").trees()) == []


def test_from_file(tmp_path):
    plain = tmp_path / "test.conllu"
    plain.write_text(SAMPLE)
    compressed = tmp_path / "test.conllu.gz"
    with gzip.open(compressed, "wt", encoding="utf-8") as f:
        f.write(SAMPLE)

    for path in [plain, compressed, str(plain)]:
        assert [len(t) for t in treesearch.Treebank.from_file(path).trees()] == [6]

    # Every member of a concatenated gzip file is read
    multi = tmp_path / "multi.conllu.gz"
    multi.write_bytes(gzip.compress(SAMPLE.encode()) * 2)
    assert [len(t) for t in treesearch.Treebank.from_file(multi).trees()] == [6, 6]


def test_load(corpus_dir):
    glob = f"{corpus_dir}/*.conllu"
    paths = [corpus_dir / f"{name}.conllu" for name in "cab"]

    # Globs are expanded in sorted order; explicit paths are read in the order given
    assert first_letters(treesearch.load(glob).trees()) == list("aabbcc")
    assert first_letters(treesearch.load(Path(glob)).trees()) == list("aabbcc")
    assert first_letters(treesearch.Treebank.from_glob(glob).trees()) == list("aabbcc")
    assert first_letters(treesearch.load(str(p) for p in paths).trees()) == list("ccaabb")
    assert first_letters(treesearch.Treebank.from_files(paths).trees()) == list("ccaabb")
    assert first_letters(treesearch.load(paths[1]).trees()) == list("aa")
    assert list(treesearch.load(f"{corpus_dir}/nonexistent/*.conllu").trees()) == []

    # Unordered iteration yields the same trees in some order
    unordered = first_letters(treesearch.load(glob).trees(ordered=False))
    assert sorted(unordered) == list("aabbcc")


def test_convenience_functions(corpus_dir):
    glob = f"{corpus_dir}/*.conllu"
    assert first_letters(treesearch.trees(glob)) == list("aabbcc")
    assert first_letters(t for t, _ in treesearch.search(glob, VERBS)) == list("aabbcc")


@pytest.mark.parametrize("source", [b"corpus.conllu", 3, None])
def test_load_bad_type_raises(source):
    with pytest.raises(TypeError):
        treesearch.load(source)


def test_load_errors(tmp_path):
    with pytest.raises(ValueError, match="Invalid glob pattern"):
        treesearch.load("data/[.conllu")

    # Files are opened lazily, so errors are raised on iteration
    missing = treesearch.load(tmp_path / "missing.conllu").trees()
    with pytest.raises(OSError, match="Failed to open file"):
        list(missing)

    bad = tmp_path / "bad.conllu"
    bad.write_text("1\truns\trun\tVERB\tVBZ\t_\tx\troot\t_\t_\n")
    with pytest.raises(ValueError, match=r"bad\.conllu: Parse error at line 1"):
        list(treesearch.load(bad).trees())


# ==============================================================================
# Search
# ==============================================================================


def test_search():
    tb = treesearch.from_string(SAMPLE)
    matches = list(tb.search(VERBS))
    assert [m for _, m in matches] == [{"V": 1}, {"V": 4}]
    assert all(isinstance(t, treesearch.Tree) for t, _ in matches)
    assert [m for _, m in tb.search(treesearch.compile_query(VERBS))] == [{"V": 1}, {"V": 4}]

    with pytest.raises(ValueError, match="Query parse error"):
        tb.search("INVALID SYNTAX")


def test_search_bindings():
    """Match dicts bind every MATCH variable, and OPTIONAL variables only when found."""
    tb = treesearch.from_string(SAMPLE)
    query = """
        MATCH { V [upos="VERB"]; }
        EXCEPT { V -[punct]-> _; }
        OPTIONAL { M []; V -[mark]-> M; }
    """
    assert [m for _, m in tb.search(query)] == [{"V": 4, "M": 3}]
    query = 'MATCH { V [upos="VERB"]; } OPTIONAL { M []; V -[mark]-> M; }'
    assert [m for _, m in tb.search(query)] == [{"V": 1}, {"V": 4, "M": 3}]


def test_search_trees(tree):
    assert len(list(treesearch.search_trees(tree, VERBS))) == 2

    trees = list(treesearch.from_string(TWO_TREES).trees())
    for source in [trees, iter(trees)]:
        matches = list(treesearch.search_trees(source, VERBS))
        assert len(matches) == 2
    # The original Tree objects are returned
    assert matches[0][0].word(0) == trees[0].word(0)

    with pytest.raises(TypeError):
        treesearch.search_trees([1, 2], VERBS)


def test_filter():
    tb = treesearch.from_string(SAMPLE + TWO_TREES)
    pron = 'MATCH { P [upos="PRON"]; }'
    for query in [pron, treesearch.compile_query(pron)]:
        assert [len(t) for t in tb.filter(query)] == [6]


# ==============================================================================
# Visualization
# ==============================================================================


def test_to_displacy(tree):
    data = treesearch.to_displacy(tree)
    assert data == tree.to_displacy()
    assert data["words"][:2] == [{"text": "He", "tag": "PRON"}, {"text": "helped", "tag": "VERB"}]
    assert len(data["words"]) == 6

    # One arc per non-root word, pointing from head to dependent
    arcs = {a["label"]: a for a in data["arcs"]}
    assert len(data["arcs"]) == 5
    assert arcs["nsubj"] == {"start": 0, "end": 1, "label": "nsubj", "dir": "left"}
    assert arcs["obj"] == {"start": 1, "end": 2, "label": "obj", "dir": "right"}
