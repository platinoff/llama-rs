//! Safe context wrapper: decode, sample, generate.

use crate::error::{Error, Result};
use crate::InferenceMetrics;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::sampling::LlamaSampler;
use std::time::Instant;

/// Inference context tied to a [crate::Model]. All inference is done through the context.
pub struct Context<'a> {
    pub(crate) inner: LlamaContext<'a>,
}

impl<'a> Context<'a> {
    /// Decode a batch of tokens. Call [Self::candidates] or [Self::get_logits] after decode.
    pub fn decode(&mut self, batch: &mut LlamaBatch<'_>) -> Result<()> {
        self.inner
            .decode(batch)
            .map_err(|e| Error::Decode(e.to_string()))
    }

    /// Encode a batch (e.g. for embeddings).
    pub fn encode(&mut self, batch: &mut LlamaBatch<'_>) -> Result<()> {
        self.inner
            .encode(batch)
            .map_err(|e| Error::ContextCreate(e.to_string()))
    }

    /// Logits for the last decoded token (slice of size n_vocab).
    #[inline]
    pub fn get_logits(&self) -> &[f32] {
        self.inner.get_logits()
    }

    /// Iterator over token candidates (logits) for the last token.
    #[inline]
    pub fn candidates(
        &self,
    ) -> impl Iterator<Item = llama_cpp_2::token::data::LlamaTokenData> + '_ {
        self.inner.candidates()
    }

    /// Context size (max tokens).
    #[inline]
    pub fn n_ctx(&self) -> u32 {
        self.inner.n_ctx()
    }

    /// Max batch size for decode.
    #[inline]
    pub fn n_batch(&self) -> u32 {
        self.inner.n_batch()
    }

    /// Reset the context so a fresh sequence can start.
    ///
    /// Clears the KV cache / recurrent memory. Required between independent
    /// generations on the same context: hybrid (Gated Delta Net / M-RoPE)
    /// models require strictly increasing sequence positions, so re-attempting
    /// a prompt at position 0 without a reset fails the decode.
    #[inline]
    pub fn reset(&mut self) {
        self.inner.clear_kv_cache();
    }
}

/// Options for text generation (all Rust types).
#[derive(Clone, Debug)]
pub struct GenerateOptions {
    /// Max new tokens to generate.
    pub max_tokens: u32,
    /// Temperature (0 = greedy, >0 for sampling).
    pub temperature: f32,
    /// Top-k sampling (0 = disabled).
    pub top_k: i32,
    /// Top-p (nucleus) sampling (1.0 = disabled).
    pub top_p: f32,
    /// Random seed (None = default).
    pub seed: Option<u32>,
    /// Stop at end-of-sequence token.
    pub stop_at_eos: bool,
    /// Stop generation when the output ends with any of these strings (matched after each token).
    pub stop_sequences: Vec<String>,
}

/// Builder for [GenerateOptions]. Use [GenerateOptions::builder].
#[derive(Clone, Debug, Default)]
pub struct GenerateOptionsBuilder {
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_k: Option<i32>,
    top_p: Option<f32>,
    seed: Option<u32>,
    stop_at_eos: Option<bool>,
    stop_sequences: Option<Vec<String>>,
}

impl GenerateOptionsBuilder {
    /// Set max new tokens to generate.
    #[must_use]
    pub fn max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }

    /// Set temperature (0 = greedy, >0 for sampling).
    #[must_use]
    pub fn temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }

    /// Set top-k sampling (0 = disabled).
    #[must_use]
    pub fn top_k(mut self, k: i32) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Set top-p (nucleus) sampling (1.0 = disabled).
    #[must_use]
    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set random seed.
    #[must_use]
    pub fn seed(mut self, s: u32) -> Self {
        self.seed = Some(s);
        self
    }

    /// Set whether to stop at end-of-sequence token.
    #[must_use]
    pub fn stop_at_eos(mut self, stop: bool) -> Self {
        self.stop_at_eos = Some(stop);
        self
    }

    /// Set stop sequences: generation stops when output ends with any of these (matched after each token).
    #[must_use]
    pub fn stop_sequences(mut self, seq: Vec<String>) -> Self {
        self.stop_sequences = Some(seq);
        self
    }

    /// Add a single stop sequence.
    #[must_use]
    pub fn stop_sequence(mut self, s: impl Into<String>) -> Self {
        let mut v = self.stop_sequences.unwrap_or_default();
        v.push(s.into());
        self.stop_sequences = Some(v);
        self
    }

    /// Build [GenerateOptions] with defaults for any unset fields.
    #[must_use]
    pub fn build(self) -> GenerateOptions {
        GenerateOptions {
            max_tokens: self.max_tokens.unwrap_or(256),
            temperature: self.temperature.unwrap_or(0.7),
            top_k: self.top_k.unwrap_or(40),
            top_p: self.top_p.unwrap_or(0.95),
            seed: self.seed,
            stop_at_eos: self.stop_at_eos.unwrap_or(true),
            stop_sequences: self.stop_sequences.unwrap_or_default(),
        }
    }
}

impl GenerateOptions {
    /// Create a builder with all options unset (defaults used at [GenerateOptionsBuilder::build]).
    #[must_use]
    pub fn builder() -> GenerateOptionsBuilder {
        GenerateOptionsBuilder::default()
    }
}

impl Default for GenerateOptions {
    fn default() -> Self {
        GenerateOptionsBuilder::default().build()
    }
}

/// Presets for `ContextParams` (pure Rust, see `docs/SIZING.md`).
///
/// `low_memory()` → `n_ctx=2048, n_batch=512` (small KV cache, fits 16 GiB + 27B mmap).
/// `max_speed()` → `n_ctx=4096, n_batch=2048` (larger batch for faster prefill).
pub mod presets {
    use crate::ContextParams;
    use std::num::NonZeroU32;

    /// Low-memory preset: `n_ctx=2048`, `n_batch=512`.
    #[must_use]
    pub fn low_memory() -> ContextParams {
        ContextParams::default()
            .with_n_ctx(NonZeroU32::new(2048))
            .with_n_batch(512)
    }

    /// Max-speed preset: `n_ctx=4096`, `n_batch=2048` (prefill in fewer steps).
    #[must_use]
    pub fn max_speed() -> ContextParams {
        ContextParams::default()
            .with_n_ctx(NonZeroU32::new(4096))
            .with_n_batch(2048)
    }
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod preset_tests {
    use super::presets;
    use std::num::NonZeroU32;

    #[test]
    fn low_memory_preset() {
        let p = presets::low_memory();
        assert_eq!(p.n_ctx(), NonZeroU32::new(2048));
        assert_eq!(p.n_batch(), 512);
    }

    #[test]
    fn max_speed_preset() {
        let p = presets::max_speed();
        assert_eq!(p.n_ctx(), NonZeroU32::new(4096));
        assert_eq!(p.n_batch(), 2048);
    }
}

/// Generate text from a prompt using the given context and model. Pure Rust orchestration.
/// If `on_chunk` is `Some`, it is called with each decoded text piece (streaming).
/// If `metrics` is `Some`, it is filled with pp/tg/TTFT (llama-bench phases).
pub(crate) fn generate_impl(
    model: &llama_cpp_2::model::LlamaModel,
    context: &mut Context<'_>,
    prompt: &str,
    opts: &GenerateOptions,
    mut on_chunk: Option<&mut dyn FnMut(&str)>,
    mut metrics: Option<&mut InferenceMetrics>,
) -> Result<String> {
    let start = Instant::now();
    let tokens = model
        .str_to_token(prompt, llama_cpp_2::model::AddBos::Always)
        .map_err(|e| Error::Tokenize(e.to_string()))?;

    if tokens.is_empty() {
        return Ok(String::new());
    }

    let prompt_tokens = tokens.len() as u32;
    let n_ctx = context.n_ctx() as i32;
    let n_batch = context.n_batch() as usize;
    let eos_token = model.token_eos();
    let seq_id: i32 = 0;

    let mut batch = LlamaBatch::new(n_batch, 1);
    batch.clear();
    batch
        .add_sequence(&tokens, seq_id, false)
        .map_err(|e| Error::Decode(e.to_string()))?;
    context.decode(&mut batch)?;
    let prompt_ms = start.elapsed().as_millis() as u64;
    if let Some(m) = metrics.as_mut() {
        m.decode_count += 1;
        m.prompt_tokens = prompt_tokens;
        m.prompt_ms = prompt_ms;
    }
    let mut ttft_ms: Option<u64> = None;

    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(if opts.temperature <= 0.0 {
            1e-6
        } else {
            opts.temperature
        }),
        LlamaSampler::top_k(opts.top_k),
        LlamaSampler::top_p(opts.top_p, 1),
        LlamaSampler::dist(opts.seed.unwrap_or(0xFFFF_FFFF)),
    ]);

    let mut running_output = String::new();
    running_output.reserve((opts.max_tokens as usize).saturating_mul(4)); // ~4 chars per token heuristic
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut n_cur = tokens.len() as i32;
    let mut n_gen = 0u32;
    let stop_sequences_empty = opts.stop_sequences.is_empty();

    while n_gen < opts.max_tokens && n_cur < n_ctx {
        let mut arr = context.inner.token_data_array();
        sampler.apply(&mut arr);
        let token = arr.selected_token().unwrap_or(eos_token);
        sampler.accept(token);

        if opts.stop_at_eos && model.is_eog_token(token) {
            break;
        }

        n_gen += 1;

        let piece = match model.token_to_piece(token, &mut decoder, false, None) {
            Ok(p) => p,
            Err(e) => return Err(Error::TokenToString(e.to_string())),
        };
        running_output.push_str(&piece);
        if ttft_ms.is_none() {
            ttft_ms = Some(start.elapsed().as_millis() as u64);
        }
        if let Some(m) = metrics.as_mut() {
            if m.ttft_ms.is_none() {
                m.ttft_ms = ttft_ms;
            }
        }
        if let Some(f) = on_chunk.as_mut() {
            f(&piece);
        }

        let mut stop_hit = false;
        if !stop_sequences_empty {
            for stop in &opts.stop_sequences {
                if !stop.is_empty() && running_output.ends_with(stop) {
                    let trim = running_output.len().saturating_sub(stop.len());
                    running_output.truncate(trim);
                    stop_hit = true;
                    break;
                }
            }
        }
        if stop_hit {
            break;
        }

        batch.clear();
        batch
            .add(token, n_cur, &[seq_id], true)
            .map_err(|e| Error::Decode(e.to_string()))?;
        n_cur += 1;
        context.decode(&mut batch)?;
        if let Some(m) = metrics.as_mut() {
            m.decode_count += 1;
        }
    }

    if let Some(m) = metrics.as_mut() {
        m.tokens_generated = n_gen;
        m.wall_time_ms = start.elapsed().as_millis() as u64;
        m.ttft_ms = ttft_ms.or(m.ttft_ms);
        m.eval_ms = m.wall_time_ms.saturating_sub(m.prompt_ms);
    }
    Ok(running_output)
}
