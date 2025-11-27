#!/usr/bin/env python3
"""Test script to verify Python bindings work correctly."""

import treesearch
import tempfile
import os

# Create a simple CoNLL-U test string
SAMPLE_CONLLU = """# text = He helped us to win.
1	He	he	PRON	PRP	_	2	nsubj	_	_
2	helped	help	VERB	VBD	_	0	root	_	_
3	us	we	PRON	PRP	_	2	obj	_	_
4	to	to	PART	TO	_	5	mark	_	_
5	win	win	VERB	VB	_	2	xcomp	_	_
6	.	.	PUNCT	.	_	2	punct	_	_

"""


def test_pattern_creation():
    """Test that we can create a pattern from a query."""
    print("Testing pattern creation...")
    pattern = treesearch.parse_query("""
        V [upos="VERB"];
    """)
    print(f"  Created pattern with {pattern.n_vars} variable(s)")
    assert pattern.n_vars == 1
    print("  ✓ Pattern creation works")


def test_simple_search():
    """Test searching for a simple pattern."""
    print("\nTesting simple search...")

    # Create pattern for verb
    pattern = treesearch.parse_query('V [upos="VERB"];')

    # Write sample to temp file
    with tempfile.NamedTemporaryFile(mode="w", suffix=".conllu", delete=False) as f:
        f.write(SAMPLE_CONLLU)
        temp_path = f.name

    try:
        # Search using search_file
        matches = list(treesearch.search_file(temp_path, pattern))
        print(f"  Found {len(matches)} tree(s) with matches")

        for tree, match in matches:
            print(f"  Tree has {len(tree)} words")
            print(f"  Match: {match}")

            # Get the matched word
            word = tree.get_word(match["V"])
            print(f"  Matched verb: {word.form} (lemma={word.lemma}, pos={word.pos})")

        assert len(matches) > 0
        print("  ✓ Simple search works")
    finally:
        os.unlink(temp_path)


def test_edge_constraint():
    """Test searching with edge constraints."""
    print("\nTesting edge constraint search...")

    # Pattern: verb with an xcomp child
    pattern = treesearch.parse_query("""
        V1 [upos="VERB"];
        V2 [upos="VERB"];
        V1 -[xcomp]-> V2;
    """)

    # Write sample to temp file
    with tempfile.NamedTemporaryFile(mode="w", suffix=".conllu", delete=False) as f:
        f.write(SAMPLE_CONLLU)
        temp_path = f.name

    try:
        matches = list(treesearch.search_file(temp_path, pattern))
        print(f"  Found {len(matches)} match(es)")

        for tree, match in matches:
            v1 = tree.get_word(match["V1"])
            v2 = tree.get_word(match["V2"])
            print(f"  Match: {v1.form} -[xcomp]-> {v2.form}")

        assert len(matches) > 0
        print("  ✓ Edge constraint search works")
    finally:
        os.unlink(temp_path)


def test_word_properties():
    """Test accessing word properties."""
    print("\nTesting word properties...")

    pattern = treesearch.parse_query('V [lemma="help"];')

    # Write sample to temp file
    with tempfile.NamedTemporaryFile(mode="w", suffix=".conllu", delete=False) as f:
        f.write(SAMPLE_CONLLU)
        temp_path = f.name

    try:
        matches = list(treesearch.search_file(temp_path, pattern))

        for tree, match in matches:
            word = tree.get_word(match["V"])
            print(f"  Word ID: {word.id}")
            print(f"  Form: {word.form}")
            print(f"  Lemma: {word.lemma}")
            print(f"  POS: {word.pos}")
            print(f"  XPOS: {word.xpos}")
            print(f"  Deprel: {word.deprel}")
            print(f"  Head: {word.head}")

            # Test parent/children methods
            parent = word.parent()
            if parent:
                print(f"  Parent: {parent.form}")
            else:
                print(f"  Parent: None (root)")

            children = word.children()
            print(f"  Children: {[c.form for c in children]}")

            # Test children_by_deprel
            obj_children = word.children_by_deprel("obj")
            print(f"  'obj' children: {[c.form for c in obj_children]}")

        print("  ✓ Word properties work")
    finally:
        os.unlink(temp_path)


def test_tree_properties():
    """Test accessing tree properties."""
    print("\nTesting tree properties...")

    pattern = treesearch.parse_query('V [upos="VERB"];')

    # Write sample to temp file
    with tempfile.NamedTemporaryFile(mode="w", suffix=".conllu", delete=False) as f:
        f.write(SAMPLE_CONLLU)
        temp_path = f.name

    try:
        matches = list(treesearch.search_file(temp_path, pattern))

        for tree, match in matches:
            print(f"  Tree length: {len(tree)}")
            print(f"  Sentence text: {tree.sentence_text}")
            print(f"  Tree repr: {repr(tree)}")
            break  # Just test first match

        print("  ✓ Tree properties work")
    finally:
        os.unlink(temp_path)


def main():
    """Run all tests."""
    print("=" * 60)
    print("Testing Python Bindings for Treesearch")
    print("=" * 60)

    try:
        test_pattern_creation()
        test_simple_search()
        test_edge_constraint()
        test_word_properties()
        test_tree_properties()

        print("\n" + "=" * 60)
        print("All tests passed! ✓")
        print("=" * 60)

    except Exception as e:
        print(f"\n✗ Test failed with error: {e}")
        import traceback

        traceback.print_exc()
        return 1

    return 0


if __name__ == "__main__":
    exit(main())
