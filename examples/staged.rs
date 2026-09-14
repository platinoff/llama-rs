//! Staged (mmap) load with progress: `cargo run --example staged -- <model.gguf>`.

use llama_rs::{Backend, Model, Result, StagedLoadOptions};

fn main() -> Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example staged -- <model.gguf>");
        return Ok(());
    };

    let backend = Backend::init()?;
    let model = Model::load_staged(
        &backend,
        &path,
        StagedLoadOptions::mmap(),
        Some(|p: f32| {
            println!("{:>5.1}%", p * 100.0);
            true
        }),
    )?;
    println!(
        "loaded: {} layers, ctx_train {}",
        model.n_layer(),
        model.n_ctx_train()
    );
    Ok(())
}
