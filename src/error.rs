//! Unified error type for llama.rs (100% Rust API surface).

use std::path::PathBuf;

/// Errors that can occur when using llama.rs.
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
        match &e {
            LlamaCppError::BackendAlreadyInitialized => Error::BackendAlreadyInitialized,
            LlamaCppError::LlamaModelLoadError(inner) => {
                let path = match inner {
                    llama_cpp_2::LlamaModelLoadError::PathToStrError(p) => p.clone(),
                    _ => PathBuf::new(),
                };
                Error::ModelLoad {
                    path,
                    message: inner.to_string(),
                }
            }
            LlamaCppError::LlamaContextLoadError(_) => Error::ContextCreate(e.to_string()),
            LlamaCppError::DecodeError(_) => Error::Decode(e.to_string()),
            LlamaCppError::BatchAddError(_) => Error::Decode(e.to_string()),
            _ => Error::ContextCreate(e.to_string()),
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
        let path = match &e {
            llama_cpp_2::LlamaModelLoadError::PathToStrError(p) => p.clone(),
            _ => PathBuf::new(),
        };
        Error::ModelLoad {
            path,
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
