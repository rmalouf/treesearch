"""Type stubs for treesearch PyO3 extension module."""

from __future__ import annotations

from typing import Iterator, Optional

class Tree:
    """Represents a dependency tree."""

    @property
    def sentence_text(self) -> Optional[str]:
        """Reconstructed sentence text from CoNLL-U comments."""
        ...

    @property
    def metadata(self) -> dict[str, str]:
        """Tree metadata from CoNLL-U comment lines."""
        ...

    def get_word(self, id: int) -> Optional[Word]:
        """Get word by ID (0-based index).

        Args:
            id: Word ID (0-based)

        Returns:
            Word object or None if ID is out of bounds
        """
        ...

    def __len__(self) -> int:
        """Number of words in tree."""
        ...

    def __repr__(self) -> str: ...

class Word:
    """Represents a single word/token in a dependency tree."""

    @property
    def id(self) -> int:
        """Word ID (0-based index in tree)."""
        ...

    @property
    def token_id(self) -> int:
        """Token ID from CoNLL-U (1-based)."""
        ...

    @property
    def form(self) -> str:
        """Word form (surface text)."""
        ...

    @property
    def lemma(self) -> str:
        """Lemma (base form)."""
        ...

    @property
    def pos(self) -> str:
        """Universal POS tag (upos)."""
        ...

    @property
    def xpos(self) -> Optional[str]:
        """Language-specific POS tag."""
        ...

    @property
    def deprel(self) -> str:
        """Dependency relation to parent."""
        ...

    @property
    def head(self) -> Optional[int]:
        """Head word ID (0-based index), None for root."""
        ...

    @property
    def children_ids(self) -> list[int]:
        """IDs of all children words."""
        ...

    def parent(self) -> Optional[Word]:
        """Get parent word, None for root."""
        ...

    def children(self) -> list[Word]:
        """Get all children words."""
        ...

    def children_by_deprel(self, deprel: str) -> list[Word]:
        """Get children with specific dependency relation.

        Args:
            deprel: Dependency relation to filter by

        Returns:
            List of child words with matching deprel
        """
        ...

    def __repr__(self) -> str: ...

class Pattern:
    """Compiled query pattern."""

    def __repr__(self) -> str: ...

class Treebank:
    """Collection of dependency trees from files or strings."""

    @classmethod
    def from_string(cls, text: str) -> Treebank:
        """Create treebank from CoNLL-U string.

        Args:
            text: CoNLL-U formatted text

        Returns:
            Treebank object
        """
        ...

    @classmethod
    def from_file(cls, file_path: str) -> Treebank:
        """Create treebank from single CoNLL-U file.

        Args:
            file_path: Path to CoNLL-U file (supports .conllu and .conllu.gz)

        Returns:
            Treebank object
        """
        ...

    @classmethod
    def from_files(cls, file_paths: list[str]) -> Treebank:
        """Create treebank from multiple CoNLL-U files.

        Args:
            file_paths: List of paths to CoNLL-U files

        Returns:
            Treebank object
        """
        ...

    def trees(self, ordered: bool = True) -> TreeIterator:
        """Iterate over trees in treebank.

        Args:
            ordered: If True (default), return trees in deterministic order.
                    If False, trees may arrive in any order for better performance.

        Returns:
            Iterator over Tree objects
        """
        ...

    def matches(self, pattern: Pattern, ordered: bool = True) -> MatchIterator:
        """Search for pattern matches across all trees.

        Args:
            pattern: Compiled pattern from parse_query()
            ordered: If True (default), return matches in deterministic order.
                    If False, matches may arrive in any order for better performance.

        Returns:
            Iterator over (Tree, match_dict) tuples
        """
        ...

    def __repr__(self) -> str: ...

class TreeIterator(Iterator[Tree]):
    """Iterator over Tree objects."""

    def __iter__(self) -> TreeIterator: ...
    def __next__(self) -> Tree: ...

class MatchIterator(Iterator[tuple[Tree, dict[str, int]]]):
    """Iterator over (Tree, match_dict) tuples."""

    def __iter__(self) -> MatchIterator: ...
    def __next__(self) -> tuple[Tree, dict[str, int]]: ...

def parse_query(query: str) -> Pattern:
    """Parse query string into Pattern object.

    Args:
        query: Query string in treesearch query language

    Returns:
        Compiled Pattern object

    Raises:
        ValueError: If query syntax is invalid
    """
    ...

def search(tree: Tree, pattern: Pattern) -> list[dict[str, int]]:
    """Search a single tree for pattern matches.

    Args:
        tree: Tree to search
        pattern: Compiled pattern from parse_query()

    Returns:
        List of match dictionaries mapping variable names to word IDs (0-based)
    """
    ...

# Deprecated functions (still exported but not recommended for use)
def search_file(path: str, pattern: Pattern, ordered: bool = True) -> MatchIterator:
    """Search single file for matches.

    .. deprecated::
        Use get_matches() from the main module instead.
    """
    ...

def read_trees_glob(glob_pattern: str, ordered: bool = True) -> TreeIterator:
    """Read trees from multiple files.

    .. deprecated::
        Use get_trees() from the main module instead.
    """
    ...

def search_files(glob_pattern: str, pattern: Pattern, ordered: bool = True) -> MatchIterator:
    """Search multiple files for matches.

    .. deprecated::
        Use get_matches() from the main module instead.
    """
    ...
