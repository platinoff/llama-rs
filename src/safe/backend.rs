//! Backend initialization (must be called once before loading models).

use crate::error::{Error, Result};
use llama_cpp_2::llama_backend::LlamaBackend;

/// Proof that the llama backend has been initialized.
/// Required before loading any model or creating a context.
#[derive(Debug)]
pub struct Backend {
    inner: LlamaBackend,
}

impl Backend {
    /// Initialize the llama backend. Call once per process.
    /// Returns an error if already initialized.
    pub fn init() -> Result<Self> {
        LlamaBackend::init()
            .map(|inner| Backend { inner })
            .map_err(Error::from)
    }

    /// Access the inner backend for use with low-level APIs.
    #[inline]
    pub fn inner(&self) -> &LlamaBackend {
        &self.inner
    }
}
