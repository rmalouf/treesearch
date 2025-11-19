"""Comprehensive pytest tests for treesearch Python bindings."""

import pytest
import tempfile
from pathlib import Path

import treesearch


# Test data fixtures
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
def temp_conllu_file(sample_conllu):
    """Create a temporary CoNLL-U file."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.conllu', delete=False) as f:
        f.write(sample_conllu)
        path = f.name
    yield path
    Path(path).unlink()


@pytest.fixture
def temp_multi_files(multi_tree_conllu):
    """Create multiple temporary CoNLL-U files."""
    files = []
    with tempfile.TemporaryDirectory() as tmpdir:
        for i in range(3):
            path = Path(tmpdir) / f"test_{i}.conllu"
            path.write_text(multi_tree_conllu)
            files.append(str(path))
        yield tmpdir, files


# Test Pattern creation and parsing
class TestPattern:
    """Tests for pattern creation and query parsing."""

    def test_parse_simple_query(self):
        """Test parsing a simple query."""
        pattern = treesearch.parse_query('V [pos="VERB"];')
        assert pattern.n_vars == 1

    def test_parse_multi_variable_query(self):
        """Test parsing a query with multiple variables."""
        pattern = treesearch.parse_query('''
            V1 [pos="VERB"];
            V2 [pos="VERB"];
            V1 -[xcomp]-> V2;
        ''')
        assert pattern.n_vars == 2

    def test_parse_complex_query(self):
        """Test parsing a complex query."""
        pattern = treesearch.parse_query('''
            Verb [pos="VERB"];
            Noun [pos="NOUN"];
            Pron [pos="PRON"];
            Verb -[nsubj]-> Pron;
            Verb -[obj]-> Noun;
        ''')
        assert pattern.n_vars == 3

    def test_pattern_repr(self):
        """Test pattern string representation."""
        pattern = treesearch.parse_query('V [pos="VERB"];')
        repr_str = repr(pattern)
        assert "Pattern" in repr_str
        assert "vars" in repr_str

    def test_invalid_query(self):
        """Test that invalid queries raise errors."""
        with pytest.raises(Exception):  # Should raise PyValueError
            treesearch.parse_query('INVALID SYNTAX [[[')


# Test Tree reading and iteration
class TestTreeReading:
    """Tests for reading trees from CoNLL-U data."""

    def test_read_tree_from_file(self, temp_conllu_file):
        """Test reading a tree from a file."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        assert len(trees) == 1
        tree = trees[0]
        assert len(tree) == 6  # 6 words

    def test_tree_properties(self, temp_conllu_file):
        """Test tree properties."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        tree = trees[0]

        assert tree.sentence_text == "He helped us to win."
        assert len(tree) == 6
        assert "Tree" in repr(tree)
        assert "6 words" in repr(tree)

    def test_tree_get_word(self, temp_conllu_file):
        """Test getting words from a tree."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        tree = trees[0]

        # Get first word (id=0)
        word = tree.get_word(0)
        assert word is not None
        assert word.form == "He"

        # Test out of bounds
        assert tree.get_word(999) is None


# Test Word properties and methods
class TestWord:
    """Tests for Word class."""

    @pytest.fixture
    def sample_tree(self, temp_conllu_file):
        """Get a sample tree for word tests."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        return trees[0]

    def test_word_basic_properties(self, sample_tree):
        """Test basic word properties."""
        word = sample_tree.get_word(1)  # "helped"
        assert word.id == 1
        assert word.token_id == 2  # 1-based in CoNLL-U
        assert word.form == "helped"
        assert word.lemma == "help"
        assert word.pos == "VERB"
        assert word.deprel == "root"

    def test_word_xpos(self, sample_tree):
        """Test xpos property (optional)."""
        word = sample_tree.get_word(1)
        xpos = word.xpos
        # xpos might be None or a value depending on the data
        assert xpos is None or isinstance(xpos, str)

    def test_word_head(self, sample_tree):
        """Test word head property."""
        word = sample_tree.get_word(0)  # "He"
        assert word.head == 1  # Head is "helped" (id=1)

        root = sample_tree.get_word(1)  # "helped"
        assert root.head is None  # Root has no head

    def test_word_parent(self, sample_tree):
        """Test getting parent word."""
        word = sample_tree.get_word(0)  # "He"
        parent = word.parent()
        assert parent is not None
        assert parent.form == "helped"

        # Root word has no parent
        root = sample_tree.get_word(1)
        assert root.parent() is None

    def test_word_children(self, sample_tree):
        """Test getting children words."""
        verb = sample_tree.get_word(1)  # "helped"
        children = verb.children()
        assert len(children) > 0

        # Check that children are Word objects
        for child in children:
            assert hasattr(child, 'form')
            assert hasattr(child, 'lemma')

    def test_word_children_ids(self, sample_tree):
        """Test getting children IDs."""
        verb = sample_tree.get_word(1)  # "helped"
        child_ids = verb.children_ids
        assert isinstance(child_ids, list)
        assert len(child_ids) > 0
        assert all(isinstance(cid, int) for cid in child_ids)

    def test_word_children_by_deprel(self, sample_tree):
        """Test getting children by dependency relation."""
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


# Test searching
class TestSearch:
    """Tests for search functionality."""

    @pytest.fixture
    def sample_tree(self, temp_conllu_file):
        """Get a sample tree for search tests."""
        trees = list(treesearch.read_trees(temp_conllu_file))
        return trees[0]

    def test_search_simple_pattern(self, sample_tree):
        """Test searching a tree with a simple pattern."""
        pattern = treesearch.parse_query('V [pos="VERB"];')
        matches = treesearch.search(sample_tree, pattern)

        # Convert to list to check length
        matches = list(matches)
        assert len(matches) == 2  # "helped" and "win"

        # Each match should be a list of word IDs
        for match in matches:
            assert isinstance(match, list)
            assert len(match) == 1  # One variable

    def test_search_with_edge_constraint(self, sample_tree):
        """Test searching with edge constraints."""
        pattern = treesearch.parse_query('''
            V1 [pos="VERB"];
            V2 [pos="VERB"];
            V1 -[xcomp]-> V2;
        ''')
        matches = list(treesearch.search(sample_tree, pattern))

        assert len(matches) == 1
        match = matches[0]
        assert len(match) == 2

        # Verify the match
        v1 = sample_tree.get_word(match[0])
        v2 = sample_tree.get_word(match[1])
        assert v1.form == "helped"
        assert v2.form == "win"

    def test_search_with_lemma(self, sample_tree):
        """Test searching with lemma constraint."""
        pattern = treesearch.parse_query('V [lemma="help"];')
        matches = list(treesearch.search(sample_tree, pattern))

        assert len(matches) == 1
        word = sample_tree.get_word(matches[0][0])
        assert word.lemma == "help"
        assert word.form == "helped"


# Test file searching
class TestFileSearch:
    """Tests for searching files."""

    def test_search_file(self, temp_conllu_file):
        """Test searching a single file."""
        pattern = treesearch.parse_query('V [pos="VERB"];')
        results = list(treesearch.search_file(temp_conllu_file, pattern))

        assert len(results) > 0

        # Each result should be (tree, match)
        for tree, match in results:
            assert hasattr(tree, 'get_word')
            assert isinstance(match, list)
            assert len(match) == 1  # One variable in pattern

    def test_search_file_complex_pattern(self, temp_conllu_file):
        """Test searching a file with a complex pattern."""
        pattern = treesearch.parse_query('''
            Verb [pos="VERB"];
            Pron [pos="PRON"];
            Verb -[nsubj]-> Pron;
        ''')
        results = list(treesearch.search_file(temp_conllu_file, pattern))

        # Should find "helped" with nsubj "He"
        assert len(results) == 1
        tree, match = results[0]

        verb = tree.get_word(match[0])
        pron = tree.get_word(match[1])
        assert verb.form == "helped"
        assert pron.form == "He"


# Test multi-file operations
class TestMultiFile:
    """Tests for multi-file operations."""

    def test_read_trees_glob(self, temp_multi_files):
        """Test reading trees from multiple files using glob."""
        tmpdir, files = temp_multi_files
        pattern = f"{tmpdir}/*.conllu"

        trees = list(treesearch.read_trees_glob(pattern))

        # Should have 6 trees (2 trees per file × 3 files)
        assert len(trees) == 6

    def test_read_trees_glob_sequential(self, temp_multi_files):
        """Test reading trees sequentially (no parallel)."""
        tmpdir, files = temp_multi_files
        pattern = f"{tmpdir}/*.conllu"

        trees = list(treesearch.read_trees_glob(pattern, parallel=False))

        # Should have 6 trees (2 trees per file × 3 files)
        assert len(trees) == 6

    def test_search_files_glob(self, temp_multi_files):
        """Test searching multiple files using glob."""
        tmpdir, files = temp_multi_files
        pattern = treesearch.parse_query('V [pos="VERB"];')
        glob_pattern = f"{tmpdir}/*.conllu"

        results = list(treesearch.search_files(glob_pattern, pattern))

        # Each file has 2 trees, each with 1 VERB
        # So we should get 6 matches (2 × 3 files)
        assert len(results) == 6

    def test_search_files_sequential(self, temp_multi_files):
        """Test searching multiple files sequentially."""
        tmpdir, files = temp_multi_files
        pattern = treesearch.parse_query('V [pos="VERB"];')
        glob_pattern = f"{tmpdir}/*.conllu"

        results = list(treesearch.search_files(glob_pattern, pattern, parallel=False))

        # Should get same results as parallel
        assert len(results) == 6


# Test error handling
class TestErrorHandling:
    """Tests for error handling."""

    def test_read_nonexistent_file(self):
        """Test reading a file that doesn't exist."""
        with pytest.raises(Exception):
            list(treesearch.read_trees("/nonexistent/path/file.conllu"))

    def test_invalid_glob_pattern(self):
        """Test using an invalid glob pattern."""
        pattern = treesearch.parse_query('V [pos="VERB"];')
        # This should handle the error gracefully or raise an exception
        # depending on implementation
        try:
            results = list(treesearch.search_files("/nonexistent/**/*.conllu", pattern))
            assert len(results) == 0  # No files found
        except Exception:
            pass  # Also acceptable to raise an error


# Integration test
class TestIntegration:
    """End-to-end integration tests."""

    def test_full_workflow(self, temp_conllu_file):
        """Test a complete workflow from query to results."""
        # 1. Parse a query
        query = '''
            Verb [pos="VERB", lemma="help"];
            Noun [pos="PRON"];
            Verb -[obj]-> Noun;
        '''
        pattern = treesearch.parse_query(query)
        assert pattern.n_vars == 2

        # 2. Search a file
        results = list(treesearch.search_file(temp_conllu_file, pattern))
        assert len(results) == 1

        # 3. Extract and verify results
        tree, match = results[0]
        verb = tree.get_word(match[0])
        noun = tree.get_word(match[1])

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
