use divan::AllocProfiler;
use divan::{Bencher, black_box};
//use std::path::Path;
// use treesearch::conllu::TreeIterator;
use treesearch::{Treebank, compile_query};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
// static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    divan::main();
}

/*
/// Benchmark parsing the lw970831.conll file
#[divan::bench(sample_count = 5)]
fn parse_plain(bencher: Bencher) {
    let path = Path::new("examples/text_fic_2010.conll");
    bencher.bench_local(|| {
        let reader = TreeIterator::from_file(black_box(path)).unwrap();
        for result in reader {
            black_box(result.unwrap());
        }
    });
}
*/

/*
#[divan::bench(sample_count = 5)]
    let path = Path::new("examples/text_fic_2010.conll.gz");
    bencher.bench_local(|| {
        let reader = TreeIterator::from_file(black_box(path)).unwrap();
        for result in reader {
            black_box(result.unwrap());
        }
    });
}
*/

/*
// Benchmark ordered vs unordered tree iteration - single file
#[divan::bench(sample_count = 5)]
fn tree_iter_ordered_single(bencher: Bencher) {
    bencher.bench_local(|| {
        let treebank = Treebank::from_file(Path::new("examples/text_fic_2010.conll"));
        let count = black_box(treebank.tree_iter(true).count());
        black_box(count);
    });
}
 */

/*
#[divan::bench(sample_count = 5)]
fn tree_iter_unordered_single(bencher: Bencher) {
    bencher.bench_local(|| {
        let treebank = Treebank::from_file(Path::new("examples/text_fic_2010.conll"));
        let count = black_box(treebank.tree_iter(false).count());
        black_box(count);
    });
}
*/

/*
// Benchmark ordered vs unordered tree iteration - heterogeneous mix
// 27 files ranging from 15M to 133M across all genres (1900-1950)
#[divan::bench(sample_count = 3)]
fn tree_iter_ordered_multi(bencher: Bencher) {
    let treebank = Treebank::from_glob("/Volumes/Corpora/COHA/conll/text_*_19[0-5]0.conllu.gz").unwrap();
    bencher.bench_local(|| {
        let count = black_box(treebank.clone().tree_iter(true).count());
        black_box(count);
    });
}
 */

/*
#[divan::bench(sample_count = 3)]
fn tree_iter_unordered_multi(bencher: Bencher) {
    let treebank = Treebank::from_glob("/Volumes/Corpora/COHA/conll/text_*_19[0-5]0.conllu.gz").unwrap();
    bencher.bench_local(|| {
        let count = black_box(treebank.clone().tree_iter(false).count());
        black_box(count);
    });
}
 */

/*
// Benchmark ordered vs unordered pattern matching - single file
#[divan::bench(sample_count = 5)]
fn match_iter_ordered_single(bencher: Bencher) {
    let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
    bencher.bench_local(|| {
        let treebank = Treebank::from_file(Path::new("examples/text_fic_2010.conll"));
        let count = black_box(treebank.match_iter(pattern.clone(), true).count());
        black_box(count);
    });
}
*/

/*
#[divan::bench(sample_count = 5)]
fn match_iter_unordered_single(bencher: Bencher) {
    let pattern = parse_query("MATCH { V [upos=\"VERB\"]; }").unwrap();
    bencher.bench_local(|| {
        let treebank = Treebank::from_file(Path::new("examples/text_fic_2010.conll"));
        let count = black_box(treebank.match_iter(pattern.clone(), false).count());
        black_box(count);
    });
}
*/

// Benchmark ordered vs unordered pattern matching - heterogeneous mix
// 27 files ranging from 15M to 133M across all genres (1900-1950)
#[divan::bench(sample_count = 3)]
fn match_iter_ordered_multi(bencher: Bencher) {
    let pattern = compile_query("MATCH { V [upos=\"VERB\"]; N1[upos=\"NOUN\"]; N2[upos=\"NOUN\"]; P[upos=\"ADP\"];V -[obj]->N1; N1-[nmod]->N2; N2-[case]->P;}").unwrap();
    // let pattern = parse_query("MATCH { V [upos=\"VERB\"];}").unwrap();

    let treebank =
        Treebank::from_glob("/Volumes/Corpora/COHA/conll_gz/text_*_19[0-5]0.conllu.gz").unwrap();
    bencher.bench_local(|| {
        let count = black_box(treebank.clone().match_iter(pattern.clone(), true).count());
        //dbg!(count);
        black_box(count);
    });
}

#[divan::bench(sample_count = 3)]
fn match_iter_unordered_multi(bencher: Bencher) {
    let pattern = compile_query("MATCH { V [upos=\"VERB\"]; N1[upos=\"NOUN\"]; N2[upos=\"NOUN\"]; P[upos=\"ADP\"];V -[obj]->N1; N1-[nmod]->N2; N2-[case]->P;}").unwrap();
    let treebank =
        Treebank::from_glob("/Volumes/Corpora/COHA/conll_gz/text_*_19[0-5]0.conllu.gz").unwrap();
    bencher.bench_local(|| {
        let count = black_box(treebank.clone().match_iter(pattern.clone(), false).count());
        //dbg!(count);
        black_box(count);
    });
}
