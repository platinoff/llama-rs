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
