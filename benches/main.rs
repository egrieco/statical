// https://bheisler.github.io/criterion.rs/book/getting_started.html

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use statical::*;

pub fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_function("hello", |b| b.iter(|| black_box(hello())));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
