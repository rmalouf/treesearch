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

from treesearch import CoNLLUReader, search_query


def main():
    print("=== Treesearch Python Example ===\n")

    # Example 1: Simple pattern matching
    print("1. SIMPLE PATTERN MATCHING")
    print("   Query: Verb with NOUN subject")

    query1 = """
        Verb [pos="VERB"];
        Noun [pos="NOUN"];
        Verb -[nsubj]-> Noun;
    """

    # Read trees from CoNLL-U file
    reader = CoNLLUReader.from_file("examples/lw970831.conll")

    match_count = 0
    for tree in reader:
        for match in search_query(tree, query1):
            verb = match.get_node("Verb")
            noun = match.get_node("Noun")
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
        Verb [pos="VERB"];
    """

    reader = CoNLLUReader.from_file("examples/lw970831.conll")

    for tree in reader:
        for match in search_query(tree, query2):
            verb_node = match.get_node("Verb")

            # Get all object dependents
            objects = verb_node.children_by_deprel("obj")
            if objects:
                obj_forms = [obj.form for obj in objects]
                print(f"   {verb_node.form} has objects: {', '.join(obj_forms)}")

            # Get subject (usually just one)
            subjects = verb_node.children_by_deprel("nsubj")
            if subjects:
                print(f"   {verb_node.form} has subject: {subjects[0].form}")

            if match_count >= 3:
                break
        if match_count >= 3:
            break

    print()

    # Example 3: Accessing node properties
    print("3. NODE PROPERTIES")
    print("   Showing detailed node information")

    reader = CoNLLUReader.from_file("examples/lw970831.conll")

    for tree in reader:
        if len(tree) > 5:  # Get a tree with some nodes
            node = tree.get_node(1)  # Get second node (first is usually root)
            if node:
                print(f"   Node {node.id}:")
                print(f"     Form: {node.form}")
                print(f"     Lemma: {node.lemma}")
                print(f"     POS: {node.pos}")
                print(f"     DepRel: {node.deprel}")
                print(f"     Position: {node.position}")

                # Show parent and children
                if node.parent():
                    print(f"     Parent: {node.parent().form}")

                children = node.children()
                if children:
                    child_forms = [c.form for c in children]
                    print(f"     Children: {', '.join(child_forms)}")
                break

    print()

    # Example 4: Match bindings
    print("4. MATCH BINDINGS")
    print("   Accessing all matched variables")

    query4 = """
        Head [];
        Dep1 [];
        Dep2 [];
        Head -> Dep1;
        Head -> Dep2;
    """

    reader = CoNLLUReader.from_file("examples/lw970831.conll")

    for tree in reader:
        matches = search_query(tree, query4)
        if matches:
            match = matches[0]  # Get first match

            # Get all bindings as a dictionary
            bindings = match.bindings()
            print(f"   Variable bindings: {bindings}")

            # Get nodes dictionary
            nodes_dict = match.nodes()
            for var_name, node in nodes_dict.items():
                print(f"   {var_name} = {node.form} ({node.pos})")
            break

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
