use std::{path::PathBuf, time::Instant};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use offdictd::{
    fst_index::fstmmap,
    topk::{Strprox, TopkParam},
    Indexer,
};
use rand::{seq::SliceRandom, thread_rng};

const DATA: &str = "../data";

pub fn criterion_benchmark(c: &mut Criterion) {
    let dir = PathBuf::from(DATA);
    let pp = dir.join(Strprox::FILE_NAME);
    let strpr = Strprox::load_file(&pp).unwrap();
    let mut rng = thread_rng();
    c.bench_function("newalgo", |b| {
        b.iter_custom(|iters| {
            let word = strpr
                .yoke
                .get()
                .trie
                .strings
                .choose_multiple(&mut rng, iters.try_into().unwrap());
            let start = Instant::now();
            for q in word {
                strpr.query(&q, TopkParam::new(2));
            }
            start.elapsed()
        })
    });

    let fst = fstmmap::load_file(&dir.join(fstmmap::FILE_NAME)).unwrap();
    c.bench_function("fst", |b| {
        b.iter_custom(|iters| {
            let word = strpr
                .yoke
                .get()
                .trie
                .strings
                .choose_multiple(&mut rng, iters.try_into().unwrap());
            let start = Instant::now();
            for q in word {
                fst.query(&q, false).unwrap();
            }
            start.elapsed()
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
