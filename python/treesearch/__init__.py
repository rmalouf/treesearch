"""Treesearch: High-performance dependency tree pattern matching.
"""

from importlib.metadata import version
__version__ = version("treesearch")

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
