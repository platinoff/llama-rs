//! Safe context wrapper: decode, sample, generate.

use crate::error::{Error, Result};
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;

/// Inference context tied to a [crate::Model]. All inference is done through the context.
pub struct Context<'a> {
    pub(crate) inner: LlamaContext<'a>,
}

impl<'a> Context<'a> {
    /// Decode a batch of tokens. Call [Self::candidates] or [Self::get_logits] after decode.
    pub fn decode(&mut self, batch: &mut LlamaBatch<'_>) -> Result<()> {
        self.inner.decode(batch).map_err(|e| Error::Decode(e.to_string()))
    }

    /// Encode a batch (e.g. for embeddings).
    pub fn encode(&mut self, batch: &mut LlamaBatch<'_>) -> Result<()> {
        self.inner.encode(batch).map_err(|e| Error::ContextCreate(e.to_string()))
    }

    /// Logits for the last decoded token (slice of size n_vocab).
    #[inline]
    pub fn get_logits(&self) -> &[f32] {
        self.inner.get_logits()
    }

    /// Iterator over token candidates (logits) for the last token.
    #[inline]
    pub fn candidates(&self) -> impl Iterator<Item = llama_cpp_2::token::data::LlamaTokenData> + '_ {
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
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.7,
            top_k: 40,
            top_p: 0.95,
            seed: None,
            stop_at_eos: true,
        }
    }
}

/// Generate text from a prompt using the given context and model. Pure Rust orchestration.
pub(crate) fn generate_impl(
    model: &llama_cpp_2::model::LlamaModel,
    context: &mut Context<'_>,
    prompt: &str,
    opts: &GenerateOptions,
) -> Result<String> {
    let tokens = model
        .str_to_token(prompt, llama_cpp_2::model::AddBos::Always)
        .map_err(|e| Error::Tokenize(e.to_string()))?;

    if tokens.is_empty() {
        return Ok(String::new());
    }

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

    let mut output_tokens: Vec<LlamaToken> = Vec::with_capacity(opts.max_tokens as usize);
    let mut n_cur = tokens.len() as i32;
    let mut n_gen = 0u32;

    while n_gen < opts.max_tokens && n_cur < n_ctx {
        let mut arr = context.inner.token_data_array();
        sampler.apply(&mut arr);
        let token = arr.selected_token().unwrap_or(eos_token);
        sampler.accept(token);

        if opts.stop_at_eos && model.is_eog_token(token) {
            break;
        }

        output_tokens.push(token);
        n_gen += 1;

        batch.clear();
        batch
            .add(token, n_cur, &[seq_id], true)
            .map_err(|e| Error::Decode(e.to_string()))?;
        n_cur += 1;
        context.decode(&mut batch)?;
    }

    let mut s = String::new();
    #[allow(deprecated)]
    let special = llama_cpp_2::model::Special::Plaintext;
    for t in &output_tokens {
        if let Ok(piece) = model.token_to_str(*t, special) {
            s.push_str(&piece);
        }
    }
    Ok(s)
}
