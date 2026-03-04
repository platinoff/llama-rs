//! Benchmarks for Llama-RS (ultra-speed verification).
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llama_rs::hello_llama_rust;

fn bench_hello(c: &mut Criterion) {
    c.bench_function("hello_llama_rust", |b| {
        b.iter(|| black_box(hello_llama_rust()))
    });
}

criterion_group!(benches, bench_hello);
criterion_main!(benches);
