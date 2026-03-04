//! Llama-RS — ultra-fast and safe Rust interface to llama.cpp.
//!
//! This library provides a Rust-first API for loading and running
//! GGUF models via the llama.cpp master build.

/// Returns the greeting string for the first run.
#[must_use]
pub fn hello_llama_rust() -> &'static str {
    "Hello, Llama Rust!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_non_empty() {
        let msg = hello_llama_rust();
        assert!(!msg.is_empty());
        assert!(msg.contains("Llama"));
        assert!(msg.contains("Rust"));
    }
}
