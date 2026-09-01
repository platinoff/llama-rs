//! Benchmarks for llama.rs — llama-bench style (pp/tg/TTFT).
//!
//! Run with: cargo bench
//! Optional: set `LLAMA_RS_BENCH_MODEL` to a GGUF path to run inference benchmarks.
//! Qwen default locally: `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` (6.8G, GDN hybrid).
//! For Nemotron swap via env: `LLAMA_RS_BENCH_MODEL=/path/to/Nemotron.gguf cargo bench`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llama_rs::hello_llama_rust;
use std::path::Path;

fn bench_hello(c: &mut Criterion) {
    c.bench_function("hello_llama_rust", |b| {
        b.iter(|| black_box(hello_llama_rust()));
    });
}

/// Model benches (pp/tg/TTFT) share one backend/model — Backend::init is once per process.
fn bench_model_group(c: &mut Criterion) {
    let path = match std::env::var("LLAMA_RS_BENCH_MODEL") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            eprintln!("skip model benches: LLAMA_RS_BENCH_MODEL not set (use Qwen locally)");
            return;
        }
    };
    let path = Path::new(&path);
    if !path.exists() {
        eprintln!("skip model benches: not found {}", path.display());
        return;
    }
    let backend = match llama_rs::Backend::init() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("skip model benches: backend init failed: {}", e);
            return;
        }
    };
    let params = llama_rs::ModelParams::default();
    let model = llama_rs::Model::load_from_file(&backend, path, &params)
        .unwrap_or_else(|e| panic!("load {}: {}", path.display(), e));

    // pp: prompt processing — long prompt, 1 token gen (measures prefill)
    {
        let ctx_params = llama_rs::ContextParams::default();
        let mut ctx = model.new_context(&backend, ctx_params).expect("context");
        let prompt = "The quick brown fox jumps over the lazy dog. ".repeat(32); // ~256 tokens
        let opts = llama_rs::GenerateOptions::builder()
            .max_tokens(1)
            .stop_at_eos(true)
            .build();
        c.bench_function("pp_256", |b| {
            b.iter(|| {
                ctx.reset();
                black_box(llama_rs::generate(&model, &mut ctx, &prompt, &opts).expect("generate"))
            })
        });
    }

    // tg: token generation — short generation 32 tokens (tg tok/s = 32 / time)
    {
        let ctx_params = llama_rs::ContextParams::default();
        let mut ctx = model.new_context(&backend, ctx_params).expect("context");
        let opts = llama_rs::GenerateOptions::builder()
            .max_tokens(32)
            .stop_at_eos(true)
            .build();
        c.bench_function("tg_32", |b| {
            b.iter(|| {
                ctx.reset();
                black_box(
                    llama_rs::generate(&model, &mut ctx, "One two three.", &opts)
                        .expect("generate"),
                )
            })
        });
    }

    // TTFT: time to first token
    {
        let ctx_params = llama_rs::ContextParams::default();
        let mut ctx = model.new_context(&backend, ctx_params).expect("context");
        let opts = llama_rs::GenerateOptions::builder()
            .max_tokens(8)
            .stop_at_eos(true)
            .build();
        c.bench_function("ttft", |b| {
            b.iter(|| {
                ctx.reset();
                let start = std::time::Instant::now();
                let mut ttft: Option<u64> = None;
                let _ = llama_rs::generate_stream(&model, &mut ctx, "Hi", &opts, |_| {
                    if ttft.is_none() {
                        ttft = Some(start.elapsed().as_millis() as u64);
                    }
                })
                .expect("generate_stream");
                black_box(ttft.unwrap_or(0))
            })
        });
    }
}

criterion_group!(benches, bench_hello, bench_model_group);
criterion_main!(benches);
