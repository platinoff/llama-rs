//! CLI entry point for llama.rs (64-bit release binary).
//!
//! Usage:
//!   llama_rs                      — print greeting
//!   llama_rs <model.gguf> [prompt] — load model and generate
//!   llama_rs --max-tokens 64 --temperature 0.5 model.gguf "Hello"
//!   llama_rs --system "You are helpful." model.gguf "Explain this"
//!   llama_rs --help               — show all options

use clap::Parser;
use llama_rs::{Backend, ContextParams, GenerateOptions, Model, ModelParams};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "llama_rs")]
#[command(about = "llama.rs — ultra-fast Rust wrapper around llama.cpp")]
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

    let model_params = ModelParams::default();
    let model = match Model::load_from_file(&backend, path, &model_params) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: failed to load model: {}", e);
            std::process::exit(1);
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
