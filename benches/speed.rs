//! Benchmarks for Llama-RS (ultra-speed verification).
//!
//! Run with: cargo bench
//!
//! Optional: set `LLAMA_RS_BENCH_MODEL` to a GGUF path to run inference benchmark (tokens/sec).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llama_rs::hello_llama_rust;
use std::path::Path;

fn bench_hello(c: &mut Criterion) {
    c.bench_function("hello_llama_rust", |b| {
        b.iter(|| black_box(hello_llama_rust()))
    });
}

/// Measures time per short generation (tokens/sec ≈ 32 / time) when LLAMA_RS_BENCH_MODEL is set.
/// Model is loaded once; only generation is timed.
fn bench_inference_tokens_per_sec(c: &mut Criterion) {
    let path = match std::env::var("LLAMA_RS_BENCH_MODEL") {
        Ok(p) if !p.is_empty() => p,
        _ => return,
    };
    let path = Path::new(&path);
    if !path.exists() {
        return;
    }

    let backend = llama_rs::Backend::init().expect("backend");
    let params = llama_cpp_2::model::params::LlamaModelParams::default();
    let model = llama_rs::Model::load_from_file(&backend, path, &params).expect("load model");
    let ctx_params = llama_cpp_2::context::params::LlamaContextParams::default();
    let mut context = model.new_context(&backend, ctx_params).expect("context");
    let mut opts = llama_rs::GenerateOptions::default();
    opts.max_tokens = 32;
    opts.stop_at_eos = true;

    c.bench_function("inference_tokens_per_sec", |b| {
        b.iter(|| {
            black_box(
                llama_rs::generate(&model, &mut context, "One two three.", &opts)
                    .expect("generate"),
            )
        });
    });
}

criterion_group!(benches, bench_hello, bench_inference_tokens_per_sec);
criterion_main!(benches);
