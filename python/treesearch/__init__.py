"""Treesearch: High-performance dependency tree pattern matching.

A toolkit for querying linguistic dependency parses at scale.

Example usage:
    >>> from treesearch import Pattern, MatchIterator
    >>>
    >>> # Parse a query into a pattern
    >>> pattern = Pattern.from_query('''
    ...     Verb [pos="VERB"];
    ...     Noun [pos="NOUN"];
    ...     Verb -[nsubj]-> Noun;
    ... ''')
    >>>
    >>> # Search a single file
    >>> for tree, match in MatchIterator.from_file("corpus.conllu", pattern):
    ...     verb_id = match[0]  # First variable
    ...     noun_id = match[1]  # Second variable
    ...     verb = tree.get_word(verb_id)
    ...     noun = tree.get_word(noun_id)
    ...     print(f"{verb.form} -> {noun.form}")
    >>>
    >>> # Search multiple files in parallel
    >>> from treesearch import MultiFileMatchIterator
    >>> for tree, match in MultiFileMatchIterator.from_glob("data/*.conllu", pattern):
    ...     # Process matches from all files
    ...     pass
"""

__version__ = "0.1.0"

# Import native extension
try:
    from .treesearch import (
        Tree,
        Word,
        Match,
        Pattern,
        search,
        MatchIterator,
        MultiFileTreeIterator,
        MultiFileMatchIterator,
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
    "Word",
    "Match",
    "Pattern",
    "search",
    "MatchIterator",
    "MultiFileTreeIterator",
    "MultiFileMatchIterator",
]
