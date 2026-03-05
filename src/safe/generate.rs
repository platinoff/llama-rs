//! Text generation: options and entry point (pure Rust loop).

use super::{Context, GenerateOptions, Model};
use crate::error::Result;

/// Generate text from a prompt. All orchestration is pure Rust.
pub fn generate(
    model: &Model,
    context: &mut Context<'_>,
    prompt: &str,
    opts: &GenerateOptions,
) -> Result<String> {
    super::context::generate_impl(&model.inner, context, prompt, opts)
}
