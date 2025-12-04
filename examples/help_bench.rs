use pariter::IteratorExt as _;
use treesearch::{MatchSet, Treebank, parse_query};

fn main() {
    let query = r#"MATCH {
    Head [upos="VERB", lemma="help"];
    XComp [upos="VERB", feats.VerbForm="Inf"];
    Head -[xcomp]-> XComp;
    Head !-[aux:pass]-> _;
    _ !-[conj]-> Head;
    Head !-[conj]-> _;
    XComp !-[conj]-> _;
    Head << XComp; }
    "#;

    let path = "/Volumes/Corpora/COHA/conll/*.conllu.gz";
    let pattern = parse_query(query).unwrap();
    let tree_set = Treebank::from_glob(path).unwrap();
    let count = MatchSet::new(&tree_set, &pattern)
        .into_iter()
        .parallel_map(|m| m)
        .count();

    println!("{}", count);
}
