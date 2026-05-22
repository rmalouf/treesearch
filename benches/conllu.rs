use divan::AllocProfiler;
use divan::{Bencher, black_box};
use std::path::Path;
use treesearch::conllu::TreeIterator;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// #[divan::bench]
// fn parse_plain(bencher: Bencher) {
//     let path = Path::new("examples/text_fic_2010.conll");
//     bencher.bench_local(|| {
//         let reader = TreeIterator::from_file(black_box(path)).unwrap();
//         for result in reader {
//             black_box(result.unwrap());
//         }
//     });
// }

#[divan::bench]
fn parse_gz(bencher: Bencher) {
    let path = Path::new("examples/text_fic_2010.conll.gz");
    bencher.bench_local(|| {
        let reader = TreeIterator::from_file(black_box(path)).unwrap();
        for result in reader {
            black_box(result.unwrap());
        }
    });
}
