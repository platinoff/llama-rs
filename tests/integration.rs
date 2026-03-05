//! Integration tests for Llama-RS.

use llama_rs::{hello_llama_rust, GenerateOptions};

#[test]
fn hello_llama_rust_integration() {
    let msg = hello_llama_rust();
    assert_eq!(msg, "Hello, Llama Rust!");
}

#[test]
fn greeting_is_ascii_printable() {
    let msg = hello_llama_rust();
    assert!(msg.is_ascii());
}

#[test]
fn generate_options_default() {
    let opts = GenerateOptions::default();
    assert!(opts.max_tokens > 0);
    assert!(opts.temperature >= 0.0);
    assert!(opts.top_p > 0.0 && opts.top_p <= 1.0);
}

/// Loads a real model and runs a short generation when `LLAMA_RS_TEST_MODEL` is set.
/// Skip when env is unset so CI and normal `cargo test` do not require a GGUF file.
#[test]
fn generate_with_model_if_env_set() {
    let path = match std::env::var("LLAMA_RS_TEST_MODEL") {
        Ok(p) if !p.is_empty() => std::path::PathBuf::from(p),
        _ => return,
    };
    if !path.exists() {
        eprintln!("LLAMA_RS_TEST_MODEL path does not exist: {}", path.display());
        return;
    }

    let backend = llama_rs::Backend::init().expect("backend init");
    let params = llama_cpp_2::model::params::LlamaModelParams::default();
    let model = llama_rs::Model::load_from_file(&backend, &path, &params).expect("load model");
    let ctx_params = llama_cpp_2::context::params::LlamaContextParams::default();
    let mut context = model.new_context(&backend, ctx_params).expect("new context");
    let mut opts = GenerateOptions::default();
    opts.max_tokens = 4;
    opts.stop_at_eos = true;

    let out = llama_rs::generate(&model, &mut context, "Hi", &opts).expect("generate");
    assert!(out.len() <= 256, "short run should not produce huge output");
}
