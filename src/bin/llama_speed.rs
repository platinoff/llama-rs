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
use llama_rs::{preflight, Backend, ContextParams, GenerateOptions, Model, StagedLoadOptions};
#[cfg(feature = "metrics")]
use llama_rs::{MtpParams, MtpSession};
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

    /// Skip the free-RAM vs model-size preflight warnings.
    #[arg(long, default_value_t = false)]
    skip_preflight: bool,

    /// Print only one JSON line (metrics + load config) instead of the summary.
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Path to a draft GGUF for MTP speculative decoding (requires --model too).
    #[arg(long)]
    draft: Option<String>,
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
    if !args.skip_preflight {
        for w in preflight::warnings(
            preflight::free_ram_mib(),
            preflight::model_file_mib(path),
            &staged,
        ) {
            eprintln!("{w}");
        }
    }
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

    let opts = GenerateOptions::builder()
        .max_tokens(args.gen_tokens)
        .temperature(0.0)
        .seed(0)
        .stop_at_eos(false)
        .build();

    if let Some(draft_path_str) = &args.draft {
        // MTP speculative path: MtpSession creates both target and draft contexts.
        let draft_path = Path::new(draft_path_str);
        if !draft_path.exists() {
            eprintln!("error: draft model not found: {}", draft_path.display());
            return 2;
        }
        let draft_model = match Model::load_staged(
            &backend,
            draft_path,
            StagedLoadOptions::new()
                .with_mmap(use_mmap)
                .with_mlock(args.mlock),
            None::<fn(f32) -> bool>,
        ) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: failed to load draft model: {}", e);
                return 1;
            }
        };
        let draft_ctx_params = ContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(args.n_ctx))
            .with_n_batch(args.n_batch);
        let draft_ctx_params = match args.threads {
            Some(threads) if threads > 0 => draft_ctx_params.with_n_threads(threads),
            _ => draft_ctx_params,
        };
        let mtp_params = MtpParams::default();
        let mut session = match MtpSession::new(
            &backend,
            &model,
            &draft_model,
            ctx_params,
            draft_ctx_params,
            mtp_params,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to create MTP session: {}", e);
                return 1;
            }
        };
        let mut metrics_out = llama_rs::InferenceMetrics::default();
        match session.generate(
            &model,
            &args.prompt,
            &opts,
            Some(&mut |chunk: &str| {
                if !args.json {
                    eprint!("{chunk}");
                }
            }),
            Some(&mut metrics_out),
        ) {
            Ok(_out) => {
                if args.json {
                    println!(
                        "{}",
                        json_line(&metrics_out, &model_arg, use_mmap, args.mlock)
                    );
                } else {
                    eprintln!();
                    println!(
                        "model : {} (mmap={}, mlock={}) [MTP draft={}]",
                        model_arg, use_mmap, args.mlock, draft_path_str
                    );
                    println!(
                        "pp    : {} tokens in {} ms -> {:.3} tok/s",
                        metrics_out.prompt_tokens,
                        metrics_out.prompt_ms,
                        metrics_out.prompt_tokens_per_sec()
                    );
                    println!(
                        "tg    : {} tokens in {} ms -> {:.3} tok/s",
                        metrics_out.tokens_generated,
                        metrics_out.eval_ms,
                        metrics_out.tokens_per_sec()
                    );
                    println!(
                        "ttft  : {} ms",
                        metrics_out
                            .ttft_ms
                            .map_or("n/a".to_string(), |v| v.to_string())
                    );
                    println!(
                        "wall  : {} ms (decode_count={})",
                        metrics_out.wall_time_ms, metrics_out.decode_count
                    );
                }
                0
            }
            Err(e) => {
                eprintln!("error: MTP generation failed: {}", e);
                1
            }
        }
    } else {
        // Standard (non-speculative) path.
        let mut context = match model.new_context(&backend, ctx_params) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to create context: {}", e);
                return 1;
            }
        };
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
