use rayon::prelude::*;
use treesearch::{MatchSet, parse_query};

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
    let count = MatchSet::from_glob(path, pattern)
        .unwrap()
        .into_par_iter()
        .count();

    println!("{}", count);
}
