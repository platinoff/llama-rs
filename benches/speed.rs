//! Benchmarks for llama.rs (ultra-speed verification).
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
    let params = llama_rs::ModelParams::default();
    let model = llama_rs::Model::load_from_file(&backend, path, &params).expect("load model");
    let ctx_params = llama_rs::ContextParams::default();
    let mut context = model.new_context(&backend, ctx_params).expect("context");
    let opts = llama_rs::GenerateOptions::builder()
        .max_tokens(32)
        .stop_at_eos(true)
        .build();

    c.bench_function("inference_tokens_per_sec", |b| {
        b.iter(|| {
            black_box(
                llama_rs::generate(&model, &mut context, "One two three.", &opts)
                    .expect("generate"),
            )
        });
    });
}

/// Time from start of generation to first decoded token (user-visible latency). Runs when LLAMA_RS_BENCH_MODEL is set.
fn bench_time_to_first_token(c: &mut Criterion) {
    let path = match std::env::var("LLAMA_RS_BENCH_MODEL") {
        Ok(p) if !p.is_empty() => p,
        _ => return,
    };
    let path = Path::new(&path);
    if !path.exists() {
        return;
    }

    let backend = llama_rs::Backend::init().expect("backend");
    let params = llama_rs::ModelParams::default();
    let model = llama_rs::Model::load_from_file(&backend, path, &params).expect("load model");
    let ctx_params = llama_rs::ContextParams::default();
    let mut context = model.new_context(&backend, ctx_params).expect("context");
    let opts = llama_rs::GenerateOptions::builder()
        .max_tokens(8)
        .stop_at_eos(true)
        .build();

    c.bench_function("time_to_first_token", |b| {
        b.iter(|| {
            let first = std::sync::atomic::AtomicBool::new(true);
            let ttf = std::sync::atomic::AtomicU64::new(0);
            let start = std::time::Instant::now();
            let _ = llama_rs::generate_stream(
                &model,
                &mut context,
                "Hi",
                &opts,
                |_| {
                    if first.swap(false, std::sync::atomic::Ordering::Relaxed) {
                        ttf.store(
                            start.elapsed().as_millis() as u64,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                    }
                },
            )
            .expect("generate_stream");
            black_box(ttf.load(std::sync::atomic::Ordering::Relaxed))
        });
    });
}

criterion_group!(benches, bench_hello, bench_inference_tokens_per_sec, bench_time_to_first_token);
criterion_main!(benches);
