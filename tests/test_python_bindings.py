"""Tests for treesearch Python bindings.

This test suite focuses on:
- Python API surface and type correctness
- Error handling and exception types
- Integration with Python idioms (iterators, context managers)
- Edge cases specific to the binding layer

Algorithm correctness is tested in the Rust test suite.
"""

import gzip

import pytest

import treesearch


# ==============================================================================
# Fixtures
# ==============================================================================


@pytest.fixture
def sample_conllu():
    """Simple CoNLL-U test data."""
    return """# text = He helped us to win.
1	He	he	PRON	PRP	_	2	nsubj	_	_
2	helped	help	VERB	VBD	_	0	root	_	_
3	us	we	PRON	PRP	_	2	obj	_	_
4	to	to	PART	TO	_	5	mark	_	_
5	win	win	VERB	VB	_	2	xcomp	_	_
6	.	.	PUNCT	.	_	2	punct	_	_

"""


@pytest.fixture
def complex_conllu():
    """CoNLL-U data with metadata and xpos."""
    return """# sent_id = 1
# text = The big dog runs.
# source = test
1	The	the	DET	DT	Definite=Def	2	det	_	SpaceAfter=No
2	big	big	ADJ	JJ	Degree=Pos	3	amod	_	_
3	dog	dog	NOUN	NN	Number=Sing	4	nsubj	_	_
4	runs	run	VERB	VBZ	Tense=Pres	0	root	_	_

"""


@pytest.fixture
def multi_tree_conllu():
    """CoNLL-U data with multiple trees."""
    return """# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	_

# text = Cats sleep.
1	Cats	cat	NOUN	NNS	_	2	nsubj	_	_
2	sleep	sleep	VERB	VBP	_	0	root	_	_

"""


@pytest.fixture
def temp_conllu_file(sample_conllu, tmp_path):
    """Create a temporary CoNLL-U file."""
    path = tmp_path / "test.conllu"
    path.write_text(sample_conllu)
    return str(path)


@pytest.fixture
def temp_gzip_file(sample_conllu, tmp_path):
    """Create a temporary gzipped CoNLL-U file."""
    path = tmp_path / "test.conllu.gz"
    with gzip.open(path, "wt", encoding="utf-8") as f:
        f.write(sample_conllu)
    return str(path)


@pytest.fixture
def temp_multi_files(multi_tree_conllu, tmp_path):
    """Create multiple temporary CoNLL-U files."""
    files = []
    for i in range(3):
        path = tmp_path / f"test_{i}.conllu"
        path.write_text(multi_tree_conllu)
        files.append(str(path))
    return tmp_path, files


# ==============================================================================
# Pattern Tests
# ==============================================================================


class TestPattern:
    """Tests for pattern compilation."""

    def test_compile_query_returns_pattern(self):
        """compile_query returns a Pattern object."""
        pattern = treesearch.compile_query('MATCH { V [upos="VERB"]; }')
        assert pattern is not None
        assert "Pattern" in repr(pattern)

    @pytest.mark.parametrize(
        "query",
        [
            "INVALID SYNTAX [[[",
            "",
            "MATCH",
            "MATCH {}",  # Empty match block - no MATCH {} is valid, but this should work
        ],
    )
    def test_invalid_query_raises_error(self, query):
        """Invalid queries raise ValueError or Exception."""
        # Note: empty MATCH {} is actually valid, so we just check it doesn't crash
        if query == "MATCH {}":
            treesearch.compile_query(query)  # Should not raise
        else:
            with pytest.raises(Exception):
                treesearch.compile_query(query)


# ==============================================================================
# Tree Reading Tests
# ==============================================================================


class TestTreeReading:
    """Tests for reading trees from CoNLL-U data."""

    def test_from_file(self, temp_conllu_file):
        """Read trees from a file."""
        trees = list(treesearch.Treebank.from_file(temp_conllu_file).trees())
        assert len(trees) == 1
        assert len(trees[0]) == 6

    def test_from_gzip(self, temp_gzip_file):
        """Read trees from gzipped file."""
        trees = list(treesearch.Treebank.from_file(temp_gzip_file).trees())
        assert len(trees) == 1

    def test_from_string(self, sample_conllu):
        """Read trees from string."""
        trees = list(treesearch.Treebank.from_string(sample_conllu).trees())
        assert len(trees) == 1

    def test_multiple_trees(self, multi_tree_conllu, tmp_path):
        """Read multiple trees from a file."""
        path = tmp_path / "multi.conllu"
        path.write_text(multi_tree_conllu)
        trees = list(treesearch.Treebank.from_file(str(path)).trees())
        assert len(trees) == 2

    def test_tree_iterator_protocol(self, temp_conllu_file):
        """.trees() returns a proper iterator."""
        tree_iter = treesearch.Treebank.from_file(temp_conllu_file).trees()
        assert hasattr(tree_iter, "__iter__")
        assert hasattr(tree_iter, "__next__")

    def test_nonexistent_file_raises_oserror(self):
        """Reading nonexistent file raises OSError."""
        trees = treesearch.Treebank.from_file("/nonexistent/file.conllu").trees()
        with pytest.raises(OSError, match="Failed to open file"):
            list(trees)


# ==============================================================================
# Tree Properties Tests
# ==============================================================================


class TestTreeProperties:
    """Tests for Tree object properties."""

    def test_sentence_text(self, sample_conllu):
        """Tree.sentence_text returns the text annotation."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        assert tree.sentence_text == "He helped us to win."

    def test_metadata(self, complex_conllu):
        """Tree.metadata returns a dict of metadata."""
        tree = list(treesearch.Treebank.from_string(complex_conllu).trees())[0]
        assert isinstance(tree.metadata, dict)
        assert tree.metadata["sent_id"] == "1"
        assert tree.metadata["source"] == "test"

    def test_len(self, sample_conllu):
        """len(tree) returns word count."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        assert len(tree) == 6

    def test_repr(self, sample_conllu):
        """Tree repr shows length and words."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        assert "<Tree len=6" in repr(tree)

    def test_getitem(self, sample_conllu):
        """tree[i] returns word by index."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        word = tree[0]
        assert word.form == "He"

    def test_getitem_out_of_bounds(self, sample_conllu):
        """tree[invalid] raises IndexError."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        with pytest.raises(IndexError):
            tree[999]

    def test_word_method(self, sample_conllu):
        """tree.word(i) returns word by index."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        word = tree.word(1)
        assert word.form == "helped"

    def test_word_out_of_bounds(self, sample_conllu):
        """tree.word(invalid) raises IndexError with message."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        with pytest.raises(IndexError, match="word index out of range: 999"):
            tree.word(999)


# ==============================================================================
# Word Properties Tests
# ==============================================================================


class TestWordProperties:
    """Tests for Word object properties."""

    @pytest.fixture
    def tree(self, sample_conllu):
        return list(treesearch.Treebank.from_string(sample_conllu).trees())[0]

    @pytest.fixture
    def complex_tree(self, complex_conllu):
        return list(treesearch.Treebank.from_string(complex_conllu).trees())[0]

    def test_basic_properties(self, tree):
        """Word has form, lemma, upos, deprel."""
        word = tree.word(1)  # "helped"
        assert word.id == 1
        assert word.token_id == 2  # 1-based in CoNLL-U
        assert word.form == "helped"
        assert word.lemma == "help"
        assert word.upos == "VERB"
        assert word.deprel == "root"

    def test_xpos_with_value(self, complex_tree):
        """Word.xpos returns string when present."""
        word = complex_tree.word(0)  # "The" with xpos=DT
        assert word.xpos == "DT"

    def test_xpos_underscore_returns_none(self, sample_conllu):
        """Word.xpos returns None for underscore."""
        # sample_conllu has xpos values like PRP, VBD - not underscores
        # Let's create data with underscore xpos
        conllu = "1\tword\tlemma\tNOUN\t_\t_\t0\troot\t_\t_\n\n"
        tree = list(treesearch.Treebank.from_string(conllu).trees())[0]
        assert tree.word(0).xpos is None

    def test_head_property(self, tree):
        """Word.head returns parent id or None for root."""
        assert tree.word(0).head == 1  # "He" -> "helped"
        assert tree.word(1).head is None  # "helped" is root

    def test_feats_as_dict(self, complex_tree):
        """Word.feats returns dict of morphological features."""
        word = complex_tree.word(0)  # "The" with Definite=Def
        assert isinstance(word.feats, dict)
        assert word.feats.get("Definite") == "Def"

    def test_feats_empty(self, sample_conllu):
        """Word.feats returns empty dict when no features."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        assert tree.word(0).feats == {}

    def test_misc_as_dict(self, complex_tree):
        """Word.misc returns dict of misc annotations."""
        word = complex_tree.word(0)  # "The" with SpaceAfter=No
        assert isinstance(word.misc, dict)
        assert word.misc.get("SpaceAfter") == "No"

    def test_repr(self, tree):
        """Word repr shows key properties."""
        word = tree.word(1)
        r = repr(word)
        assert "Word" in r
        assert "helped" in r
        assert "VERB" in r


# ==============================================================================
# Word Navigation Tests
# ==============================================================================


class TestWordNavigation:
    """Tests for Word navigation methods."""

    @pytest.fixture
    def tree(self, sample_conllu):
        return list(treesearch.Treebank.from_string(sample_conllu).trees())[0]

    def test_parent(self, tree):
        """word.parent() returns parent Word."""
        word = tree.word(0)  # "He"
        parent = word.parent()
        assert parent.form == "helped"

    def test_parent_of_root_is_none(self, tree):
        """Root word has no parent."""
        root = tree.word(1)  # "helped"
        assert root.parent() is None

    def test_children(self, tree):
        """word.children() returns list of child Words."""
        verb = tree.word(1)  # "helped"
        children = verb.children()
        forms = [c.form for c in children]
        assert "He" in forms
        assert "us" in forms

    def test_children_ids(self, tree):
        """word.children_ids returns list of child ids."""
        verb = tree.word(1)  # "helped"
        ids = verb.children_ids
        assert isinstance(ids, list)
        assert all(isinstance(i, int) for i in ids)

    def test_children_by_deprel(self, tree):
        """word.children_by_deprel filters by relation."""
        verb = tree.word(1)  # "helped"
        nsubj = verb.children_by_deprel("nsubj")
        assert len(nsubj) == 1
        assert nsubj[0].form == "He"

    def test_children_by_deprel_empty(self, tree):
        """children_by_deprel returns empty list if no match."""
        verb = tree.word(1)
        assert verb.children_by_deprel("nonexistent") == []


# ==============================================================================
# Search Tests - API Surface
# ==============================================================================


class TestSearch:
    """Tests for search functionality - API correctness."""

    @pytest.fixture
    def tree(self, sample_conllu):
        return list(treesearch.Treebank.from_string(sample_conllu).trees())[0]

    def test_search_returns_iterator(self, sample_conllu):
        """Treebank.search returns an iterator."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        result = tb.search('MATCH { V [upos="VERB"]; }')
        assert hasattr(result, "__iter__")
        assert hasattr(result, "__next__")

    def test_search_yields_tree_and_dict(self, sample_conllu):
        """Search yields (tree, match_dict) tuples."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        for tree, match in tb.search('MATCH { V [upos="VERB"]; }'):
            assert hasattr(tree, "word")
            assert isinstance(match, dict)
            break

    def test_search_accepts_string_query(self, sample_conllu):
        """Treebank.search accepts query string directly."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { V [upos="VERB"]; }'))
        assert len(matches) == 2  # helped, win

    def test_search_accepts_compiled_pattern(self, sample_conllu):
        """Treebank.search accepts compiled Pattern."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        pattern = treesearch.compile_query('MATCH { V [upos="VERB"]; }')
        matches = list(tb.search(pattern))
        assert len(matches) == 2

    def test_search_string_and_pattern_equivalent(self, sample_conllu):
        """String and compiled Pattern produce same results."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        query = 'MATCH { V [upos="VERB"]; }'
        str_matches = [m for _, m in tb.search(query)]
        pattern_matches = [m for _, m in tb.search(treesearch.compile_query(query))]
        assert str_matches == pattern_matches

    def test_search_invalid_string_raises_valueerror(self, sample_conllu):
        """Invalid query string raises ValueError."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        with pytest.raises(ValueError, match="Query parse error"):
            list(tb.search("INVALID SYNTAX"))

    def test_search_trees_function(self, tree):
        """search_trees function works on single tree."""
        matches = list(treesearch.search_trees(tree, 'MATCH { V [upos="VERB"]; }'))
        assert len(matches) == 2

    def test_search_trees_with_list(self, sample_conllu, multi_tree_conllu, tmp_path):
        """search_trees works on list of trees."""
        path = tmp_path / "multi.conllu"
        path.write_text(multi_tree_conllu)
        trees = list(treesearch.Treebank.from_file(str(path)).trees())
        matches = list(treesearch.search_trees(trees, 'MATCH { V [upos="VERB"]; }'))
        assert len(matches) == 2  # One verb per tree


# ==============================================================================
# Filter Tests
# ==============================================================================


class TestFilter:
    """Tests for Treebank.filter method."""

    def test_filter_returns_trees(self, sample_conllu):
        """filter() returns Tree objects."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        trees = list(tb.filter('MATCH { V [upos="VERB"]; }'))
        assert len(trees) == 1
        assert hasattr(trees[0], "word")

    def test_filter_accepts_string(self, sample_conllu):
        """filter() accepts query string."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        trees = list(tb.filter('MATCH { V [upos="VERB"]; }'))
        assert len(trees) > 0

    def test_filter_deduplicates(self):
        """filter() returns each tree once even with multiple matches."""
        conllu = """1\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_
2\trunning\trun\tVERB\tVBG\t_\t1\txcomp\t_\t_

"""
        tb = treesearch.Treebank.from_string(conllu)
        # search() returns 2 matches (one per verb)
        assert len(list(tb.search('MATCH { V [upos="VERB"]; }'))) == 2
        # filter() returns 1 tree
        assert len(list(tb.filter('MATCH { V [upos="VERB"]; }'))) == 1

    def test_filter_no_matches(self, sample_conllu):
        """filter() returns empty when no matches."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        trees = list(tb.filter('MATCH { X [upos="NONEXISTENT"]; }'))
        assert len(trees) == 0


# ==============================================================================
# Multi-file Tests
# ==============================================================================


class TestMultiFile:
    """Tests for multi-file operations."""

    def test_load_glob(self, temp_multi_files):
        """load() with glob pattern."""
        tmpdir, _ = temp_multi_files
        trees = list(treesearch.load(f"{tmpdir}/*.conllu").trees())
        assert len(trees) == 6  # 2 trees × 3 files

    def test_ordered_vs_unordered(self, temp_multi_files):
        """ordered parameter controls iteration order."""
        tmpdir, _ = temp_multi_files
        tb = treesearch.load(f"{tmpdir}/*.conllu")

        ordered1 = list(tb.trees(ordered=True))
        ordered2 = list(tb.trees(ordered=True))
        unordered = list(tb.trees(ordered=False))

        # Same count
        assert len(ordered1) == len(unordered) == 6

        # Ordered runs should be identical
        for t1, t2 in zip(ordered1, ordered2):
            assert t1.sentence_text == t2.sentence_text

    def test_search_glob(self, temp_multi_files):
        """search() works with glob pattern."""
        tmpdir, _ = temp_multi_files
        results = list(treesearch.load(f"{tmpdir}/*.conllu").search('MATCH { V [upos="VERB"]; }'))
        assert len(results) == 6

    def test_glob_no_matches(self, tmp_path):
        """Glob that matches no files returns empty."""
        results = list(treesearch.load(f"{tmp_path}/nonexistent/*.conllu").trees())
        assert len(results) == 0


# ==============================================================================
# Constraint Type Tests
# ==============================================================================


class TestConstraintTypes:
    """Tests for different constraint types."""

    def test_upos_constraint(self, sample_conllu):
        """upos constraint matches POS tag."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { V [upos="VERB"]; }'))
        assert len(matches) == 2

    def test_lemma_constraint(self, sample_conllu):
        """lemma constraint matches lemma."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { V [lemma="help"]; }'))
        assert len(matches) == 1

    def test_form_constraint(self, sample_conllu):
        """form constraint matches word form."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { W [form="He"]; }'))
        assert len(matches) == 1

    def test_deprel_constraint(self, sample_conllu):
        """deprel constraint matches dependency relation."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { W [deprel="nsubj"]; }'))
        assert len(matches) == 1

    def test_xpos_constraint(self, complex_conllu):
        """xpos constraint matches language-specific POS."""
        tb = treesearch.Treebank.from_string(complex_conllu)
        matches = list(tb.search('MATCH { W [xpos="DT"]; }'))
        assert len(matches) == 1
        _, match = matches[0]
        assert tb.trees().__next__().word(match["W"]).form == "The"

    def test_feature_constraint(self, complex_conllu):
        """feats.X constraint matches morphological features."""
        tb = treesearch.Treebank.from_string(complex_conllu)
        matches = list(tb.search('MATCH { W [feats.Definite="Def"]; }'))
        assert len(matches) == 1

    def test_misc_constraint(self, complex_conllu):
        """misc.X constraint matches misc annotations."""
        tb = treesearch.Treebank.from_string(complex_conllu)
        matches = list(tb.search('MATCH { W [misc.SpaceAfter="No"]; }'))
        assert len(matches) == 1

    def test_and_constraint(self, sample_conllu):
        """& combines multiple constraints."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { V [upos="VERB" & lemma="help"]; }'))
        assert len(matches) == 1

    def test_negated_constraint(self, sample_conllu):
        """!= negates a constraint."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { W [upos!="VERB"]; }'))
        assert len(matches) == 4  # He, us, to, .


# ==============================================================================
# Edge Type Tests
# ==============================================================================


class TestEdgeTypes:
    """Tests for different edge constraint types."""

    def test_labeled_edge(self, sample_conllu):
        """Labeled edge constraint."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search("MATCH { V []; N []; V -[nsubj]-> N; }"))
        assert len(matches) == 1

    def test_unlabeled_edge(self, sample_conllu):
        """Unlabeled edge constraint."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { V [upos="VERB"]; W []; V -> W; }'))
        # helped has 4 children: He, us, win, .
        assert len(matches) >= 4

    def test_precedence(self, sample_conllu):
        """<< precedence constraint."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { A [form="He"]; B [form="win"]; A << B; }'))
        assert len(matches) == 1

    def test_immediate_precedence(self, sample_conllu):
        """< immediate precedence constraint."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        matches = list(tb.search('MATCH { A [form="to"]; B [form="win"]; A < B; }'))
        assert len(matches) == 1


# ==============================================================================
# EXCEPT and OPTIONAL Tests - Python API
# ==============================================================================


class TestExceptOptional:
    """Tests for EXCEPT and OPTIONAL - focus on Python API, not algorithm."""

    def test_except_basic(self):
        """EXCEPT block filters matches."""
        conllu = """1\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_
2\trunning\trun\tVERB\tVBG\t_\t1\txcomp\t_\t_
3\tquickly\tquickly\tADV\tRB\t_\t2\tadvmod\t_\t_

"""
        tb = treesearch.Treebank.from_string(conllu)
        # Without EXCEPT: 2 verbs
        assert len(list(tb.search('MATCH { V [upos="VERB"]; }'))) == 2
        # With EXCEPT: only verb without advmod child
        matches = list(
            tb.search("""
            MATCH { V [upos="VERB"]; }
            EXCEPT { A []; V -[advmod]-> A; }
        """)
        )
        assert len(matches) == 1

    def test_optional_binds_when_present(self):
        """OPTIONAL variable bound when match exists."""
        conllu = """1\tJohn\tJohn\tPROPN\tNNP\t_\t2\tnsubj\t_\t_
2\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_

"""
        tb = treesearch.Treebank.from_string(conllu)
        matches = list(
            tb.search("""
            MATCH { V [upos="VERB"]; }
            OPTIONAL { S []; V -[nsubj]-> S; }
        """)
        )
        assert len(matches) == 1
        _, match = matches[0]
        assert "S" in match

    def test_optional_absent_when_no_match(self):
        """OPTIONAL variable absent when no match."""
        conllu = """1\tsaw\tsee\tVERB\tVBD\t_\t0\troot\t_\t_

"""
        tb = treesearch.Treebank.from_string(conllu)
        matches = list(
            tb.search("""
            MATCH { V [upos="VERB"]; }
            OPTIONAL { S []; V -[nsubj]-> S; }
        """)
        )
        assert len(matches) == 1
        _, match = matches[0]
        assert "S" not in match


# ==============================================================================
# Edge Cases
# ==============================================================================


class TestEdgeCases:
    """Edge cases and boundary conditions."""

    def test_empty_tree(self):
        """Empty CoNLL-U produces no trees."""
        trees = list(treesearch.Treebank.from_string("").trees())
        assert len(trees) == 0

    def test_single_word_tree(self):
        """Single word tree works correctly."""
        conllu = "1\tword\tword\tNOUN\tNN\t_\t0\troot\t_\t_\n\n"
        trees = list(treesearch.Treebank.from_string(conllu).trees())
        assert len(trees) == 1
        assert len(trees[0]) == 1

    def test_tree_with_no_sentence_text(self):
        """Tree without # text annotation has None sentence_text."""
        conllu = "1\tword\tword\tNOUN\tNN\t_\t0\troot\t_\t_\n\n"
        tree = list(treesearch.Treebank.from_string(conllu).trees())[0]
        assert tree.sentence_text is None

    def test_empty_query_match_block(self):
        """Empty MATCH {} block is valid."""
        pattern = treesearch.compile_query("MATCH { }")
        assert pattern is not None

    def test_unicode_in_form(self):
        """Unicode characters in form work correctly."""
        conllu = "1\t日本語\t日本語\tNOUN\tNN\t_\t0\troot\t_\t_\n\n"
        tree = list(treesearch.Treebank.from_string(conllu).trees())[0]
        assert tree.word(0).form == "日本語"

    def test_treebank_reusable(self, sample_conllu):
        """Treebank can be iterated multiple times."""
        tb = treesearch.Treebank.from_string(sample_conllu)
        trees1 = list(tb.trees())
        trees2 = list(tb.trees())
        assert len(trees1) == len(trees2)


# ==============================================================================
# Integration Tests
# ==============================================================================


class TestIntegration:
    """End-to-end integration tests."""

    def test_full_workflow(self, temp_conllu_file):
        """Complete workflow from file to results."""
        # Load and search
        pattern = treesearch.compile_query("""
            MATCH {
                Verb [upos="VERB" & lemma="help"];
                Obj [upos="PRON"];
                Verb -[obj]-> Obj;
            }
        """)
        results = list(treesearch.search(temp_conllu_file, pattern))
        assert len(results) == 1

        # Extract match
        tree, match = results[0]
        verb = tree.word(match["Verb"])
        obj = tree.word(match["Obj"])

        # Verify
        assert verb.lemma == "help"
        assert obj.form == "us"
        assert obj.parent().id == verb.id

    def test_glob_workflow(self, temp_multi_files):
        """Multi-file glob workflow."""
        tmpdir, _ = temp_multi_files
        pattern = treesearch.compile_query("""
            MATCH {
                Noun [upos="NOUN"];
                Verb [upos="VERB"];
                Verb -[nsubj]-> Noun;
            }
        """)
        results = list(treesearch.load(f"{tmpdir}/*.conllu").search(pattern))
        assert len(results) == 6  # 2 trees × 3 files


# ==============================================================================
# Visualization Tests
# ==============================================================================


class TestVisualization:
    """Tests for visualization functions."""

    def test_to_displacy_structure(self, sample_conllu):
        """to_displacy returns correct structure."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        data = treesearch.to_displacy(tree)

        assert "words" in data
        assert "arcs" in data
        assert isinstance(data["words"], list)
        assert isinstance(data["arcs"], list)

    def test_to_displacy_words(self, sample_conllu):
        """to_displacy words have text and tag."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        data = treesearch.to_displacy(tree)

        assert len(data["words"]) == 6
        assert data["words"][0] == {"text": "He", "tag": "PRON"}
        assert data["words"][1] == {"text": "helped", "tag": "VERB"}

    def test_to_displacy_arcs(self, sample_conllu):
        """to_displacy arcs have start, end, label, dir."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        data = treesearch.to_displacy(tree)

        # Check that arcs exist and have correct structure
        assert len(data["arcs"]) == 5  # 6 words, 1 root (no arc)
        for arc in data["arcs"]:
            assert "start" in arc
            assert "end" in arc
            assert "label" in arc
            assert "dir" in arc
            assert arc["dir"] in ("left", "right")

    def test_to_displacy_arc_direction(self):
        """to_displacy arc direction is correct."""
        # Tree: helped (root) -> He (nsubj)
        # He (dep) is before helped (head), so arc points left (toward head)
        conllu = """1\tHe\the\tPRON\tPRP\t_\t2\tnsubj\t_\t_
2\thelped\thelp\tVERB\tVBD\t_\t0\troot\t_\t_

"""
        tree = list(treesearch.Treebank.from_string(conllu).trees())[0]
        data = treesearch.to_displacy(tree)

        # He (0) <- helped (1): start=0, end=1, dir=left (pointing toward head)
        nsubj_arc = next(a for a in data["arcs"] if a["label"] == "nsubj")
        assert nsubj_arc["start"] == 0
        assert nsubj_arc["end"] == 1
        assert nsubj_arc["dir"] == "left"

    def test_tree_to_displacy_method(self, sample_conllu):
        """Tree.to_displacy() works as instance method."""
        tree = list(treesearch.Treebank.from_string(sample_conllu).trees())[0]
        data = tree.to_displacy()

        assert "words" in data
        assert "arcs" in data
