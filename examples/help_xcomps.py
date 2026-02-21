import treesearch
import polars as pl
import time

def verbs():
    xcomp_query = """
    MATCH {
        Head [upos="VERB"];
    }
    """

    data = []
    path = "/Volumes/Corpora/CCOHA/conll/*.conllu.gz"
    pattern = treesearch.compile_query(xcomp_query)

    treebank = treesearch.load(path)
    for tree, match in treebank.search(pattern, ordered=False):
        head = tree[match["Head"]].lemma
        doc_id = tree.metadata["doc_id"]
        data.append({'verb':head, 'year':doc_id})

        if head == 'clearp':
            print(doc_id)
            print(tree.sentence_text)
            print(tree.metadata['sent_id'])


    df = (
        pl.DataFrame(data)
            .with_columns(pl.col('year').str.extract(r'_([0-9]+)', group_index=1))
            .sort('year')
    )
    df.write_parquet("verbs.parquet")



if __name__ == "__main__":
    print('verbs')
    verbs()
