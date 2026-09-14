//! Minimal generation: `cargo run --example generate -- <model.gguf> [prompt]`.

use llama_rs::{generate, Backend, ContextParams, GenerateOptions, Model, ModelParams, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: cargo run --example generate -- <model.gguf> [prompt]");
        return Ok(());
    };
    let prompt = args.next().unwrap_or_else(|| "Hello!".to_string());

    let backend = Backend::init()?;
    let model = Model::load_from_file(&backend, &path, &ModelParams::default())?;
    let mut context = model.new_context(&backend, ContextParams::default())?;
    let opts = GenerateOptions::builder().max_tokens(32).build();
    let out = generate(&model, &mut context, &prompt, &opts)?;
    println!("{out}");
    Ok(())
}
