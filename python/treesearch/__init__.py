"""Treesearch: High-performance dependency tree pattern matching.

A toolkit for querying linguistic dependency parses at scale.

Example usage:
    >>> from treesearch import CoNLLUReader, search_query
    >>>
    >>> # Read trees from a CoNLL-U file
    >>> reader = CoNLLUReader.from_file("corpus.conllu")
    >>>
    >>> # Search for a pattern
    >>> query = '''
    ...     Verb [pos="VERB"];
    ...     Noun [pos="NOUN"];
    ...     Verb -[nsubj]-> Noun;
    ... '''
    >>>
    >>> for tree in reader:
    ...     for match in search_query(tree, query):
    ...         verb = match.get_node("Verb")
    ...         noun = match.get_node("Noun")
    ...         print(f"{verb.form} -> {noun.form}")
"""

__version__ = "0.1.0"

# Import native extension
try:
    from .treesearch import (
        Tree,
        Node,
        Match,
        search_query,
        CoNLLUReader,
    )
except ImportError as e:
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
    "Node",
    "Match",
    "search_query",
    "CoNLLUReader",
]
