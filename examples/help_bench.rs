use treesearch::{Treebank, parse_query};

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
    let treebank = Treebank::from_glob(path).unwrap();
    // Note: parallel processing is now handled internally by match_iter()
    let count = treebank.match_iter(pattern, true).count();

    println!("{}", count);
}
