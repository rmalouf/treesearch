## collect data for help_to paper
##   20-Mar-2026

import treesearch
import polars as pl
import time

PATH = "/Volumes/Corpora/CCOHA/conllu/*.conllu.gz"

# VERB_QUERY = """
# MATCH {
#     Head [upos="VERB"];
#     XComp [upos="VERB" & feats.VerbForm="Inf"];
#     Head -/[xc]comp/-> XComp;
# }
# """

VERB_QUERY = """
MATCH {
    Head [upos="VERB"];
}
"""

def verbs():
    data = []
    pattern = treesearch.compile_query(VERB_QUERY)

    treebank = treesearch.load(PATH)
    for tree, match in treebank.search(pattern, ordered=False):
        head = tree[match["Head"]].lemma
        doc_id = tree.metadata["doc_id"]
        data.append({"verb": head, "year": doc_id})

        if head == "clearp":
            print(doc_id)
            print(tree.sentence_text)
            print(tree.metadata["sent_id"])

    df = (
        pl.DataFrame(data)
        .with_columns(pl.col("year").str.extract(r"_([0-9]+)", group_index=1))
        .sort("year")
    )
    df.write_parquet("verbs.parquet")


def check_dep(tree, node, deprel, tag=None):
    deps = node.children_by_deprel(deprel)
    for dep in deps:
        if tag is None or dep.xpos == tag:
            return True
    return False


HELP_QUERY = """
MATCH {
    Head [upos="VERB" & lemma="help"];
    XComp [upos="VERB" & feats.VerbForm="Inf"];
    Head -/[cx]comp/-> XComp;
    Head !-[auxpass]-> _;
    _ !-[conj]-> Head;
    Head !-[conj]-> _;
    _ !-[conj]-> XComp;
    XComp !-[conj]-> _;
    Head << XComp;
}
EXCEPT {
    But [lemma="but"];
    XComp -[cc]-> But;
    Head !-[neg]-> _;
}
OPTIONAL {
    HeadTo [lemma="to"];
    Head -[aux]-> HeadTo;
}
OPTIONAL {
    XCompTo [lemma="to"];
    XComp -[aux]-> XCompTo;
}
OPTIONAL {
    XCompNeg [lemma="not"];
    XComp -[neg]-> XCompNeg;
}
OPTIONAL {
    HeadNeg [lemma="not"];
    Head -[neg]-> HeadNeg;
}
"""


def helps():
    data = []
    pattern = treesearch.compile_query(HELP_QUERY)

    treebank = treesearch.load(PATH)
    for tree, match in treebank.search(pattern, ordered=False):
        head = tree[match["Head"]]
        xcomp = tree[match["XComp"]]
        head_neg = "HeadNeg" in match
        xcomp_neg = "XCompNeg" in match

        data.append(
            {
                "head_form": head.form.lower(),
                "transitive": check_dep(tree, head, "dobj") or check_dep(tree, xcomp, "nsubj"),
                "head_to": "HeadTo" in match,
                "head_aux": check_dep(tree, head, "aux"),
                "head_neg": head_neg,
                "xcomp_neg": xcomp_neg,
                "xcomp_lemma": xcomp.lemma,
                "bare_inf": "XCompTo" not in match,
                "xcomp_transitive": check_dep(tree, xcomp, "dobj")
                or check_dep(tree, xcomp, "ccomp"),
                "distance": int(xcomp.id - head.id),
                "doc_id": tree.metadata["doc_id"],
                "sent_id": tree.metadata["sent_id"],
                "text": tree.sentence_text,
            }
        )
    df = pl.DataFrame(data)
    df.write_parquet("help.parquet")


DARE_QUERY = """
MATCH {
    Head [upos="VERB" & lemma="dare"];
    XComp [upos="VERB" & feats.VerbForm="Inf"];
    Head -/[xc]comp/-> XComp;
    Head !-[auxpass]-> _;
    Head !-[dobj]-> _;
    XComp !-[nsubj]-> _;
    _ !-[conj]-> Head;
    Head !-[conj]-> _;
    _ !-[conj]-> XComp;
    XComp !-[conj]-> _;
    Head << XComp;        
}
EXCEPT {
    Head [form="dare"];
    XComp [form="say"];
}
OPTIONAL {
    HeadTo [lemma="to"];
    Head -[aux]-> HeadTo;
}
OPTIONAL {
    XCompTo [lemma="to"];
    XComp -[aux]-> XCompTo;
}
OPTIONAL {
    XCompNeg [lemma="not"];
    XComp -[neg]-> XCompNeg;
}
OPTIONAL {
    HeadNeg [lemma="not"];
    Head -[neg]-> HeadNeg;
}
"""


def dares():

    data = []
    pattern = treesearch.compile_query(DARE_QUERY)

    treebank = treesearch.load(PATH)
    for tree, match in treebank.search(pattern, ordered=False):
        head = tree[match["Head"]]
        xcomp = tree[match["XComp"]]
        head_neg = "HeadNeg" in match
        xcomp_neg = "XCompNeg" in match
        data.append(
            {
                "head_form": head.form.lower(),
                "transitive": check_dep(tree, head, "obj") or check_dep(tree, xcomp, "nsubj"),
                "head_to": "HeadTo" in match,
                "head_aux": check_dep(tree, head, "aux"),
                "head_neg": head_neg,
                "xcomp_neg": xcomp_neg,
                "xcomp_lemma": xcomp.lemma,
                "bare_inf": "XCompTo" not in match,
                "xcomp_transitive": check_dep(tree, xcomp, "obj")
                or check_dep(tree, xcomp, "ccomp"),
                "distance": int(xcomp.id - head.id),
                "doc_id": tree.metadata["doc_id"],
                "sent_id": tree.metadata["sent_id"],
                "text": tree.sentence_text,
            }
        )
    df = pl.DataFrame(data)
    print(len(df))
    df.write_parquet("dare.parquet")


if __name__ == "__main__":
    print("verbs")
    verbs()
    print("helps")
    helps()
    print("dares")
    dares()
