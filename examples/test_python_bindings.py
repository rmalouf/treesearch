#!/usr/bin/env python3
"""Test script to verify Python bindings work correctly."""

from treesearch import Pattern, MatchIterator

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
    pattern = Pattern.from_query("""
        V [pos="VERB"];
    """)
    print(f"  Created pattern with {pattern.n_vars} variable(s)")
    assert pattern.n_vars == 1
    print("  ✓ Pattern creation works")


def test_simple_search():
    """Test searching for a simple pattern."""
    print("\nTesting simple search...")

    # Create pattern for verb
    pattern = Pattern.from_query('V [pos="VERB"];')

    # Search using iterator
    matches = list(MatchIterator.from_string(SAMPLE_CONLLU, pattern))
    print(f"  Found {len(matches)} tree(s) with matches")

    for tree, match in matches:
        print(f"  Tree has {len(tree)} words")
        print(f"  Match has {len(match)} word(s)")

        # Get the matched word
        word_id = match[0]
        word = tree.get_word(word_id)
        print(f"  Matched verb: {word.form} (lemma={word.lemma}, pos={word.pos})")

    assert len(matches) > 0
    print("  ✓ Simple search works")


def test_edge_constraint():
    """Test searching with edge constraints."""
    print("\nTesting edge constraint search...")

    # Pattern: verb with an xcomp child
    pattern = Pattern.from_query("""
        V1 [pos="VERB"];
        V2 [pos="VERB"];
        V1 -[xcomp]-> V2;
    """)

    matches = list(MatchIterator.from_string(SAMPLE_CONLLU, pattern))
    print(f"  Found {len(matches)} match(es)")

    for tree, match in matches:
        v1_id = match[0]
        v2_id = match[1]
        v1 = tree.get_word(v1_id)
        v2 = tree.get_word(v2_id)
        print(f"  Match: {v1.form} -[xcomp]-> {v2.form}")

    assert len(matches) > 0
    print("  ✓ Edge constraint search works")


def test_word_properties():
    """Test accessing word properties."""
    print("\nTesting word properties...")

    pattern = Pattern.from_query('V [lemma="help"];')
    matches = list(MatchIterator.from_string(SAMPLE_CONLLU, pattern))

    for tree, match in matches:
        word = tree.get_word(match[0])
        print(f"  Word ID: {word.id}")
        print(f"  Token ID: {word.token_id}")
        print(f"  Form: {word.form}")
        print(f"  Lemma: {word.lemma}")
        print(f"  POS: {word.pos}")
        print(f"  XPOS: {word.xpos}")
        print(f"  Deprel: {word.deprel}")
        print(f"  Head: {word.head}")
        print(f"  Children IDs: {word.children_ids}")

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


def test_tree_properties():
    """Test accessing tree properties."""
    print("\nTesting tree properties...")

    pattern = Pattern.from_query('V [pos="VERB"];')
    matches = list(MatchIterator.from_string(SAMPLE_CONLLU, pattern))

    for tree, match in matches:
        print(f"  Tree length: {len(tree)}")
        print(f"  Sentence text: {tree.sentence_text}")
        print(f"  Tree repr: {repr(tree)}")

    print("  ✓ Tree properties work")


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
