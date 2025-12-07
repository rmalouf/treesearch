import marimo

__generated_with = "0.18.1"
app = marimo.App(width="medium")


@app.cell
def _():
    import polars as pl
    import polars_corpus as plc
    import treesearch
    from collections import Counter

    return Counter, pl, plc, treesearch


@app.cell
def _(pl):
    df = pl.read_parquet("xcomps.parquet")
    return (df,)


@app.cell
def _(df):
    df.head()
    return


@app.cell
def _(df):
    table = df.corpus.crosstab("head_lemma", "xcomp_lemma")
    return (table,)


@app.cell
def _(pl, plc, table):
    table.with_columns(ll=plc.loglik("f12", "f1", "f2", "n")).sort(by="ll", descending=True).filter(
        pl.col("f12") > pl.col("f1") * pl.col("f2") / pl.col("n")
    )
    return


@app.cell
def _(Counter, treesearch):
    query = 'Verb [upos="VERB"];'
    path = "/Volumes/Corpora/CCOHA/conll/*.conllu.gz"
    pattern = treesearch.parse_query(query)
    verbs = Counter(
        tree.get_word(match["Verb"]).lemma for tree, match in treesearch.search_files(path, pattern)
    )
    return (verbs,)


@app.cell
def _(verbs):
    len(verbs)
    return


@app.cell
def _():
    return


if __name__ == "__main__":
    app.run()
