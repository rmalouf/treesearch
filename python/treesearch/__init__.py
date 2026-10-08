"""Treesearch: High-performance dependency tree pattern matching."""

from __future__ import annotations

import os
from collections.abc import Iterable
from importlib.metadata import version

__version__ = version("treesearch-ud")

try:
    from .treesearch import (
        MatchIterator,
        Pattern,
        Tree,
        Treebank,
        TreeIterator,
        Word,
        compile_query,
        search_trees,
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
    "MatchIterator",
    "Pattern",
    "Tree",
    "TreeIterator",
    "Treebank",
    "Word",
    "compile_query",
    "from_string",
    "load",
    "render",
    "search",
    "search_trees",
    "to_displacy",
    "trees",
]


type Source = str | os.PathLike[str] | Iterable[str | os.PathLike[str]]

_GLOB_CHARS = frozenset("*?[")


def load(source: Source) -> Treebank:
    """Open a treebank from a file, a glob pattern, or a collection of files.

    A str or path containing glob characters (``*``, ``?``, ``[``) is expanded
    as a glob pattern, with matching files sorted so results are deterministic.
    Anything else is taken as a literal file path. Files are opened lazily, so
    a missing file raises OSError when the treebank is iterated.

    Args:
        source: Path to a CoNLL-U file, a glob pattern such as
            "data/**/*.conllu.gz", or an iterable of file paths

    Returns:
        Treebank object

    Raises:
        TypeError: If source is not a str, path, or iterable of those
        ValueError: If glob pattern is invalid

    Example:
        >>> tb = treesearch.load("corpus.conllu")
        >>> tb = treesearch.load(Path("corpus.conllu"))
        >>> tb = treesearch.load("data/*.conllu")
        >>> for tree in tb.trees():
        ...     print(tree.sentence_text)
    """
    if isinstance(source, (str, os.PathLike)):
        path = os.fspath(source)
        if _GLOB_CHARS.intersection(path):
            return Treebank.from_glob(path)
        return Treebank.from_file(path)
    if isinstance(source, Iterable) and not isinstance(source, (bytes, bytearray)):
        return Treebank.from_files(list(source))
    raise TypeError("source must be str, os.PathLike[str], or an iterable of those")


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


def trees(source: Source, ordered: bool = True) -> TreeIterator:
    """Read trees from one or more CoNLL-U files.

    Args:
        source: File path, glob pattern, or iterable of file paths
        ordered: If True (default), return trees in deterministic order

    Returns:
        Iterator over Tree objects
    """
    treebank = load(source)
    return treebank.trees(ordered=ordered)


def search(
    source: Source,
    query: str | Pattern,
    ordered: bool = True,
) -> MatchIterator:
    """Search one or more files for pattern matches.

    Args:
        source: File path, glob pattern, or iterable of file paths
        query: Query string or compiled Pattern
        ordered: If True (default), return matches in deterministic order

    Returns:
        Iterator over (Tree, match_dict) tuples
    """
    treebank = load(source)
    return treebank.search(query, ordered=ordered)


def to_displacy(tree: Tree) -> dict[str, list]:
    """Convert a Tree to displaCy's manual rendering format.

    Same as ``tree.to_displacy()``.

    Example:
        >>> tree = next(treesearch.trees("corpus.conllu"))
        >>> data = treesearch.to_displacy(tree)
        >>> from spacy import displacy
        >>> displacy.render(data, style="dep", manual=True)
    """
    return tree.to_displacy()


def render(tree: Tree, **options) -> str:
    """Render a Tree as an SVG dependency visualization using displaCy.

    Same as ``tree.render(**options)``. Requires spaCy
    (``pip install treesearch-ud[viz]``).

    Args:
        tree: A Tree object to render
        **options: Additional options passed to displacy.render(),
            e.g. jupyter=True or options={"compact": True}

    Raises:
        ImportError: If spaCy is not installed

    Example:
        >>> tree = next(treesearch.trees("corpus.conllu"))
        >>> svg = treesearch.render(tree)
        >>> with open("tree.svg", "w") as f:
        ...     f.write(svg)
    """
    return tree.render(**options)
