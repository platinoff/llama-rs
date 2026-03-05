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
    super::context::generate_impl(&model.inner, context, prompt, opts, None, None)
}

/// Generate text from a prompt, calling `on_chunk` with each decoded text piece as it is produced.
/// Returns the full generated string. Use this for streaming UIs or progress display.
pub fn generate_stream<F>(
    model: &Model,
    context: &mut Context<'_>,
    prompt: &str,
    opts: &GenerateOptions,
    mut on_chunk: F,
) -> Result<String>
where
    F: FnMut(&str),
{
    super::context::generate_impl(&model.inner, context, prompt, opts, Some(&mut on_chunk), None)
}

/// Generate text and collect metrics (tokens generated, decode count, wall time). Requires feature `metrics`.
#[cfg(feature = "metrics")]
pub fn generate_with_metrics(
    model: &Model,
    context: &mut Context<'_>,
    prompt: &str,
    opts: &GenerateOptions,
) -> Result<(String, crate::InferenceMetrics)> {
    let mut metrics = crate::InferenceMetrics::default();
    let s = super::context::generate_impl(
        &model.inner,
        context,
        prompt,
        opts,
        None,
        Some(&mut metrics),
    )?;
    Ok((s, metrics))
}
