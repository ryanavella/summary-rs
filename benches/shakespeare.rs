use std::num::NonZeroU32;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use summary::{Language, Summarizer};

pub fn criterion_benchmark(c: &mut Criterion) {
    const MAX: NonZeroU32 = NonZeroU32::MAX;
    let summarizer = Summarizer::new(Language::English);
    let text = include_str!("gutenberg/1513.txt");

    c.bench_function("shakespeare", |b| {
        b.iter(|| summarizer.summarize_sentences(black_box(text), MAX))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
