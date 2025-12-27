//use divan::AllocProfiler;
use divan::{Bencher, black_box};
use std::path::Path;
use treesearch::conllu::TreeIterator;
use treesearch::{Treebank, compile_query};

#[global_allocator]
//static ALLOC: AllocProfiler = AllocProfiler::system();
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
// static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    let pattern = compile_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
    let treebank =
        Treebank::from_glob("/Volumes/Corpora/COHA/conll/text_*_19[0-5]0.conllu.gz").unwrap();
    let count = black_box(treebank.clone().match_iter(pattern.clone(), true).count());
    black_box(count);
}
