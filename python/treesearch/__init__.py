"""Treesearch: High-performance dependency tree pattern matching."""

from __future__ import annotations

from importlib.metadata import version
from pathlib import Path
import glob
from typing import Iterable, Iterator, Union

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
    "open",
    "from_string",
    "get_trees",
    "get_matches",
]


def open(source: str | Path | Iterable[str | Path]) -> Treebank:
    """Open a treebank from a file or glob pattern.

    Automatically detects whether the path is a glob pattern (contains * or ?)
    and uses the appropriate method to create a Treebank.

    Args:
        source: Path to a CoNLL-U file or glob pattern (str or pathlib.Path)
              e.g., "data/*.conllu" or Path("corpus.conllu")

    Returns:
        Treebank object

    Raises:
        ValueError: If glob pattern is invalid

    Example:
        >>> tb = treesearch.open("corpus.conllu")
        >>> tb = treesearch.open(Path("corpus.conllu"))
        >>> tb = treesearch.open("data/*.conllu")
        >>> for tree in tb.trees():
        ...     print(tree.sentence_text)
    """

    if isinstance(source, str):
        paths = list(glob.glob(source, recursive=True))
        return Treebank.from_files(paths)
    elif isinstance(source, Path):
        return Treebank.from_file(str(source))
    elif isinstance(source, Iterable):
        source_list = [str(path) for path in source]
        return Treebank.from_files(source_list)
    else:
        raise ValueError("source must be str, Path, or Iterable[str | Path]")


def from_string(text: str) -> Treebank:
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


def get_trees(source: str | Path | Iterable[str | Path], ordered: bool = True) -> TreeIterator:
    """Read trees from one or more CoNLL-U files.

    Args:
        source: Path to a single file or glob pattern
        ordered: If True (default), return trees in deterministic order

    Returns:
        Iterator over Tree objects
    """
    treebank = open(source)
    return treebank.trees(ordered=ordered)


def get_matches(
    source: str | Path | Iterable[str | Path],
    query: str | Pattern,
    ordered: bool = True,
) -> MatchIterator:
    """Search one or more files for pattern matches.

    Args:
        source: Path to a single file or glob pattern
        query: Query string or compiled Pattern
        ordered: If True (default), return matches in deterministic order

    Returns:
        Iterator over (Tree, match_dict) tuples
    """
    treebank = open(source)
    if isinstance(query, str):
        query = parse_query(query)
    return treebank.matches(query, ordered=ordered)
