use divan::AllocProfiler;
use divan::{Bencher, black_box};
use treesearch::Treebank;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
// static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    divan::main();
}

#[divan::bench(sample_count = 3)]
fn tree_iter_ordered_multi(bencher: Bencher) {
    let treebank =
        Treebank::from_glob("/Volumes/Corpora/COHA/conll_gz/text_*_19[0-5]0.conllu.gz").unwrap();
    bencher.bench_local(|| {
        let count = black_box(treebank.clone().tree_iter(true).count());
        black_box(count);
    });
}

#[divan::bench(sample_count = 3)]
fn tree_iter_unordered_multi(bencher: Bencher) {
    let treebank =
        Treebank::from_glob("/Volumes/Corpora/COHA/conll_gz/text_*_19[0-5]0.conllu.gz").unwrap();
    bencher.bench_local(|| {
        let count = black_box(treebank.clone().tree_iter(false).count());
        black_box(count);
    });
}
