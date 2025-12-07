"""Treesearch: High-performance dependency tree pattern matching.

A toolkit for querying linguistic dependency parses at scale.

Example usage:
    >>> import treesearch
    >>>
    >>> # Parse a query into a pattern
    >>> pattern = treesearch.parse_query('''
    ...     Verb [pos="VERB"];
    ...     Noun [pos="NOUN"];
    ...     Verb -[nsubj]-> Noun;
    ... ''')
    >>>
    >>> # Search a single file
    >>> for tree, match in treesearch.search_file("corpus.conllu", pattern):
    ...     verb_id = match[0]  # First variable
    ...     noun_id = match[1]  # Second variable
    ...     verb = tree.get_word(verb_id)
    ...     noun = tree.get_word(noun_id)
    ...     print(f"{verb.form} -> {noun.form}")
    >>>
    >>> # Search multiple files in parallel
    >>> for tree, match in treesearch.search_files("data/*.conllu", pattern):
    ...     # Process matches from all files
    ...     pass
"""

__version__ = "0.1.0"

# Import native extension
try:
    from .treesearch import (
        Tree,
        Word,
        Pattern,
        Treebank,
        TreeIterator,
        MatchIterator,
        parse_query,
        search,
        read_trees,
        search_file,
        read_trees_glob,
        search_files,
    )
except ImportError:
    # Provide helpful error message if native extension not built
    import sys

    print(
        "Failed to import treesearch native extension. "
        "Please build the package with 'maturin develop' or 'pip install -e .'",
        file=sys.stderr,
    )
    raise

__all__ = [
    "Tree",
    "Word",
    "Pattern",
    "Treebank",
    "TreeIterator",
    "MatchIterator",
    "parse_query",
    "search",
    "read_trees",
    "search_file",
    "read_trees_glob",
    "search_files",
    "open",
    "from_string",
]


def open(path):
    """Open a treebank from a file or glob pattern.

    Automatically detects whether the path is a glob pattern (contains * or ?)
    and uses the appropriate method to create a Treebank.

    Args:
        path: Path to a CoNLL-U file or glob pattern (e.g., "data/*.conllu")

    Returns:
        Treebank object

    Raises:
        ValueError: If glob pattern is invalid

    Example:
        >>> tb = treesearch.open("corpus.conllu")
        >>> tb = treesearch.open("data/*.conllu")
        >>> for tree in tb.trees():
        ...     print(tree.sentence_text)
    """
    if "*" in path or "?" in path:
        return Treebank.from_glob(path)
    else:
        return Treebank.from_file(path)


def from_string(text):
    """Create a treebank from a CoNLL-U string.

    Args:
        text: CoNLL-U formatted text

    Returns:
        Treebank object

    Example:
        >>> conllu = '''# text = Hello world.
        ... 1	Hello	hello	INTJ	_	_	0	root	_	_
        ... 2	world	world	NOUN	_	_	1	vocative	_	_
        ... 3	.	.	PUNCT	_	_	1	punct	_	_
        ... '''
        >>> tb = treesearch.from_string(conllu)
        >>> for tree in tb.trees():
        ...     print(tree.sentence_text)
    """
    return Treebank.from_string(text)
