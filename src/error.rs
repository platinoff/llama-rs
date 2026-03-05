//! Unified error type for Llama-RS (100% Rust API surface).

use std::path::PathBuf;

/// Errors that can occur when using Llama-RS.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("backend already initialized")]
    BackendAlreadyInitialized,

    #[error("failed to load model from {path}: {message}")]
    ModelLoad { path: PathBuf, message: String },

    #[error("failed to create context: {0}")]
    ContextCreate(String),

    #[error("failed to tokenize: {0}")]
    Tokenize(String),

    #[error("decode failed: {0}")]
    Decode(String),

    #[error("sampler error: {0}")]
    Sampler(String),

    #[error("token to string: {0}")]
    TokenToString(String),
}

impl From<llama_cpp_2::LlamaCppError> for Error {
    fn from(e: llama_cpp_2::LlamaCppError) -> Self {
        use llama_cpp_2::LlamaCppError;
        let msg = e.to_string();
        match &e {
            LlamaCppError::BackendAlreadyInitialized => Error::BackendAlreadyInitialized,
            LlamaCppError::LlamaModelLoadError(_) => Error::ModelLoad {
                path: PathBuf::new(),
                message: msg,
            },
            LlamaCppError::LlamaContextLoadError(_) => Error::ContextCreate(msg),
            LlamaCppError::DecodeError(_) => Error::Decode(msg),
            LlamaCppError::BatchAddError(_) => Error::Decode(msg),
            _ => Error::ContextCreate(msg),
        }
    }
}

impl From<llama_cpp_2::DecodeError> for Error {
    fn from(e: llama_cpp_2::DecodeError) -> Self {
        Error::Decode(e.to_string())
    }
}

impl From<llama_cpp_2::EncodeError> for Error {
    fn from(e: llama_cpp_2::EncodeError) -> Self {
        Error::ContextCreate(e.to_string())
    }
}

impl From<llama_cpp_2::llama_batch::BatchAddError> for Error {
    fn from(e: llama_cpp_2::llama_batch::BatchAddError) -> Self {
        Error::Decode(e.to_string())
    }
}

impl From<llama_cpp_2::LlamaModelLoadError> for Error {
    fn from(e: llama_cpp_2::LlamaModelLoadError) -> Self {
        Error::ModelLoad {
            path: PathBuf::new(),
            message: e.to_string(),
        }
    }
}

impl From<llama_cpp_2::LlamaContextLoadError> for Error {
    fn from(e: llama_cpp_2::LlamaContextLoadError) -> Self {
        Error::ContextCreate(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
