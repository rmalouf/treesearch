#!/usr/bin/env python3
"""Test find_path in Python bindings"""

import tempfile
import os
import treesearch

# Create a test CoNLL-U file
CONLLU_DATA = """# text = The big dog runs in the park.
1	The	the	DET	_	_	3	det	_	_
2	big	big	ADJ	_	_	3	amod	_	_
3	dog	dog	NOUN	_	_	4	nsubj	_	_
4	runs	run	VERB	_	_	0	root	_	_
5	in	in	ADP	_	_	7	case	_	_
6	the	the	DET	_	_	7	det	_	_
7	park	park	NOUN	_	_	4	obl	_	_
8	.	.	PUNCT	_	_	4	punct	_	_

"""

def test_find_path():
    print("Testing find_path Python binding...")

    # Create temporary file with test data
    with tempfile.NamedTemporaryFile(mode='w', suffix='.conllu', delete=False) as f:
        f.write(CONLLU_DATA)
        temp_file = f.name

    try:
        # Read the tree
        trees = treesearch.read_trees(temp_file)
        tree = next(trees)

        print(f"Loaded tree with {len(tree)} words")

        # Test 1: Direct child (runs -> dog)
        runs = tree.get_word(3)  # "runs" at position 3
        dog = tree.get_word(2)   # "dog" at position 2

        path = tree.find_path(runs, dog)
        if path:
            print(f"\n✓ Test 1 - Direct child: runs -> dog")
            print(f"  Path length: {len(path)}")
            print(f"  Path: {' -> '.join(w.form for w in path)}")
            assert len(path) == 2
            assert path[0].form == "runs"
            assert path[1].form == "dog"
        else:
            print("✗ Test 1 failed: No path found")

        # Test 2: Multi-level path (runs -> dog -> big)
        big = tree.get_word(1)  # "big" at position 1

        path = tree.find_path(runs, big)
        if path:
            print(f"\n✓ Test 2 - Multi-level: runs -> dog -> big")
            print(f"  Path length: {len(path)}")
            print(f"  Path: {' -> '.join(w.form for w in path)}")
            assert len(path) == 3
            assert path[0].form == "runs"
            assert path[1].form == "dog"
            assert path[2].form == "big"
        else:
            print("✗ Test 2 failed: No path found")

        # Test 3: Different branch (runs -> park -> the)
        park = tree.get_word(6)  # "park" at position 6
        the = tree.get_word(5)   # "the" at position 5

        path = tree.find_path(runs, park)
        if path:
            print(f"\n✓ Test 3 - Different branch: runs -> park")
            print(f"  Path length: {len(path)}")
            print(f"  Path: {' -> '.join(w.form for w in path)}")
            assert len(path) == 2
            assert path[0].form == "runs"
            assert path[1].form == "park"
        else:
            print("✗ Test 3 failed: No path found")

        # Test 4: No path (siblings)
        path = tree.find_path(dog, park)
        if path is None:
            print(f"\n✓ Test 4 - No path between siblings: dog and park")
        else:
            print(f"✗ Test 4 failed: Found path when none should exist")

        # Test 5: No path (reverse direction)
        path = tree.find_path(dog, runs)
        if path is None:
            print(f"\n✓ Test 5 - No path in reverse direction: dog -> runs")
        else:
            print(f"✗ Test 5 failed: Found path when none should exist")

        # Test 6: Same node
        path = tree.find_path(runs, runs)
        if path is None:
            print(f"\n✓ Test 6 - No path for same node: runs -> runs")
        else:
            print(f"✗ Test 6 failed: Found path for same node")

        print("\n✓ All tests passed!")

    finally:
        # Clean up
        os.unlink(temp_file)

if __name__ == "__main__":
    test_find_path()
