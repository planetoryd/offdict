use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use offdictd::{topk::Strprox, Indexer};

const DATA: &str = "../data";

pub fn criterion_benchmark(c: &mut Criterion) {
    let pp = PathBuf::from(DATA).join(Strprox::FILE_NAME);
    unsafe { STRPROX = Some(Strprox::load_file(&pp).unwrap()) };
    let brw: <Strprox as Indexer>::Brw = bincode::deserialize(unsafe { &STRPROX.as_ref().unwrap().file }).unwrap();
}

static mut STRPROX: Option<Strprox> = None;

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
