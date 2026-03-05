//! CLI entry point for Llama-RS (64-bit release binary).
//!
//! Usage:
//!   llama_rs                    — print greeting
//!   llama_rs <model.gguf> [prompt] — load model and generate (prompt optional)

use llama_rs::{hello_llama_rust, Backend, GenerateOptions, Model};
use std::path::Path;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::model::params::LlamaModelParams;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("{}", hello_llama_rust());
        return;
    }

    let path = Path::new(&args[1]);
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

    let model_params = LlamaModelParams::default();
    let model = match Model::load_from_file(&backend, path, &model_params) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: failed to load model: {}", e);
            std::process::exit(1);
        }
    };

    let ctx_params = LlamaContextParams::default();
    let mut context = match model.new_context(&backend, ctx_params) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to create context: {}", e);
            std::process::exit(1);
        }
    };

    let prompt = args.get(2).map(String::as_str).unwrap_or("Hello");
    let opts = GenerateOptions::default();
    match llama_rs::generate(&model, &mut context, prompt, &opts) {
        Ok(out) => print!("{}", out),
        Err(e) => {
            eprintln!("error: generation failed: {}", e);
            std::process::exit(1);
        }
    }
}
