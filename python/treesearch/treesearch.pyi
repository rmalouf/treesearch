"""Type stubs for treesearch PyO3 extension module."""

from collections.abc import Iterable, Iterator
from os import PathLike
from typing import Any

class Tree:
    """Represents a dependency tree."""

    @property
    def sentence_text(self) -> str | None:
        """Reconstructed sentence text from CoNLL-U comments."""

    @property
    def metadata(self) -> dict[str, str]:
        """Tree metadata from CoNLL-U comment lines."""

    def word(self, id: int) -> Word:
        """Get word by ID (0-based index).

        Args:
            id: Word ID (0-based)

        Returns:
            Word object

        Raises:
            IndexError: If ID is out of bounds
        """

    def __getitem__(self, id: int) -> Word:
        """Get word by position; negative positions count from the end.

        Raises:
            IndexError: If position is out of bounds
        """

    def __len__(self) -> int:
        """Number of words in tree."""

    def __iter__(self) -> Iterator[Word]: ...
    def to_displacy(self) -> dict[str, list[dict[str, Any]]]:
        """Convert to displaCy's manual rendering format ('words' and 'arcs' keys)."""

    def render(self, **options: Any) -> str:
        """Render as an SVG dependency visualization using displaCy.

        Args:
            **options: Additional options passed to displacy.render()

        Raises:
            ImportError: If spaCy is not installed
        """

class Word:
    """Represents a single word/token in a dependency tree.

    Two words are equal when they are the same word of the same Tree object.
    """

    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    @property
    def id(self) -> int:
        """Word ID (0-based index in tree)."""

    @property
    def token_id(self) -> int:
        """Token ID from CoNLL-U (1-based)."""

    @property
    def form(self) -> str:
        """Word form (surface text)."""

    @property
    def lemma(self) -> str:
        """Lemma (base form)."""

    @property
    def upos(self) -> str:
        """Universal POS tag (upos)."""

    @property
    def xpos(self) -> str | None:
        """Language-specific POS tag."""

    @property
    def deprel(self) -> str:
        """Dependency relation to parent."""

    @property
    def head(self) -> int | None:
        """Head word ID (0-based index), None for root."""

    @property
    def children_ids(self) -> list[int]:
        """IDs of all children words."""

    @property
    def feats(self) -> dict[str, str]:
        """Morphological features as key-value pairs."""

    @property
    def misc(self) -> dict[str, str]:
        """Miscellaneous annotations as key-value pairs."""

    def parent(self) -> Word | None:
        """Get parent word, None for root."""

    def children(self) -> list[Word]:
        """Get all children words."""

    def children_by_deprel(self, deprel: str) -> list[Word]:
        """Get children with specific dependency relation.

        Args:
            deprel: Dependency relation to filter by

        Returns:
            List of child words with matching deprel
        """

    @property
    def descendant_ids(self) -> list[int]:
        """IDs of all transitive dependents (children, grandchildren, etc.)."""

    def descendants(self) -> list[Word]:
        """Get all transitive dependents (children, grandchildren, etc.)."""

class Pattern:
    """Compiled query pattern."""

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

    @classmethod
    def from_file(cls, file_path: str | PathLike[str]) -> Treebank:
        """Create treebank from single CoNLL-U file.

        Args:
            file_path: Path to CoNLL-U file (plain, .gz, or .zst)

        Returns:
            Treebank object
        """

    @classmethod
    def from_files(cls, file_paths: list[str | PathLike[str]]) -> Treebank:
        """Create treebank from multiple CoNLL-U files, processed in the order given.

        Args:
            file_paths: List of paths to CoNLL-U files

        Returns:
            Treebank object
        """

    @classmethod
    def from_glob(cls, pattern: str) -> Treebank:
        """Create treebank from a glob pattern; matching files are sorted.

        Args:
            pattern: Glob pattern, e.g. "data/**/*.conllu.gz"

        Returns:
            Treebank object

        Raises:
            ValueError: If the glob pattern is malformed
        """

    def trees(self, ordered: bool = True) -> TreeIterator:
        """Iterate over trees in treebank.

        Args:
            ordered: If True (default), return trees in deterministic order.
                    If False, trees may arrive in any order for better performance.

        Returns:
            Iterator over Tree objects
        """

    def search(self, pattern: Pattern | str, ordered: bool = True) -> MatchIterator:
        """Search for pattern matches across all trees.

        Args:
            pattern: Compiled Pattern or query string
            ordered: If True (default), return matches in deterministic order.
                    If False, matches may arrive in any order for better performance.

        Returns:
            Iterator over (Tree, match_dict) tuples
        """

    def filter(self, pattern: Pattern | str, ordered: bool = True) -> TreeIterator:
        """Iterate over trees that have at least one match for a pattern.

        Args:
            pattern: Compiled Pattern or query string
            ordered: If True (default), return trees in deterministic order.
                    If False, trees may arrive in any order for better performance.

        Returns:
            Iterator over Tree objects
        """

class TreeIterator(Iterator[Tree]):
    """Iterator over Tree objects."""

    def __iter__(self) -> TreeIterator: ...
    def __next__(self) -> Tree: ...

class MatchIterator(Iterator[tuple[Tree, dict[str, int]]]):
    """Iterator over (Tree, match_dict) tuples."""

    def __iter__(self) -> MatchIterator: ...
    def __next__(self) -> tuple[Tree, dict[str, int]]: ...

def compile_query(query: str) -> Pattern:
    """Compile query string into Pattern object.

    Args:
        query: Query string in treesearch query language

    Returns:
        Compiled Pattern object

    Raises:
        ValueError: If query syntax is invalid
    """

def search_trees(source: Tree | Iterable[Tree], query: Pattern | str) -> MatchIterator:
    """Search a tree or an iterable of trees for pattern matches.

    Args:
        source: Single Tree or iterable of Trees
        query: Compiled Pattern or query string

    Returns:
        Iterator over (Tree, match_dict) tuples from all trees
    """
