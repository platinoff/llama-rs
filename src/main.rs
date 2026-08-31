//! CLI entry point for llama.rs (64-bit release binary).
//!
//! Usage:
//!   llama_rs                      — print greeting
//!   llama_rs <model.gguf> [prompt] — load model and generate
//!   llama_rs --max-tokens 64 --temperature 0.5 model.gguf "Hello"
//!   llama_rs --system "You are helpful." model.gguf "Explain this"
//!   llama_rs --help               — show all options

use clap::Parser;
use llama_rs::{Backend, ContextParams, GenerateOptions, Model, StagedLoadOptions};
use std::io::Write as _;
use std::path::Path;
use std::time::Duration;

/// Best-effort push of staged progress to GSV live (`GSV_LIVE=1` → `127.0.0.1:9999`).
/// Pure Rust std only, no extra deps, 150ms timeout, ignore errors.
fn gsv_report_progress(p: f32) {
    if std::env::var_os("GSV_LIVE").is_none() {
        return;
    }
    let body = format!(
        r#"{{"staged_progress":{:.3},"ts":{}}}"#,
        p,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let req = format!(
        "POST /api/ingest HTTP/1.1\r\nHost: 127.0.0.1:9999\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    if let Ok(mut s) = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], 9999)),
        Duration::from_millis(120),
    ) {
        let _ = s.set_write_timeout(Some(Duration::from_millis(80)));
        let _ = s.write_all(req.as_bytes());
    }
}

#[derive(Parser, Debug)]
#[command(name = "llama_rs")]
#[command(about = "llama.rs — Llama in Rust (backend: llama.cpp, staged disk→RAM)")]
struct Args {
    /// Path to the GGUF model file.
    #[arg(index = 1)]
    model: Option<String>,

    /// Prompt to complete (default: "Hello"). Ignored if --system is used without prompt.
    #[arg(index = 2)]
    prompt: Option<String>,

    /// Maximum new tokens to generate.
    #[arg(long, default_value_t = 256)]
    max_tokens: u32,

    /// Sampling temperature (0 = greedy, >0 for sampling).
    #[arg(long, default_value_t = 0.7)]
    temperature: f32,

    /// Random seed (omit for non-deterministic).
    #[arg(long)]
    seed: Option<u32>,

    /// Do not stop at end-of-sequence token.
    #[arg(long)]
    no_eos: bool,

    /// System or prefix prompt (prepended to the main prompt with a newline).
    #[arg(long)]
    system: Option<String>,

    /// Use mmap for model load (default true; low-RAM, paged). Use --no-mmap to fully resident.
    #[arg(long, default_value_t = true)]
    mmap: bool,

    /// Disable mmap (fully read into RAM, needs ~8 GiB for 27B).
    #[arg(long, default_value_t = false)]
    no_mmap: bool,

    /// Pin model pages with mlock (needs privilege + RAM, default false).
    #[arg(long, default_value_t = false)]
    mlock: bool,

    /// Show staged load progress (0..100%).
    #[arg(long, default_value_t = false)]
    progress: bool,
}

fn main() {
    let args = Args::parse();

    if args.model.is_none() {
        println!("{}", llama_rs::hello_llama_rust());
        return;
    }

    let path = Path::new(args.model.as_ref().unwrap());
    if !path.exists() {
        eprintln!("error: model file not found: {}", path.display());
        std::process::exit(1);
    }

    let backend = match Backend::init() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: backend init failed: {}", e);
            std::process::exit(1);
        }
    };

    let use_mmap = if args.no_mmap { false } else { args.mmap };
    let staged = StagedLoadOptions::new()
        .with_mmap(use_mmap)
        .with_mlock(args.mlock);
    // Staged loading with optional progress (pure Rust, controls disk→RAM).
    let model = if args.progress {
        let mut last_pct = 0u32;
        match Model::load_staged_with_progress(&backend, path, staged, &mut |p: f32| {
            let pct = (p * 100.0) as u32;
            gsv_report_progress(p);
            if pct != last_pct && pct.is_multiple_of(5) {
                eprintln!("loading {}% (mmap={}, mlock={})", pct, use_mmap, args.mlock);
                last_pct = pct;
            }
            true
        }) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: failed to load model (staged): {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match Model::load_staged(&backend, path, staged, None::<fn(f32) -> bool>) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: failed to load model (staged): {}", e);
                std::process::exit(1);
            }
        }
    };

    let ctx_params = ContextParams::default();
    let mut context = match model.new_context(&backend, ctx_params) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to create context: {}", e);
            std::process::exit(1);
        }
    };

    let prompt = match (&args.system, &args.prompt) {
        (Some(s), Some(p)) => format!("{}\n{}", s, p),
        (Some(s), None) => s.clone(),
        (None, Some(p)) => p.clone(),
        (None, None) => "Hello".to_string(),
    };

    let mut opts_builder = GenerateOptions::builder()
        .max_tokens(args.max_tokens)
        .temperature(args.temperature)
        .stop_at_eos(!args.no_eos);
    if let Some(s) = args.seed {
        opts_builder = opts_builder.seed(s);
    }
    let opts = opts_builder.build();

    match llama_rs::generate(&model, &mut context, &prompt, &opts) {
        Ok(out) => print!("{}", out),
        Err(e) => {
            eprintln!("error: generation failed: {}", e);
            std::process::exit(1);
        }
    }
}
