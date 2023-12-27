use std::{path::PathBuf, time::Instant};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use offdictd::{topk::Strprox, Indexer, fst_index::fstmmap};
use rand::{seq::SliceRandom, thread_rng};

const DATA: &str = "../data";

pub fn criterion_benchmark(c: &mut Criterion) {
    let dir = PathBuf::from(DATA);
    let pp = dir.join(Strprox::FILE_NAME);
    unsafe { STRPROX = Some(Strprox::load_file(&pp).unwrap()) };
    let brw: <Strprox as Indexer>::Brw =
        bincode::deserialize(unsafe { &STRPROX.as_ref().unwrap().file }).unwrap();
    let mut rng = thread_rng();
    c.bench_function("newalgo", |b| {
        b.iter_custom(|iters| {
            let word = brw
                .trie
                .strings
                .choose_multiple(&mut rng, iters.try_into().unwrap());
            let start = Instant::now();
            for q in word {
                brw.autocomplete(&q, 3);
            }
            start.elapsed()
        })
    });

    let fst = fstmmap::load_file(&dir.join(fstmmap::FILE_NAME)).unwrap();
    c.bench_function("fst", |b| {
        b.iter_custom(|iters| {
            let word = brw
                .trie
                .strings
                .choose_multiple(&mut rng, iters.try_into().unwrap());
            let start = Instant::now();
            for q in word {
                fst.query(&q, false, &()).unwrap();
            }
            start.elapsed()
        })
    });
}

static mut STRPROX: Option<Strprox> = None;

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
