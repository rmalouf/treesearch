"""Treesearch: High-performance dependency tree pattern matching."""

from __future__ import annotations

import glob
from importlib.metadata import version
from pathlib import Path
from typing import Iterable

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
        py_search_trees,
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
    "compile_query",
    "search",
    "load",
    "from_string",
    "trees",
    "search",
    "search_trees",
    "to_displacy",
    "render",
]


def load(source: str | Path | Iterable[str | Path]) -> Treebank:
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
        >>> tb = treesearch.load("corpus.conllu")
        >>> tb = treesearch.load(Path("corpus.conllu"))
        >>> tb = treesearch.load("data/*.conllu")
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


def trees(source: str | Path | Iterable[str | Path], ordered: bool = True) -> TreeIterator:
    """Read trees from one or more CoNLL-U files.

    Args:
        source: Path to a single file or glob pattern
        ordered: If True (default), return trees in deterministic order

    Returns:
        Iterator over Tree objects
    """
    treebank = load(source)
    return treebank.trees(ordered=ordered)


def search(
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
    treebank = load(source)
    return treebank.search(query, ordered=ordered)


def search_trees(
    source: Tree | Iterable[Tree],
    query: str | Pattern,
) -> MatchIterator:
    """Search a tree or list of trees for pattern matches.

    Args:
        source: Single Tree or iterable of Trees
        query: Query string or compiled Pattern

    Returns:
        Iterator over (Tree, match_dict) tuples
    """
    if isinstance(source, Tree):
        source = [source]
    else:
        source = list(source)
    return py_search_trees(source, query)


def to_displacy(tree: Tree) -> dict:
    """Convert a Tree to displaCy's manual rendering format.

    Args:
        tree: A Tree object to convert

    Returns:
        Dictionary in displaCy format with 'words' and 'arcs' keys

    Example:
        >>> tree = next(treesearch.trees("corpus.conllu"))
        >>> data = treesearch.to_displacy(tree)
        >>> from spacy import displacy
        >>> displacy.render(data, style="dep", manual=True)
    """
    words = []
    arcs = []

    for i in range(len(tree)):
        word = tree.word(i)
        words.append({"text": word.form, "tag": word.upos})

        if word.head is not None:
            head_idx = word.head
            dep_idx = word.id
            if head_idx < dep_idx:
                arcs.append(
                    {
                        "start": head_idx,
                        "end": dep_idx,
                        "label": word.deprel,
                        "dir": "right",
                    }
                )
            else:
                arcs.append(
                    {
                        "start": dep_idx,
                        "end": head_idx,
                        "label": word.deprel,
                        "dir": "left",
                    }
                )
    return {"words": words, "arcs": arcs}


def render(tree: Tree, **options) -> str:
    """Render a Tree as an SVG dependency visualization using displaCy.

    Requires spaCy to be installed.

    Args:
        tree: A Tree object to render
        **options: Additional options passed to displacy.render()
            Common options include:
            - jupyter: bool - Return HTML for Jupyter display (default: auto-detect)
            - compact: bool - Use compact visualization mode
            - word_spacing: int - Spacing between words
            - distance: int - Distance between dependency arcs

    Returns:
        SVG markup string (or displays in Jupyter if jupyter=True)

    Raises:
        ImportError: If spaCy is not installed

    Example:
        >>> tree = next(treesearch.trees("corpus.conllu"))
        >>> svg = treesearch.render(tree)
        >>> with open("tree.svg", "w") as f:
        ...     f.write(svg)

        # In Jupyter notebook:
        >>> treesearch.render(tree, jupyter=True)
    """
    try:
        from spacy import displacy
    except ImportError:
        raise ImportError("spaCy is required for rendering. Install it with: pip install spacy")

    data = to_displacy(tree)
    return displacy.render(data, style="dep", manual=True, **options)


Tree.to_displacy = to_displacy
Tree.render = render
