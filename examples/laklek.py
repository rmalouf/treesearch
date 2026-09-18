import treesearch
import polars as pl


PATH = "/Volumes/Corpora/HPLT3.0/hun_parsed/**/*.conllu.gz"
#PATH = "/Volumes/Corpora/HPLT3.0/hun_parsed/5_1/*_0*.conllu.gz"

QUERY = """
MATCH {
	Verb [upos="VERB" & form=/.*l[ae]k$/];
    Head [upos="VERB"];
	Head -[xcomp]-> Verb;
}
"""

data = []
pattern = treesearch.compile_query(QUERY)

data = []
treebank = treesearch.load(PATH)
for tree, match in treebank.search(pattern, ordered=False):
    head = tree[match["Head"]]
    verb = tree[match["Verb"]]
    data.append({'head_form': head.form,
                 'head_lemma': head.lemma,
                 'verb_form': verb.form,
                 'verb_lemma': verb.lemma,
                 'text': tree.sentence_text,
                 })

pl.DataFrame(data).unique('text').write_csv('laklek.tsv', separator='\t', quote_style="never")

QUERY = """
MATCH {
    Verb [upos="VERB" & feats.VerbForm="Inf" & feats.Person="1"];
    Head [upos="VERB"];
	Head -[xcomp]-> Verb;
	Obj [upos="PRON" & feats.Person="2"];
	Verb -[obj]-> Obj;
	
}
"""

data = []
pattern = treesearch.compile_query(QUERY)

data = []
treebank = treesearch.load(PATH)
for tree, match in treebank.search(pattern, ordered=False):
    head = tree[match["Head"]]
    verb = tree[match["Verb"]]
    obj = tree[match["Obj"]]
    data.append({'head_form': head.form,
                 'head_lemma': head.lemma,
                 'verb_form': verb.form,
                 'verb_lemma': verb.lemma,
                 'obj_form': obj.form,
                 'text': tree.sentence_text,
                 })

pl.DataFrame(data).unique('text').write_csv('m.tsv', separator='\t', quote_style="never")
