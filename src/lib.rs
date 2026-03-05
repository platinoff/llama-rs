//! Llama-RS — ultra-fast and safe Rust interface to llama.cpp.
//!
//! This library is **maximally Rust**: all public API and orchestration are written in safe Rust.
//! FFI is confined to the `llama-cpp-2` dependency. You get:
//!
//! - [Backend] — initialize once per process
//! - [Model] — load GGUF from path
//! - [Context] — decode, sample, generate
//! - [generate] — pure Rust text generation loop with [GenerateOptions]

mod error;
mod safe;

pub use error::{Error, Result};
pub use safe::{generate, Backend, Context, GenerateOptions, Model};

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
