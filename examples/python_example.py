#!/usr/bin/env python3
"""Example of using treesearch from Python.

This example demonstrates how to:
1. Read CoNLL-U files
2. Search for patterns
3. Access matched nodes and their properties
4. Use the children_by_deprel API

To run this example:
1. Build the Python extension: maturin develop
2. Run: python examples/python_example.py
"""

import treesearch


def main():
    print("=== Treesearch Python Example ===\n")

    # Example 1: Simple pattern matching
    print("1. SIMPLE PATTERN MATCHING")
    print("   Query: Verb with NOUN subject")

    query1 = """
        Verb [upos="VERB"];
        Noun [upos="NOUN"];
        Verb -[nsubj]-> Noun;
    """

    pattern1 = treesearch.compile_query(query1)
    match_count = 0

    for tree in treesearch.read_trees("examples/lw970831.conll"):
        for match in treesearch.search(tree, pattern1):
            verb = tree.get_word(match["Verb"])
            noun = tree.get_word(match["Noun"])
            print(f"   Found: {verb.form} -> {noun.form}")
            match_count += 1
            if match_count >= 5:  # Limit output
                break
        if match_count >= 5:
            break

    print(f"\n   (showing first {match_count} matches)\n")

    # Example 2: Using children_by_deprel
    print("2. USING children_by_deprel API")
    print("   Finding verbs and their objects")

    query2 = """
        Verb [upos="VERB"];
    """

    pattern2 = treesearch.compile_query(query2)
    verb_count = 0

    for tree in treesearch.read_trees("examples/lw970831.conll"):
        for match in treesearch.search(tree, pattern2):
            verb_word = tree.get_word(match["Verb"])

            # Get all object dependents
            objects = verb_word.children_by_deprel("obj")
            if objects:
                obj_forms = [obj.form for obj in objects]
                print(f"   {verb_word.form} has objects: {', '.join(obj_forms)}")

            # Get subject (usually just one)
            subjects = verb_word.children_by_deprel("nsubj")
            if subjects:
                print(f"   {verb_word.form} has subject: {subjects[0].form}")

            verb_count += 1
            if verb_count >= 3:
                break
        if verb_count >= 3:
            break

    print()

    # Example 3: Accessing node properties
    print("3. NODE PROPERTIES")
    print("   Showing detailed node information")

    for tree in treesearch.read_trees("examples/lw970831.conll"):
        if len(tree) > 5:  # Get a tree with some nodes
            word = tree.get_word(1)  # Get word at position 1
            if word:
                print(f"   Word {word.id}:")
                print(f"     Form: {word.form}")
                print(f"     Lemma: {word.lemma}")
                print(f"     POS: {word.pos}")
                print(f"     DepRel: {word.deprel}")

                # Show parent and children
                if word.parent():
                    print(f"     Parent: {word.parent().form}")

                children = word.children()
                if children:
                    child_forms = [c.form for c in children]
                    print(f"     Children: {', '.join(child_forms)}")
                break

    print()

    # Example 4: Match dictionaries
    print("4. MATCH DICTIONARIES")
    print("   Accessing all matched variables")

    query4 = """
        Head [];
        Dep1 [];
        Dep2 [];
        Head -> Dep1;
        Head -> Dep2;
    """

    pattern4 = treesearch.compile_query(query4)

    for tree in treesearch.read_trees("examples/lw970831.conll"):
        for match in treesearch.search(tree, pattern4):
            # match is a dictionary: {"Head": 3, "Dep1": 5, "Dep2": 7}
            print(f"   Variable bindings: {match}")

            # Get nodes for each variable
            for var_name, word_id in match.items():
                word = tree.get_word(word_id)
                print(f"   {var_name} = {word.form} ({word.pos})")
            break
        break

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
