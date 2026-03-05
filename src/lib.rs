//! Llama-RS — ultra-fast and safe Rust interface to llama.cpp.
//!
//! This library is **maximally Rust**: all public API and orchestration are written in safe Rust.
//! FFI is confined to the `llama-cpp-2` dependency. You get:
//!
//! - [Backend] — initialize once per process
//! - [Model] — load GGUF from path
//! - [Context] — decode, sample, generate
//! - [generate], [generate_stream] — pure Rust text generation with [GenerateOptions]
//! - [ModelParams], [ContextParams] — defaults for model/context config

mod error;
mod metrics;
mod safe;

pub use error::{Error, Result};
pub use metrics::InferenceMetrics;
pub use safe::{generate, generate_stream, Backend, Context, GenerateOptions, GenerateOptionsBuilder, Model};
#[cfg(feature = "metrics")]
pub use safe::generate_with_metrics;

/// Default params for loading a model. Re-export of [llama_cpp_2::model::params::LlamaModelParams].
pub type ModelParams = llama_cpp_2::model::params::LlamaModelParams;

/// Default params for creating a context. Re-export of [llama_cpp_2::context::params::LlamaContextParams].
pub type ContextParams = llama_cpp_2::context::params::LlamaContextParams;

/// Returns the greeting string for the first run.
#[must_use]
pub fn hello_llama_rust() -> &'static str {
    "Hello, Llama Rust!"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn hello_returns_non_empty() {
        let msg = hello_llama_rust();
        assert!(!msg.is_empty());
        assert!(msg.contains("Llama"));
        assert!(msg.contains("Rust"));
    }

    #[test]
    fn error_display_backend_already_initialized() {
        let e = Error::BackendAlreadyInitialized;
        let s = e.to_string();
        assert!(s.contains("backend"));
        assert!(s.contains("initialized"));
    }

    #[test]
    fn error_display_model_load() {
        let e = Error::ModelLoad {
            path: PathBuf::from("/x.gguf"),
            message: "file not found".into(),
        };
        let s = e.to_string();
        assert!(s.contains("load"));
        assert!(s.contains("file not found"));
    }

    #[test]
    fn error_display_decode() {
        let e = Error::Decode("batch full".into());
        assert!(e.to_string().contains("decode"));
        assert!(e.to_string().contains("batch full"));
    }

    #[test]
    fn generate_options_default_bounds() {
        let opts = GenerateOptions::default();
        assert!(opts.max_tokens > 0);
        assert!(opts.temperature >= 0.0);
        assert!(opts.top_k >= 0);
        assert!(opts.top_p > 0.0 && opts.top_p <= 1.0);
    }

    #[test]
    fn generate_options_clone() {
        let opts = GenerateOptions::default();
        let opts2 = opts.clone();
        assert_eq!(opts.max_tokens, opts2.max_tokens);
        assert_eq!(opts.temperature, opts2.temperature);
    }

    #[test]
    fn generate_options_builder() {
        let opts = GenerateOptions::builder()
            .max_tokens(64)
            .temperature(0.5)
            .top_k(10)
            .top_p(0.9)
            .seed(42)
            .stop_at_eos(false)
            .build();
        assert_eq!(opts.max_tokens, 64);
        assert!((opts.temperature - 0.5).abs() < 1e-6);
        assert_eq!(opts.top_k, 10);
        assert!((opts.top_p - 0.9).abs() < 1e-6);
        assert_eq!(opts.seed, Some(42));
        assert!(!opts.stop_at_eos);
    }

    #[test]
    fn generate_options_builder_defaults() {
        let opts = GenerateOptions::builder().build();
        assert!(opts.max_tokens > 0);
        assert!(opts.temperature >= 0.0);
        assert!(opts.top_p > 0.0 && opts.top_p <= 1.0);
    }
}
