//! Chat-template round trip: render messages with the model's baked-in template, then generate.
//! `cargo run --example chat -- <model.gguf> [user message]`.

use llama_rs::{
    generate, Backend, ChatMessage, ContextParams, GenerateOptions, Model, ModelParams, Result,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: cargo run --example chat -- <model.gguf> [user message]");
        return Ok(());
    };
    let user = args
        .next()
        .unwrap_or_else(|| "What is Rust in one sentence?".to_string());

    let backend = Backend::init()?;
    let model = Model::load_from_file(&backend, &path, &ModelParams::default())?;

    if let Some(template) = model.chat_template()? {
        println!("[template {} bytes]", template.len());
    }
    let messages = [
        ChatMessage::new("system", "You are terse.")?,
        ChatMessage::new("user", user)?,
    ];
    let prompt = model.apply_chat_template(&messages, true)?;

    let mut context = model.new_context(&backend, ContextParams::default())?;
    let opts = GenerateOptions::builder()
        .max_tokens(64)
        .temperature(0.0)
        .build();
    let out = generate(&model, &mut context, &prompt, &opts)?;
    println!("{out}");
    Ok(())
}
