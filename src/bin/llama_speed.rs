//! llama_speed — fast single-pass bench (llama-bench style: pp/tg/TTFT).
//!
//! One process loads the model once and runs one prompt + one generation,
//! reporting classic llama-bench phases via [`llama_rs::InferenceMetrics`].
//! Built to replace multi-hour criterion runs (criterion requires
//! `--sample-size >= 10`; a 27B model makes each sample ~15–17 min). Runtime
//! here is **minutes**, not hours. Output is either a human summary or one
//! JSON line ready for GSV live ingest.
//!
//! Usage (requires the `metrics` feature):
//! ```text
//! cargo run --bin llama_speed --features metrics -- --model <path.gguf> --json
//! cargo run --bin llama_speed --features metrics -- \
//!     --model models/Qwen3.8-27B-UD-IQ2_XXS.gguf \
//!     --n-ctx 2048 --n-batch 2048 --mmap --gen-tokens 32
//! ```
//! Model also resolves from `$LLAMA_RS_BENCH_MODEL` when `--model` is absent.

use clap::Parser;
use llama_rs::{Backend, ContextParams, GenerateOptions, Model, StagedLoadOptions};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "llama_speed")]
#[command(about = "Fast single-pass pp/tg/TTFT bench (llama-bench style)")]
struct Args {
    /// Path to the GGUF model file (falls back to $LLAMA_RS_BENCH_MODEL).
    #[arg(long)]
    model: Option<String>,

    /// Prompt text to prefill. Reported `pp tok/s` uses the actual token count.
    #[arg(long, default_value = "Hello")]
    prompt: String,

    /// Number of new tokens to generate (tg phase).
    #[arg(long, default_value_t = 32)]
    gen_tokens: u32,

    /// Context length (KV cache budget).
    #[arg(long, default_value_t = 512)]
    n_ctx: u32,

    /// Decode batch size (higher speeds up prefill of long prompts).
    #[arg(long, default_value_t = 512)]
    n_batch: u32,

    /// Thread count (default = llama.cpp auto: physical cores).
    #[arg(long)]
    threads: Option<i32>,

    /// Use mmap for model load (default true; paged, low-RAM).
    #[arg(long, default_value_t = true)]
    mmap: bool,

    /// Disable mmap (fully resident; faster when pages would be refaulted).
    #[arg(long, default_value_t = false)]
    no_mmap: bool,

    /// Pin model pages with mlock (needs privilege + enough RAM).
    #[arg(long, default_value_t = false)]
    mlock: bool,

    /// Show staged load progress on stderr.
    #[arg(long, default_value_t = false)]
    progress: bool,

    /// Print only one JSON line (metrics + load config) instead of the summary.
    #[arg(long, default_value_t = false)]
    json: bool,
}

/// One-line JSON of metrics + load config (GSV live ingest shape).
fn json_line(
    m: &llama_rs::InferenceMetrics,
    model: &str,
    use_mmap: bool,
    use_mlock: bool,
) -> String {
    format!(
        r#"{{"pp_tokens":{},"pp_tokens_per_sec":{:.3},"tg_tokens":{},"tg_tokens_per_sec":{:.3},"ttft_ms":{},"prompt_ms":{},"eval_ms":{},"wall_time_ms":{},"decode_count":{},"model":"{}","use_mmap":{},"use_mlock":{}}}"#,
        m.prompt_tokens,
        m.prompt_tokens_per_sec(),
        m.tokens_generated,
        m.tokens_per_sec(),
        m.ttft_ms.map_or(0, |v| v as i64),
        m.prompt_ms,
        m.eval_ms,
        m.wall_time_ms,
        m.decode_count,
        model.replace('\\', "\\\\").replace('"', "\\\""),
        use_mmap,
        use_mlock,
    )
}

/// Single-pass bench: load model, prefill prompt, generate `gen_tokens` tokens.
fn run(args: Args) -> i32 {
    let model_arg = match &args.model {
        Some(m) => m.clone(),
        None => match std::env::var_os("LLAMA_RS_BENCH_MODEL") {
            Some(v) => v.to_string_lossy().into_owned(),
            None => {
                eprintln!("error: no model; pass --model <path> or set LLAMA_RS_BENCH_MODEL");
                return 2;
            }
        },
    };
    let path = Path::new(&model_arg);
    if !path.exists() {
        eprintln!("error: model file not found: {}", path.display());
        return 2;
    }

    let backend = match Backend::init() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: backend init failed: {}", e);
            return 1;
        }
    };

    let use_mmap = if args.no_mmap { false } else { args.mmap };
    let staged = StagedLoadOptions::new()
        .with_mmap(use_mmap)
        .with_mlock(args.mlock);
    let model = if args.progress {
        let mut last_pct = 0u32;
        match Model::load_staged_with_progress(&backend, path, staged, &mut |p: f32| {
            let pct = (p * 100.0) as u32;
            if pct != last_pct && pct.is_multiple_of(5) {
                eprintln!("load {}% (mmap={}, mlock={})", pct, use_mmap, args.mlock);
                last_pct = pct;
            }
            true
        }) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: failed to load model (staged): {}", e);
                return 1;
            }
        }
    } else {
        match Model::load_staged(&backend, path, staged, None::<fn(f32) -> bool>) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: failed to load model (staged): {}", e);
                return 1;
            }
        }
    };

    let ctx_params = ContextParams::default()
        .with_n_ctx(std::num::NonZeroU32::new(args.n_ctx))
        .with_n_batch(args.n_batch);
    let ctx_params = match args.threads {
        Some(threads) if threads > 0 => ctx_params.with_n_threads(threads),
        _ => ctx_params,
    };
    let mut context = match model.new_context(&backend, ctx_params) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to create context: {}", e);
            return 1;
        }
    };

    let opts = GenerateOptions::builder()
        .max_tokens(args.gen_tokens)
        .temperature(0.0)
        .seed(0)
        .stop_at_eos(false)
        .build();

    match llama_rs::generate_with_metrics(&model, &mut context, &args.prompt, &opts) {
        Ok((_out, m)) => {
            if args.json {
                println!("{}", json_line(&m, &model_arg, use_mmap, args.mlock));
            } else {
                println!(
                    "model : {} (mmap={}, mlock={})",
                    model_arg, use_mmap, args.mlock
                );
                println!(
                    "pp    : {} tokens in {} ms -> {:.3} tok/s",
                    m.prompt_tokens,
                    m.prompt_ms,
                    m.prompt_tokens_per_sec()
                );
                println!(
                    "tg    : {} tokens in {} ms -> {:.3} tok/s",
                    m.tokens_generated,
                    m.eval_ms,
                    m.tokens_per_sec()
                );
                println!(
                    "ttft  : {} ms",
                    m.ttft_ms.map_or("n/a".to_string(), |v| v.to_string())
                );
                println!(
                    "wall  : {} ms (decode_count={})",
                    m.wall_time_ms, m.decode_count
                );
            }
            0
        }
        Err(e) => {
            eprintln!("error: generation failed: {}", e);
            1
        }
    }
}

fn main() {
    std::process::exit(run(Args::parse()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use llama_rs::InferenceMetrics;

    #[test]
    fn json_line_shape() {
        let m = InferenceMetrics {
            prompt_tokens: 128,
            tokens_generated: 32,
            decode_count: 161,
            wall_time_ms: 20_000,
            ttft_ms: Some(800),
            prompt_ms: 600,
            eval_ms: 19_400,
        };
        let line = json_line(&m, "models/Qwen\\\"x.gguf", true, false);
        assert!(line.starts_with('{') && line.ends_with('}'));
        assert!(line.contains(r#""tg_tokens":32,"#));
        assert!(line.contains(r#""tg_tokens_per_sec":"#));
        assert!(line.contains(r#""ttft_ms":800"#));
        assert!(line.contains(r#""model":"models/Qwen\\\"x.gguf""#));
        assert!(line.contains(r#""use_mmap":true"#));
        assert!(line.contains(r#""use_mlock":false"#));
    }

    #[test]
    fn json_line_name_checks() {
        let m = InferenceMetrics::default();
        let line = json_line(&m, "a\"b\\c", false, true);
        assert!(line.contains(r#"a\"b\\c"#), "path must be JSON-escaped");
        assert!(line.contains(r#""use_mmap":false"#));
        assert!(line.contains(r#""use_mlock":true"#));
    }
}
