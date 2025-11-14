use divan::{Bencher, black_box};
use std::path::Path;
use treesearch::conllu::CoNLLUReader;
use divan::AllocProfiler;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

/// Benchmark parsing the lw970831.conll file
#[divan::bench]
fn parse_lw970831(bencher: Bencher) {
    let path = Path::new("examples/lw970831.conll");
    bencher.bench_local(|| {
        let reader = CoNLLUReader::from_file(black_box(path)).unwrap();
        for result in reader {
            black_box(result.unwrap());
        }
    });
}
