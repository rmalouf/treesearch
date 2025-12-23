# import click
import treesearch
from collections import Counter

query = """
Go [lemma="go"];
And [form="and"];
V [xpos="VB"];
V -[cc]-> And;
Go -[conj]-> V;
Go < And;
And < V;
"""

# query = """
#    Go [form="go"];
##    V [xpos="VB"];
#    Go -[xcomp]-> V;
# """

query = """
Help [lemma="help"];
To [form="to"];
V [xpos="VB"];
Help << To;
To << V;
"""


def main():
    count = Counter()
    examples = dict()
    path = "/Volumes/Corpora/COHA/conll/*.conllu.gz"
    pattern = treesearch.compile_query(query)
    # for filename in tqdm(list(Path(path).rglob("*.conllu.gz"))):
    for tree, match in treesearch.search_files(path, pattern):
        dep_path1 = tree.find_path(tree.get_word(match["Help"]), tree.get_word(match["V"]))
        if dep_path1:
            dep_path2 = tree.find_path(tree.get_word(match["V"]), tree.get_word(match["To"]))
            if dep_path2:
                dep_path = (
                    tuple([w.deprel for w in dep_path1[1:]]),
                    tuple([w.deprel for w in dep_path2[1:]]),
                )
                count[dep_path] += 1
                examples[dep_path] = tree.sentence_text
            # print(dep_path)
            # print(tree.find_path(tree.get_word(match['Help']),
            #                     tree.get_word(match['V'])))
    #
    #
    # print(tree.sentence_text)
    # print()
    # print(tree.get_word(match['Go']).form, tree.get_word(match['V']).form)
    # print(tree.get_word(match['Go']))
    # print(tree.get_word(match['And']))
    # p#rint(tree.get_word(match['V']))
    for k, v in count.most_common(25):
        print(v, k)
        print(examples[k])
        print()


if __name__ == "__main__":
    main()


def x_main(database: str) -> None:
    db = Database(database)
    print(
        "\t".join(
            (
                "linker",
                "verb_form",
                "verb_lemma",
                "deprel",
                "xcomp_form",
                "xcomp_lemma",
                "filename",
                "sentence",
            )
        )
    )
    for s in db.read_sentences():
        for tok in s.tokens:
            # if tok.upos == "VERB" and (tok.lemma == "come" or tok.lemma == "go"):
            if tok.upos == "VERB" and tok.text.lower() == tok.lemma.lower():
                verb_deps = s.get_dependents(tok.id)
                args = verb_deps.get("xcomp", [])
                linker = False
                if not args or args[0] != tok.id + 1:
                    args = verb_deps.get("conj", [])
                    linker = True
                    if not args or args[0] != tok.id + 2:
                        continue

                xcomp_tok = s.get_by_id(args[0])
                if xcomp_tok.upos == "VERB" and xcomp_tok.text.lower() == xcomp_tok.lemma.lower():
                    print(
                        "\t".join(
                            (
                                str(linker),
                                tok.text.lower(),
                                tok.lemma.lower(),
                                tok.deprel,
                                xcomp_tok.text.lower(),
                                xcomp_tok.lemma.lower(),
                                s.filename,
                                " ".join(t.text for t in s.tokens),
                            )
                        )
                    )


# @click.command()
# @click.option("--database", required=True, type=str, help="SQLite database file")
# @click.option("--loglevel", type=str, default="info")
# def goget(**args):
#     logger.setLevel(args["loglevel"].upper())
#     main(args["database"])
