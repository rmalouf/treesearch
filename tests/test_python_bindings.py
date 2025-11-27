"""Comprehensive pytest tests for treesearch Python bindings.

This test suite covers all Python API functionality for test-driven development:
- Pattern parsing and creation
- Tree reading from files (single and multiple)
- Word properties and relationships
- Pattern matching and searching
- Multi-file operations with glob patterns
- Error handling
"""

import gzip
import tempfile
from pathlib import Path

import pytest

import treesearch


# Test data fixtures
@pytest.fixture
def sample_conllu():
    """Simple CoNLL-U test data with various dependency relations."""
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
    """More complex CoNLL-U data with metadata."""
    return """# sent_id = 1
# text = The big dog runs in the park.
# source = test
1	The	the	DET	DT	_	3	det	_	_
2	big	big	ADJ	JJ	_	3	amod	_	_
3	dog	dog	NOUN	NN	_	4	nsubj	_	_
4	runs	run	VERB	VBZ	_	0	root	_	_
5	in	in	ADP	IN	_	7	case	_	_
6	the	the	DET	DT	_	7	det	_	_
7	park	park	NOUN	NN	_	4	obl	_	_
8	.	.	PUNCT	.	_	4	punct	_	_

"""


@pytest.fixture
def multi_tree_conllu():
    """CoNLL-U data with multiple trees."""
    return """# text = The dog runs.
1	The	the	DET	DT	_	2	det	_	_
2	dog	dog	NOUN	NN	_	3	nsubj	_	_
3	runs	run	VERB	VBZ	_	0	root	_	_
4	.	.	PUNCT	.	_	3	punct	_	_

# text = Cats sleep.
1	Cats	cat	NOUN	NNS	_	2	nsubj	_	_
2	sleep	sleep	VERB	VBP	_	0	root	_	_
3	.	.	PUNCT	.	_	2	punct	_	_

"""


@pytest.fixture
def temp_conllu_file(sample_conllu, tmp_path):
    """Create a temporary CoNLL-U file."""
    path = tmp_path / "test.conllu"
    path.write_text(sample_conllu)
    return str(path)


@pytest.fixture
def temp_complex_file(complex_conllu, tmp_path):
    """Create a temporary CoNLL-U file with metadata."""
    path = tmp_path / "complex.conllu"
    path.write_text(complex_conllu)
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


# Test Pattern creation and parsing
class TestPattern:
    """Tests for pattern creation and query parsing."""

    def test_parse_simple_query(self):
        """Test parsing a simple single-variable query."""
        pattern = treesearch.parse_query('V [upos="VERB"];')
        assert pattern.n_vars == 1

    def test_parse_multi_variable_query(self):
        """Test parsing a query with multiple variables."""
        pattern = treesearch.parse_query("""
            V1 [upos="VERB"];
            V2 [upos="VERB"];
            V1 -[xcomp]-> V2;
        """)
        assert pattern.n_vars == 2

    def test_parse_complex_query(self):
        """Test parsing a complex query with multiple constraints."""
        pattern = treesearch.parse_query("""
            Verb [upos="VERB"];
            Noun [upos="NOUN"];
            Pron [upos="PRON"];
            Verb -[nsubj]-> Pron;
            Verb -[obj]-> Noun;
        """)
        assert pattern.n_vars == 3

    def test_parse_query_with_lemma(self):
        """Test parsing query with lemma constraint."""
        pattern = treesearch.parse_query('V [lemma="help"];')
        assert pattern.n_vars == 1

    def test_parse_query_with_form(self):
        """Test parsing query with form constraint."""
        pattern = treesearch.parse_query('Word [form="the"];')
        assert pattern.n_vars == 1

    def test_parse_query_multiple_constraints(self):
        """Test parsing query with multiple constraints on single variable."""
        pattern = treesearch.parse_query('V [upos="VERB", lemma="help"];')
        assert pattern.n_vars == 1

    def test_pattern_repr(self):
        """Test pattern string representation."""
        pattern = treesearch.parse_query('V [upos="VERB"];')
        repr_str = repr(pattern)
        assert "Pattern" in repr_str
        assert "1 vars" in repr_str or "var" in repr_str

    def test_invalid_query_raises_error(self):
        """Test that invalid queries raise errors."""
        with pytest.raises(Exception):  # Should raise PyValueError
            treesearch.parse_query("INVALID SYNTAX [[[")

    def test_empty_query(self):
        """Test that empty queries create a pattern with 0 variables."""
        pattern = treesearch.parse_query("")
        assert pattern.n_vars == 0


# Test Tree reading and iteration
class TestTreeReading:
    """Tests for reading trees from CoNLL-U data."""

    def test_read_tree_from_file(self, temp_conllu_file):
        """Test reading a tree from a CoNLL-U file."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        assert len(trees) == 1
        tree = trees[0]
        assert len(tree) == 6  # 6 words

    def test_read_tree_from_gzip(self, temp_gzip_file):
        """Test reading a tree from a gzipped CoNLL-U file."""
        trees = list(treesearch.read_trees(temp_gzip_file))
        assert len(trees) == 1
        tree = trees[0]
        assert len(tree) == 6

    def test_read_multiple_trees(self, multi_tree_conllu, tmp_path):
        """Test reading multiple trees from a single file."""
        path = tmp_path / "multi.conllu"
        path.write_text(multi_tree_conllu)
        trees = list(treesearch.read_trees(str(path)))
        assert len(trees) == 2

    def test_tree_iterator_is_iterator(self, temp_conllu_file):
        """Test that read_trees returns a proper iterator."""
        tree_iter = treesearch.read_trees(temp_conllu_file)
        assert hasattr(tree_iter, "__iter__")
        assert hasattr(tree_iter, "__next__")

    def test_tree_properties(self, temp_conllu_file):
        """Test basic tree properties."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        tree = trees[0]

        assert tree.sentence_text == "He helped us to win."
        assert len(tree) == 6
        assert "Tree" in repr(tree)
        assert "6 words" in repr(tree)

    def test_tree_metadata(self, temp_complex_file):
        """Test tree metadata property."""
        trees = list(treesearch.read_trees(temp_complex_file))
        tree = trees[0]

        metadata = tree.metadata
        assert isinstance(metadata, dict)
        assert "sent_id" in metadata
        assert metadata["sent_id"] == "1"
        assert "source" in metadata
        assert metadata["source"] == "test"

    def test_tree_get_word(self, temp_conllu_file):
        """Test getting words from a tree by ID."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        tree = trees[0]

        # Get first word (id=0)
        word = tree.get_word(0)
        assert word is not None
        assert word.form == "He"

        # Get last word
        last_word = tree.get_word(5)
        assert last_word is not None
        assert last_word.form == "."

        # Test out of bounds
        assert tree.get_word(999) is None

    def test_read_nonexistent_file(self):
        """Test reading a file that doesn't exist raises error."""
        with pytest.raises(Exception):  # Should raise PyValueError
            list(treesearch.read_trees("/nonexistent/path/file.conllu"))


# Test Word properties and methods
class TestWord:
    """Tests for Word class properties and methods."""

    @pytest.fixture
    def sample_tree(self, temp_conllu_file):
        """Get a sample tree for word tests."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        return trees[0]

    def test_word_basic_properties(self, sample_tree):
        """Test basic word properties (id, form, lemma, pos, deprel)."""
        word = sample_tree.get_word(1)  # "helped"
        assert word.id == 1
        assert word.token_id == 2  # 1-based in CoNLL-U
        assert word.form == "helped"
        assert word.lemma == "help"
        assert word.pos == "VERB"
        assert word.deprel == "root"

    def test_word_xpos(self, sample_tree):
        """Test xpos property (language-specific POS tag)."""
        word = sample_tree.get_word(1)
        xpos = word.xpos
        # xpos might be None if not provided (underscore in CoNLL-U)
        assert xpos is None or isinstance(xpos, str)

    def test_word_head_property(self, sample_tree):
        """Test word head property."""
        word = sample_tree.get_word(0)  # "He"
        assert word.head == 1  # Head is "helped" (id=1)

        root = sample_tree.get_word(1)  # "helped"
        assert root.head is None  # Root has head=0 in CoNLL-U, None in API

    def test_word_parent_method(self, sample_tree):
        """Test getting parent word."""
        word = sample_tree.get_word(0)  # "He"
        parent = word.parent()
        assert parent is not None
        assert parent.form == "helped"

        # Root word has no parent
        root = sample_tree.get_word(1)
        assert root.parent() is None

    def test_word_children_method(self, sample_tree):
        """Test getting all children words."""
        verb = sample_tree.get_word(1)  # "helped"
        children = verb.children()
        assert len(children) > 0

        # Check that children are Word objects
        child_forms = [c.form for c in children]
        assert "He" in child_forms
        assert "us" in child_forms
        assert "win" in child_forms
        assert "." in child_forms

    def test_word_children_ids_property(self, sample_tree):
        """Test getting children IDs."""
        verb = sample_tree.get_word(1)  # "helped"
        child_ids = verb.children_ids
        assert isinstance(child_ids, list)
        assert len(child_ids) == 4  # He, us, win, .
        assert all(isinstance(cid, int) for cid in child_ids)

    def test_word_children_by_deprel(self, sample_tree):
        """Test getting children filtered by dependency relation."""
        verb = sample_tree.get_word(1)  # "helped"

        # Get nsubj children (should be "He")
        nsubj_children = verb.children_by_deprel("nsubj")
        assert len(nsubj_children) == 1
        assert nsubj_children[0].form == "He"

        # Get obj children (should be "us")
        obj_children = verb.children_by_deprel("obj")
        assert len(obj_children) == 1
        assert obj_children[0].form == "us"

        # Get xcomp children (should be "win")
        xcomp_children = verb.children_by_deprel("xcomp")
        assert len(xcomp_children) == 1
        assert xcomp_children[0].form == "win"

        # Get punct children
        punct_children = verb.children_by_deprel("punct")
        assert len(punct_children) == 1
        assert punct_children[0].form == "."

        # Get non-existent relation
        adv_children = verb.children_by_deprel("advmod")
        assert len(adv_children) == 0

    def test_word_repr(self, sample_tree):
        """Test word string representation."""
        word = sample_tree.get_word(1)
        repr_str = repr(word)
        assert "Word" in repr_str
        assert "helped" in repr_str
        assert "help" in repr_str
        assert "VERB" in repr_str


# Test find_path functionality
class TestFindPath:
    """Tests for Tree.find_path method."""

    @pytest.fixture
    def complex_tree(self, temp_complex_file):
        """Get a more complex tree for path finding tests."""
        trees = list(treesearch.read_trees(temp_complex_file))
        return trees[0]

    def test_find_path_direct_child(self, complex_tree):
        """Test finding path from parent to direct child."""
        runs = complex_tree.get_word(3)  # "runs"
        dog = complex_tree.get_word(2)  # "dog"

        path = complex_tree.find_path(runs, dog)
        assert path is not None
        assert len(path) == 2
        assert path[0].form == "runs"
        assert path[1].form == "dog"

    def test_find_path_multi_level(self, complex_tree):
        """Test finding path through multiple levels."""
        runs = complex_tree.get_word(3)  # "runs"
        big = complex_tree.get_word(1)  # "big"

        path = complex_tree.find_path(runs, big)
        assert path is not None
        assert len(path) == 3
        assert path[0].form == "runs"
        assert path[1].form == "dog"
        assert path[2].form == "big"

    def test_find_path_different_branch(self, complex_tree):
        """Test finding path to different branch."""
        runs = complex_tree.get_word(3)  # "runs"
        park = complex_tree.get_word(6)  # "park"

        path = complex_tree.find_path(runs, park)
        assert path is not None
        assert len(path) == 2
        assert path[0].form == "runs"
        assert path[1].form == "park"

    def test_find_path_no_path_siblings(self, complex_tree):
        """Test that no path exists between siblings."""
        dog = complex_tree.get_word(2)  # "dog" (child of runs)
        park = complex_tree.get_word(6)  # "park" (child of runs)

        path = complex_tree.find_path(dog, park)
        assert path is None

    def test_find_path_no_path_reverse(self, complex_tree):
        """Test that no path exists in reverse direction (child to parent)."""
        dog = complex_tree.get_word(2)  # "dog"
        runs = complex_tree.get_word(3)  # "runs"

        path = complex_tree.find_path(dog, runs)
        assert path is None

    def test_find_path_same_node(self, complex_tree):
        """Test that no path exists for same node."""
        runs = complex_tree.get_word(3)

        path = complex_tree.find_path(runs, runs)
        assert path is None


# Test searching with patterns
class TestSearch:
    """Tests for pattern matching and search functionality."""

    @pytest.fixture
    def sample_tree(self, temp_conllu_file):
        """Get a sample tree for search tests."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        return trees[0]

    def test_search_simple_pattern(self, sample_tree):
        """Test searching a tree with a simple pattern."""
        pattern = treesearch.parse_query('V [upos="VERB"];')
        matches = treesearch.search(sample_tree, pattern)

        # Convert to list to check length
        matches = list(matches)
        assert len(matches) == 2  # "helped" and "win"

        # Each match should be a dict mapping variable names to word IDs
        for match in matches:
            assert isinstance(match, dict)
            assert "V" in match
            assert isinstance(match["V"], int)

    def test_search_with_lemma_constraint(self, sample_tree):
        """Test searching with lemma constraint."""
        pattern = treesearch.parse_query('V [lemma="help"];')
        matches = list(treesearch.search(sample_tree, pattern))

        assert len(matches) == 1
        word_id = matches[0]["V"]
        word = sample_tree.get_word(word_id)
        assert word.lemma == "help"
        assert word.form == "helped"

    def test_search_with_form_constraint(self, sample_tree):
        """Test searching with form constraint."""
        pattern = treesearch.parse_query('Word [form="He"];')
        matches = list(treesearch.search(sample_tree, pattern))

        assert len(matches) == 1
        word_id = matches[0]["Word"]
        word = sample_tree.get_word(word_id)
        assert word.form == "He"

    def test_search_with_edge_constraint(self, sample_tree):
        """Test searching with edge constraints."""
        pattern = treesearch.parse_query("""
            V1 [upos="VERB"];
            V2 [upos="VERB"];
            V1 -[xcomp]-> V2;
        """)
        matches = list(treesearch.search(sample_tree, pattern))

        assert len(matches) == 1
        match = matches[0]
        assert "V1" in match
        assert "V2" in match

        # Verify the match
        v1 = sample_tree.get_word(match["V1"])
        v2 = sample_tree.get_word(match["V2"])
        assert v1.form == "helped"
        assert v2.form == "win"

    def test_search_multiple_edges(self, sample_tree):
        """Test searching with multiple edge constraints."""
        pattern = treesearch.parse_query("""
            Verb [upos="VERB"];
            Pron [upos="PRON"];
            Verb -[nsubj]-> Pron;
        """)
        matches = list(treesearch.search(sample_tree, pattern))

        # Should find "helped" with nsubj "He"
        assert len(matches) == 1
        match = matches[0]

        verb = sample_tree.get_word(match["Verb"])
        pron = sample_tree.get_word(match["Pron"])
        assert verb.form == "helped"
        assert pron.form == "He"

    def test_search_no_matches(self, sample_tree):
        """Test searching for pattern with no matches."""
        pattern = treesearch.parse_query('N [upos="NOUN"];')
        matches = list(treesearch.search(sample_tree, pattern))
        assert len(matches) == 0

    def test_search_multiple_constraints(self, sample_tree):
        """Test searching with multiple constraints on one variable."""
        pattern = treesearch.parse_query('V [upos="VERB", lemma="help"];')
        matches = list(treesearch.search(sample_tree, pattern))

        assert len(matches) == 1
        word = sample_tree.get_word(matches[0]["V"])
        assert word.pos == "VERB"
        assert word.lemma == "help"


# Test file searching
class TestFileSearch:
    """Tests for searching CoNLL-U files."""

    def test_search_file(self, temp_conllu_file):
        """Test searching a single file."""
        pattern = treesearch.parse_query('V [upos="VERB"];')
        results = list(treesearch.search_file(temp_conllu_file, pattern))

        assert len(results) > 0

        # Each result should be (tree, match)
        for tree, match in results:
            assert hasattr(tree, "get_word")
            assert isinstance(match, dict)
            assert "V" in match

    def test_search_file_complex_pattern(self, temp_conllu_file):
        """Test searching a file with a complex pattern."""
        pattern = treesearch.parse_query("""
            Verb [upos="VERB"];
            Pron [upos="PRON"];
            Verb -[nsubj]-> Pron;
        """)
        results = list(treesearch.search_file(temp_conllu_file, pattern))

        # Should find "helped" with nsubj "He"
        assert len(results) == 1
        tree, match = results[0]

        verb = tree.get_word(match["Verb"])
        pron = tree.get_word(match["Pron"])
        assert verb.form == "helped"
        assert pron.form == "He"

    def test_search_file_multiple_matches(self, multi_tree_conllu, tmp_path):
        """Test searching file with multiple trees."""
        path = tmp_path / "multi.conllu"
        path.write_text(multi_tree_conllu)

        pattern = treesearch.parse_query('V [upos="VERB"];')
        results = list(treesearch.search_file(str(path), pattern))

        # Should find 1 VERB in each tree (2 total)
        assert len(results) == 2

    def test_search_gzipped_file(self, temp_gzip_file):
        """Test searching a gzipped CoNLL-U file."""
        pattern = treesearch.parse_query('V [upos="VERB"];')
        results = list(treesearch.search_file(temp_gzip_file, pattern))

        assert len(results) == 2  # "helped" and "win"


# Test multi-file operations
class TestMultiFile:
    """Tests for multi-file glob operations."""

    def test_read_trees_glob_parallel(self, temp_multi_files):
        """Test reading trees from multiple files using glob (parallel)."""
        tmpdir, files = temp_multi_files
        pattern = f"{tmpdir}/*.conllu"

        trees = list(treesearch.read_trees_glob(pattern, parallel=True))

        # Should have 6 trees (2 trees per file × 3 files)
        assert len(trees) == 6

    def test_read_trees_glob_sequential(self, temp_multi_files):
        """Test reading trees from multiple files sequentially."""
        tmpdir, files = temp_multi_files
        pattern = f"{tmpdir}/*.conllu"

        trees = list(treesearch.read_trees_glob(pattern, parallel=False))

        # Should have 6 trees (2 trees per file × 3 files)
        assert len(trees) == 6

    def test_read_trees_glob_default_parallel(self, temp_multi_files):
        """Test that parallel=True is the default."""
        tmpdir, files = temp_multi_files
        pattern = f"{tmpdir}/*.conllu"

        trees = list(treesearch.read_trees_glob(pattern))
        assert len(trees) == 6

    def test_search_files_glob_parallel(self, temp_multi_files):
        """Test searching multiple files using glob (parallel)."""
        tmpdir, files = temp_multi_files
        pattern = treesearch.parse_query('V [upos="VERB"];')
        glob_pattern = f"{tmpdir}/*.conllu"

        results = list(treesearch.search_files(glob_pattern, pattern, parallel=True))

        # Each file has 2 trees, each with 1 VERB
        # So we should get 6 matches (2 × 3 files)
        assert len(results) == 6

    def test_search_files_sequential(self, temp_multi_files):
        """Test searching multiple files sequentially."""
        tmpdir, files = temp_multi_files
        pattern = treesearch.parse_query('V [upos="VERB"];')
        glob_pattern = f"{tmpdir}/*.conllu"

        results = list(treesearch.search_files(glob_pattern, pattern, parallel=False))

        # Should get same results as parallel
        assert len(results) == 6

    def test_search_files_default_parallel(self, temp_multi_files):
        """Test that parallel=True is the default for search_files."""
        tmpdir, files = temp_multi_files
        pattern = treesearch.parse_query('V [upos="VERB"];')
        glob_pattern = f"{tmpdir}/*.conllu"

        results = list(treesearch.search_files(glob_pattern, pattern))
        assert len(results) == 6

    def test_glob_no_matches(self, tmp_path):
        """Test glob pattern that matches no files."""
        pattern = treesearch.parse_query('V [upos="VERB"];')
        glob_pattern = f"{tmp_path}/nonexistent/*.conllu"

        results = list(treesearch.search_files(glob_pattern, pattern))
        assert len(results) == 0


# Integration tests
class TestIntegration:
    """End-to-end integration tests for complete workflows."""

    def test_full_workflow(self, temp_conllu_file):
        """Test a complete workflow from query to results."""
        # 1. Parse a query
        query = """
            Verb [upos="VERB", lemma="help"];
            Noun [upos="PRON"];
            Verb -[obj]-> Noun;
        """
        pattern = treesearch.parse_query(query)
        assert pattern.n_vars == 2

        # 2. Search a file
        results = list(treesearch.search_file(temp_conllu_file, pattern))
        assert len(results) == 1

        # 3. Extract and verify results
        tree, match = results[0]
        verb = tree.get_word(match["Verb"])
        noun = tree.get_word(match["Noun"])

        assert verb.lemma == "help"
        assert verb.pos == "VERB"
        assert noun.pos == "PRON"
        assert noun.form == "us"

        # 4. Verify tree structure
        assert verb.head is None  # Root
        assert noun.head == verb.id  # noun's head is the verb

        # 5. Verify parent/child relationships
        assert noun.parent().id == verb.id
        assert noun.id in [c.id for c in verb.children()]

    def test_workflow_with_children_by_deprel(self, temp_conllu_file):
        """Test workflow using children_by_deprel API."""
        # Read tree
        trees = list(treesearch.read_trees(temp_conllu_file))
        tree = trees[0]

        # Find the verb "helped"
        pattern = treesearch.parse_query('V [lemma="help"];')
        matches = list(treesearch.search(tree, pattern))
        verb = tree.get_word(matches[0]["V"])

        # Use children_by_deprel to find various dependents
        nsubj = verb.children_by_deprel("nsubj")
        assert len(nsubj) == 1
        assert nsubj[0].form == "He"

        obj = verb.children_by_deprel("obj")
        assert len(obj) == 1
        assert obj[0].form == "us"

        xcomp = verb.children_by_deprel("xcomp")
        assert len(xcomp) == 1
        assert xcomp[0].form == "win"

    def test_workflow_with_find_path(self, temp_complex_file):
        """Test workflow using find_path."""
        # Read tree
        trees = list(treesearch.read_trees(temp_complex_file))
        tree = trees[0]

        # Find root verb and a deeply nested word
        pattern = treesearch.parse_query('V [upos="VERB"];')
        matches = list(treesearch.search(tree, pattern))
        root = tree.get_word(matches[0]["V"])

        # Find a determiner deep in the tree
        det_pattern = treesearch.parse_query('Det [form="big"];')
        det_matches = list(treesearch.search(tree, det_pattern))
        det = tree.get_word(det_matches[0]["Det"])

        # Find path from root to determiner
        path = tree.find_path(root, det)
        assert path is not None
        assert path[0].id == root.id
        assert path[-1].id == det.id

    def test_workflow_multi_file_glob(self, temp_multi_files):
        """Test complete workflow with multiple files."""
        tmpdir, files = temp_multi_files

        # Parse a query
        pattern = treesearch.parse_query("""
            Noun [upos="NOUN"];
            Verb [upos="VERB"];
            Verb -[nsubj]-> Noun;
        """)

        # Search all files
        results = list(treesearch.search_files(f"{tmpdir}/*.conllu", pattern))

        # Should find pattern in each tree
        assert len(results) == 6  # 2 trees × 3 files

        # Verify each result
        for tree, match in results:
            noun = tree.get_word(match["Noun"])
            verb = tree.get_word(match["Verb"])

            # Verify edge
            parent = noun.parent()
            assert parent is not None
            assert parent.id == verb.id
            assert noun.deprel == "nsubj"
