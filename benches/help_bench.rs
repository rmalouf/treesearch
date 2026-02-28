use divan::AllocProfiler;
use divan::{Bencher, black_box};
use treesearch::Treebank;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();
//static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
//static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    divan::main();
}

#[divan::bench(sample_count = 3)]
fn help(bencher: Bencher) {
    let query = r#"   MATCH {
        Head [upos="VERB" & lemma="help"];
        XComp [upos="VERB" & feats.VerbForm="Inf"];
        Head -[xcomp]-> XComp;
        Head !-[aux:pass]-> _;
        _ !-[conj]-> Head;
        Head !-[conj]-> _;
        _ !-[conj]-> XComp;
        XComp !-[conj]-> _;
        Head << XComp;
    }
    OPTIONAL {
        HeadTo [lemma="to"];
        Head -[mark]-> HeadTo;
    }
    OPTIONAL {
        XCompTo [lemma="to"];
        XComp -[mark]-> XCompTo;
    }
    OPTIONAL {
        XCompNeg [lemma="not"];
        XComp -[advmod]-> XCompNeg;
    }
    OPTIONAL {
        HeadNeg [lemma="not"];
        Head -[advmod]-> HeadNeg;
    }
    "#;

    let path = "/Volumes/Corpora/COHA/conll_gz/*_1950.conllu.gz";
    bencher.bench_local(|| {
        let treebank = Treebank::from_glob(path).unwrap();
        for result in treebank.search(query, false).unwrap() {
            let _ = black_box(result);
        }
    });
}
