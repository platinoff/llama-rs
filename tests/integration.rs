//! Integration tests for Llama-RS.

use llama_rs::hello_llama_rust;

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
