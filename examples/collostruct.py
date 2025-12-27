import treesearch
import polars as pl


def xcomps():
    xcomp_query = """
    Head [upos="VERB"];
    XComp [upos="VERB" & feats.VerbForm="Inf"];
    Head -[xcomp]-> XComp;
    """

    data = []
    path = "/Volumes/Corpora/CCOHA/conll/*.conllu.gz"
    pattern = treesearch.compile_query(xcomp_query)
    for tree, match in treesearch.search_files(path, pattern):
        main = tree.get_word(match["Head"])
        xcomp = tree.get_word(match["XComp"])
        data.append({"head_lemma": main.lemma, "xcomp_lemma": xcomp.lemma})
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
    Head [upos="VERB" & lemma="help"];
    XComp [upos="VERB" & feats.VerbForm="Inf"];
    Head -[xcomp]-> XComp;
    Head !-[aux:pass]-> _;
    _ !-[conj]-> Head;
    Head !-[conj]-> _;
    XComp !-[conj]-> _;
    Head << XComp;
    """

    path = "/Volumes/Corpora/CCOHA/conll/*.conllu.gz"
    data = []
    pattern = treesearch.compile_query(help_query)
    for tree, match in treesearch.search_files(path, pattern):
        head = tree.get_word(match["Head"])
        xcomp = tree.get_word(match["XComp"])
        data.append(
            {
                "head_form": head.form.lower(),
                "transitive": check_dep(tree, head, "obj") or check_dep(tree, xcomp, "nsubj"),
                "head_to": check_dep(tree, head, "mark", tag="TO"),
                "head_aux": check_dep(tree, head, "aux"),
                "xcomp_lemma": xcomp.lemma,
                "bare_inf": not check_dep(tree, xcomp, "mark", tag="TO"),
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


if __name__ == "__main__":
    xcomps()
    helps()
