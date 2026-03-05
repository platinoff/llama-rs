//! Safe model wrapper: load GGUF from path.

use crate::error::Result;
use crate::safe::Backend;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use std::path::Path;

/// A loaded GGUF model. Use with [crate::Context] for inference.
#[derive(Debug)]
pub struct Model {
    pub(crate) inner: LlamaModel,
}

impl Model {
    /// Load a model from a GGUF file (or split files following the naming pattern).
    pub fn load_from_file(
        backend: &Backend,
        path: impl AsRef<Path>,
        params: &LlamaModelParams,
    ) -> Result<Self> {
        let inner = LlamaModel::load_from_file(backend.inner(), path, params)?;
        Ok(Model { inner })
    }

    /// Create a new context from this model with the given params.
    pub fn new_context(
        &self,
        backend: &Backend,
        params: LlamaContextParams,
    ) -> Result<super::Context<'_>> {
        let ctx = self.inner.new_context(backend.inner(), params)?;
        Ok(super::Context { inner: ctx })
    }

    /// Number of layers in the model.
    #[inline]
    pub fn n_layer(&self) -> u32 {
        self.inner.n_layer()
    }

    /// Context size the model was trained on.
    #[inline]
    pub fn n_ctx_train(&self) -> u32 {
        self.inner.n_ctx_train()
    }

    /// Vocabulary size.
    #[inline]
    pub fn n_vocab(&self) -> i32 {
        self.inner.n_vocab()
    }
}
