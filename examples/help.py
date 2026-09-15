## collect data for help_to paper
##   07-Jul-2026

import treesearch
import polars as pl
import time

PATH = "/Volumes/Corpora/CCOHA/conllu/*.conllu.gz"

VERB_QUERY = """
MATCH {
    Head [upos="VERB"];
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
    Head << But;
    But << XComp;
}
OPTIONAL {
    HeadTo [upos="PART" & lemma="to"];
    Head -> HeadTo;
    HeadTo << Head;
}
OPTIONAL {
    XCompTo [upos="PART" & lemma="to"];
    XComp -> XCompTo;
    XCompTo << XComp;
}
"""

def verbs():
    data = []
    pattern = treesearch.compile_query(VERB_QUERY)

    treebank = treesearch.load(PATH)
    for tree, match in treebank.search(pattern, ordered=False):
        head = tree[match["Head"]]
        xcomp = tree[match["XComp"]]
        data.append(
            {
                "head_form": head.form.lower(),
                "head_lemma": head.lemma.lower(),
                "transitive": check_dep(tree, head, "dobj") or check_dep(tree, xcomp, "nsubj"),
                "head_to": "HeadTo" in match,
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

    df = (
        pl.DataFrame(data)
        .with_columns(pl.col("doc_id").str.extract(r"_([0-9]+)", group_index=1).alias("year"))
        .sort("year")
    )
    df.write_parquet("verbs.parquet")




def get_dep(tree, node, deprel, tag=None):
    deps = node.children_by_deprel(deprel)
    for dep in deps:
        if tag is None or dep.xpos == tag:
            return dep
    return None

def check_dep(tree, node, deprel, tag=None):
    return get_dep(tree, node, deprel, tag) is not None



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
    Head << But;
    But << XComp;
}
EXCEPT {
    ItSubj [lemma="it"];
    Head -[nsubj]-> ItSubj;
}
OPTIONAL {
    HeadTo [upos="PART" & lemma="to"];
    Head -> HeadTo;
    HeadTo << Head;
}
OPTIONAL {
    XCompTo [upos="PART" & lemma="to"];
    XComp -> XCompTo;
    XCompTo << XComp;
}
"""


def helps():
    data = []
    pattern = treesearch.compile_query(HELP_QUERY)

    treebank = treesearch.load(PATH)
    for tree, match in treebank.search(pattern, ordered=False):
        head = tree[match["Head"]]
        xcomp = tree[match["XComp"]]
        subj = get_dep(tree, head, "nsubj") 
        it_subj = subj is not None and subj.lemma == "it"
        #print(it_subj)
        data.append(
            {
                "head_form": head.form.lower(),
                "transitive": check_dep(tree, head, "dobj") or check_dep(tree, xcomp, "nsubj"),
                "head_to": "HeadTo" in match,
                "it_subj": it_subj,
                "xcomp_lemma": xcomp.lemma,
                "bare_inf": "XCompTo" not in match,
                "helpee_pron": check_dep(tree, head, "dobj", "PRP") or check_dep(tree, xcomp, "nsubj", "PRP"),
                "xcomp_transitive": check_dep(tree, xcomp, "dobj")
                or check_dep(tree, xcomp, "ccomp"),
                "distance": int(xcomp.id - head.id),
                "doc_id": tree.metadata["doc_id"],
                "sent_id": tree.metadata["sent_id"],
                "text": tree.sentence_text,
            }
        )
    df = (
        pl.DataFrame(data)
        .with_columns(pl.col("doc_id").str.extract(r"_([0-9]+)", group_index=1).alias("year"))
        .sort("year")
    )
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
#    print("verbs")
#    verbs()
    print("helps")
    helps()
#    print("dares")
#    dares()
