import treesearch
import polars as pl
import time

def xcomps():
    xcomp_query = """
    MATCH {
        Head [upos="VERB"];
        XComp [upos="VERB" & feats.VerbForm="Inf"];
        Head -[xcomp]-> XComp;
    }
    """

    data = []
    path = "/Volumes/Corpora/CCOHA/conll/*.conllu.gz"
    pattern = treesearch.compile_query(xcomp_query)

    treebank = treesearch.load(path)
    for tree, match in treebank.search(pattern, ordered=False):
        head = tree[match["Head"]]
        xcomp = tree[match["XComp"]]
        data.append({"head_lemma": head.lemma,
                     "xcomp_lemma": xcomp.lemma,
                     "transitive": check_dep(tree, head, "obj") or check_dep(tree, xcomp, "nsubj"),
                     "doc_id": tree.metadata["doc_id"],
                     })
    df = pl.DataFrame(data)
    df.write_parquet("xcomps.parquet")


def check_dep(tree, node, deprel, tag=None):
    deps = node.children_by_deprel(deprel)
    for dep in deps:
        if tag is None or dep.xpos == tag:
            return True
    return False


def helps():
    help_query = """
    MATCH {
        Head [upos="VERB" & lemma="help"];
        XComp [upos="VERB" & feats.VerbForm="Inf"];
        Head -[xcomp]-> XComp;
        Head !-[aux:pass]-> _;
        _ !-[conj]-> Head;
        Head !-[conj]-> _;
        _ !-[conj]-> XComp;
        XComp !-[conj]-> _;
        Head << XComp;
    }
    OPTIONAL {
        HeadTo [lemma="to"];
        Head -[mark]-> HeadTo;
    }
    OPTIONAL {
        XCompTo [lemma="to"];
        XComp -[mark]-> XCompTo;
    }
    OPTIONAL {
        XCompNeg [lemma="not"];
        XComp -[advmod]-> XCompNeg;
    }
    OPTIONAL {
        HeadNeg [lemma="not"];
        Head -[advmod]-> HeadNeg;
    }
    """

    path = "/Volumes/Corpora/CCOHA/conll/*.conllu.gz"
    data = []
    pattern = treesearch.compile_query(help_query)

    treebank = treesearch.load(path)
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
    df.write_parquet("help.parquet")

def dares():
    dare_query = """
   MATCH {
        Head [upos="VERB" & lemma="dare"];
        XComp [upos="VERB" & feats.VerbForm="Inf"];
        Head -[xcomp]-> XComp;
        Head !-[aux:pass]-> _;
        Head !-[obj]-> _;
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
        Head -[mark]-> HeadTo;
    }
    OPTIONAL {
        XCompTo [lemma="to"];
        XComp -[mark]-> XCompTo;
    }
    OPTIONAL {
        XCompNeg [lemma="not"];
        XComp -[advmod]-> XCompNeg;
    }
    OPTIONAL {
        HeadNeg [lemma="not"];
        Head -[advmod]-> HeadNeg;
    }
     """

    path = "/Volumes/Corpora/CCOHA/conll/*.conllu.gz"
    data = []
    pattern = treesearch.compile_query(dare_query)

    treebank = treesearch.load(path)
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
    print('xcomps')
    xcomps()
    print('helps')
    helps()
    print('dares')
    dares()