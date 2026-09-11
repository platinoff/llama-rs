//! MTP speculative decoding: same-model draft head speeds up dense targets.
//!
//! Backend: `llama-cpp-2`'s `MtpSpeculative` (wrapper over llama.cpp `draft-mtp`).
//! The draft is an MTP-head GGUF (e.g. `mtp-Qwen3.8-27B-Q8_0.gguf`); the caller
//! still uses the main model for sampling — the draft only proposes tokens.
//!
//! Protocol (llama.cpp `common/speculative.cpp` + `tools/server/server-context.cpp`):
//! 1. `begin(prompt)` resets the speculative state.
//! 2. Prefill: decode the prompt in the target context, calling `process(batch)`
//!    after EVERY decode (including multi-batch prefills).
//! 3. Loop:
//!    - `draft(n_past, id_last, prompt)` returns candidate tokens.
//!    - Decode all candidates as one batch in the target context.
//!    - Greedy-compare each candidate position against the target logits;
//!      count consecutive hits (`n_accepted`).
//!    - `accept(n_accepted)` tells the drafter how many were accepted.
//!    - Emit accepted drafts + (if there was a first mismatch) the corrected
//!      target token; advance `n_past` by what was actually emitted.
//!
//! All code is safe Rust; FFI stays inside `llama-cpp-2`.

use crate::error::{Error, Result};
use crate::safe::Model;
use crate::{ContextParams, InferenceMetrics};
use llama_cpp_2::context::params::LlamaContextType;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::AddBos;
use llama_cpp_2::speculative::{MtpSpeculative, MtpSpeculativeParams};
use llama_cpp_2::token::LlamaToken;
use std::time::Instant;

/// Parameters for MTP speculative decoding.
///
/// Defaults mirror llama.cpp: draft up to `n_max` tokens per round, no minimum,
/// accept any confidence (`p_min = 0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MtpParams {
    /// Maximum number of draft tokens to propose per round.
    pub n_max: i32,
    /// Minimum number of draft tokens required before returning a draft.
    pub n_min: i32,
    /// Minimum draft-token probability accepted by the drafter (`0.0` = any).
    pub p_min: f32,
}

impl Default for MtpParams {
    fn default() -> Self {
        Self {
            n_max: 3,
            n_min: 0,
            p_min: 0.0,
        }
    }
}

impl MtpParams {
    /// New with defaults (`n_max = 3`, `n_min = 0`, `p_min = 0.0`).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum draft length per round.
    #[must_use]
    pub fn with_n_max(mut self, n: i32) -> Self {
        self.n_max = n;
        self
    }

    /// Set the minimum draft length per round.
    #[must_use]
    pub fn with_n_min(mut self, n: i32) -> Self {
        self.n_min = n;
        self
    }

    /// Set the minimum accepted draft probability.
    #[must_use]
    pub fn with_p_min(mut self, p: f32) -> Self {
        self.p_min = p;
        self
    }
}

/// Owner of an MTP speculative session: both contexts (target + draft models)
/// and the llama.cpp draft state.
///
/// Create with [Self::new], then run one [Self::generate] per prompt.
pub struct MtpSession<'a> {
    mtp: MtpSpeculative<'a>,
    eos: LlamaToken,
    n_ctx: i32,
    n_batch: usize,
}

impl<'a> MtpSession<'a> {
    /// Create an MTP session from a target model and an MTP draft model.
    ///
    /// Both models must be loaded from the same backend. The draft context
    /// should use `n_ctx` large enough for the prompt (its KV is separate).
    ///
    /// # Errors
    ///
    /// Returns an error when a context cannot be created or llama.cpp rejects
    /// the speculative initialization (e.g. incompatible draft head).
    pub fn new(
        backend: &crate::safe::Backend,
        target: &'a Model,
        draft: &'a Model,
        target_params: ContextParams,
        draft_params: ContextParams,
        spd: MtpParams,
    ) -> Result<Self> {
        let target_ctx = target
            .inner
            .new_context(
                backend.inner(),
                // Hybrid targets (e.g. Qwen3.5) keep a recurrent-state cache alongside
                // the KV cache; partial rollback after MTP accept requires per-seq
                // snapshots. The reference server sizes this with the draft length.
                target_params.with_n_rs_seq(spd.n_max.max(0) as u32),
            )
            .map_err(|e| Error::ContextCreate(e.to_string()))?;
        // The draft model is an MTP head-only GGUF (only the nextn tensors);
        // llama.cpp must build just the head graph via `ctx_type = MTP`, else it
        // tries to resolve the full trunk's tensors (which are not in the file).
        let draft_ctx = draft
            .inner
            .new_context(
                backend.inner(),
                draft_params.with_context_type(LlamaContextType::Mtp),
            )
            .map_err(|e| Error::ContextCreate(e.to_string()))?;

        let eos = target.inner.token_eos();
        let n_ctx = target_ctx.n_ctx() as i32;
        let n_batch = target_ctx.n_batch() as usize;

        if spd.n_max + 1 > n_batch as i32 {
            return Err(Error::Mtp(
                "draft length (n_max + 1) exceeds the target context batch size".into(),
            ));
        }

        let spd_params = MtpSpeculativeParams {
            n_max: spd.n_max,
            n_min: spd.n_min,
            p_min: spd.p_min,
        };
        let mtp = MtpSpeculative::new(target_ctx, draft_ctx, spd_params)
            .map_err(|e| Error::Mtp(e.to_string()))?;

        Ok(Self {
            mtp,
            eos,
            n_ctx,
            n_batch,
        })
    }

    /// Target-context size (max tokens).
    #[must_use]
    pub fn n_ctx(&self) -> u32 {
        self.n_ctx as u32
    }

    /// Decode one batch, feeding llama.cpp's MTP process hook.
    fn decode_and_process(&mut self, batch: &mut LlamaBatch<'_>) -> Result<()> {
        self.mtp
            .target_context_mut()
            .decode(batch)
            .map_err(|e| Error::Decode(e.to_string()))?;
        self.mtp
            .process(batch)
            .map_err(|e| Error::Mtp(e.to_string()))?;
        Ok(())
    }

    /// Greedy token id at batch index `i` for the last target decode.
    fn greedy_at(&self, i: i32) -> Option<LlamaToken> {
        let mut best: Option<(f32, LlamaToken)> = None;
        for cand in self.mtp.target_context().candidates_ith(i) {
            if best.is_none_or(|(p, _)| cand.logit() > p) {
                best = Some((cand.logit(), cand.id()));
            }
        }
        best.map(|(_, t)| t)
    }

    /// Generate text with MTP speculative decoding.
    ///
    /// `on_chunk` is called with each decoded piece (streaming); when `metrics`
    /// is set, llama-bench style phases are recorded (pp/tg/TTFT).
    pub fn generate(
        &mut self,
        model: &Model,
        prompt: &str,
        opts: &crate::safe::GenerateOptions,
        mut on_chunk: Option<&mut dyn FnMut(&str)>,
        mut metrics: Option<&mut InferenceMetrics>,
    ) -> Result<String> {
        let start = Instant::now();
        if opts.max_tokens == 0 {
            return Ok(String::new());
        }
        let tokens = model
            .inner
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| Error::Tokenize(e.to_string()))?;
        if tokens.is_empty() {
            return Ok(String::new());
        }

        let _eos = self.eos;
        let seq_id: i32 = 0;
        let prompt_tokens = tokens.len() as u32;
        let mut decodes = 0u32;

        self.mtp
            .begin(&tokens)
            .map_err(|e| Error::Mtp(e.to_string()))?;

        // Prefill: decode the prompt in chunks with EXPLICIT positions
        // (`add_sequence` restarts positions at 0 for every chunk, which would
        // corrupt a multi-chunk prefill). Call `process()` after every decode
        // so the draft context KV stays mirrored with the target.
        let mut prefill_batch = LlamaBatch::new(self.n_batch, 1);
        let mut pos0 = 0i32;
        let mut prefill_chunk_len = 0i32;
        for chunk in tokens.chunks(self.n_batch) {
            prefill_batch.clear();
            prefill_chunk_len = chunk.len() as i32;
            // MTP requires output (logits + hidden states) on every batch row
            // so that process() can mirror embeddings into the draft context.
            for (j, t) in chunk.iter().enumerate() {
                prefill_batch
                    .add(*t, pos0 + j as i32, &[seq_id], true)
                    .map_err(|e| Error::Decode(e.to_string()))?;
            }
            self.decode_and_process(&mut prefill_batch)?;
            pos0 += chunk.len() as i32;
            decodes += 1;
        }
        let prompt_ms = start.elapsed().as_millis() as u64;
        if let Some(m) = metrics.as_mut() {
            m.decode_count = decodes;
            m.prompt_tokens = prompt_tokens;
            m.prompt_ms = prompt_ms;
        }

        let mut running_output = String::new();
        running_output.reserve((opts.max_tokens as usize).saturating_mul(4));
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let stop_sequences_empty = opts.stop_sequences.is_empty();

        // n_past = number of real tokens already in the target KV cache
        // (i.e. the next free position).
        let mut n_past = pos0;

        // Track the real (accepted) token sequence for the draft prompt param.
        let mut real: Vec<LlamaToken> = tokens.clone();

        let mut n_gen = 0u32;

        // The first token is sampled from the last prefill row (it becomes the
        // first `id_last`). All later tokens come from verified draft rounds.
        let first = self.greedy_at(prefill_chunk_len - 1);
        let first = match first {
            Some(t) => t,
            None => {
                return Err(Error::Mtp(
                    "no logits after prefill for the first token".into(),
                ));
            }
        };
        let mut id_last = first;
        #[allow(unused_assignments)]
        let mut ttft_ms: Option<u64> = None;

        // Emit the first token now (sampled from prefill, not yet decoded).
        // The server emits this via the non-spec process_token path before
        // the first speculative round.
        {
            if opts.stop_at_eos && model.inner.is_eog_token(first) {
                if let Some(m) = metrics.as_mut() {
                    m.tokens_generated = 1;
                    m.wall_time_ms = start.elapsed().as_millis() as u64;
                    m.ttft_ms = Some(start.elapsed().as_millis() as u64);
                    m.eval_ms = m.wall_time_ms.saturating_sub(m.prompt_ms);
                    m.decode_count = decodes;
                }
                return Ok(String::new());
            }
            n_gen += 1;
            let piece = match model.inner.token_to_piece(first, &mut decoder, false, None) {
                Ok(p) => p,
                Err(e) => return Err(Error::TokenToString(e.to_string())),
            };
            running_output.push_str(&piece);
            ttft_ms = Some(start.elapsed().as_millis() as u64);
            if let Some(m) = metrics.as_mut() {
                m.ttft_ms = ttft_ms;
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
                if let Some(m) = metrics.as_mut() {
                    m.tokens_generated = n_gen;
                    m.wall_time_ms = start.elapsed().as_millis() as u64;
                    m.ttft_ms = ttft_ms.or(m.ttft_ms);
                    m.eval_ms = m.wall_time_ms.saturating_sub(m.prompt_ms);
                    m.decode_count = decodes;
                }
                return Ok(running_output);
            }
        }

        let mut batch = LlamaBatch::new(self.n_batch, 1);

        while n_gen < opts.max_tokens && n_past < self.n_ctx {
            let drafts = self
                .mtp
                .draft(n_past, id_last, &real)
                .map_err(|e| Error::Mtp(e.to_string()))?;

            // draft() wrote speculative MTP activations for the draft region
            // (positions n_past..) into the draft-context KV. The verify decode
            // below re-covers exactly that region from the target's verified
            // embeddings; M-RoPE forbids decoding positions that do not
            // strictly advance past the current KV max (X < Y). Roll the draft
            // region back to n_past so process() can rewrite it.
            // (mirrors server-context.cpp spec checkpoints: seq_rm at
            //  ckpt.pos_max + 1 after common_speculative_draft)
            self.mtp
                .draft_context_mut()
                .kv_cache_seq_rm(seq_id, Some(n_past as u32), None)
                .map_err(|e| Error::Decode(e.to_string()))?;

            // Tokens to emit this round (matched drafts + 1 corrected token).
            let mut pending: Vec<LlamaToken> = Vec::new();
            let n_accept: i32;

            if !drafts.is_empty() {
                // Verify batch: [id_last @ n_past, draft0 @ n_past+1, ...].
                // Logits row i (token at n_past+i) predicts the token at
                // n_past+i+1; greedy match against draft[i] decides acceptance.
                batch.clear();
                batch
                    .add(id_last, n_past, &[seq_id], true)
                    .map_err(|e| Error::Decode(e.to_string()))?;
                let base = n_past;
                for (i, d) in drafts.iter().enumerate() {
                    let pos = base + 1 + i as i32;
                    if pos >= self.n_ctx {
                        break; // running out of context: stop adding drafts
                    }
                    batch
                        .add(*d, pos, &[seq_id], true)
                        .map_err(|e| Error::Decode(e.to_string()))?;
                }
                self.decode_and_process(&mut batch)?;
                decodes += 1;

                let batch_rows = batch.n_tokens();

                // Greedy-match consecutive drafts: row i vs draft[i].
                let mut i = 0i32;
                while i < batch_rows - 1 {
                    let g = self.greedy_at(i);
                    match g {
                        Some(g) if g == drafts[i as usize] => {
                            pending.push(drafts[i as usize]);
                            i += 1;
                        }
                        Some(g) => {
                            // First mismatch: the target's greedy token is the
                            // corrected token for position n_past + i + 1.
                            pending.push(g);
                            break;
                        }
                        None => {
                            // No logits: cannot verify beyond here.
                            break;
                        }
                    }
                }
                // All drafts accepted: the target continues with one more token
                // sampled from the last decoded draft row (row batch_rows-1).
                if i == batch_rows - 1 {
                    if let Some(g) = self.greedy_at(batch_rows - 1) {
                        pending.push(g);
                    }
                }

                n_accept = i;

                // If every draft fit (no early break for n_ctx), accept() sees
                // a full-length draft. Otherwise tell it only the rows we fed.
                let n_fed = batch_rows as usize - 1;
                let n_accept_actual = (n_accept as usize).min(n_fed);
                self.mtp
                    .accept(n_accept_actual as u16)
                    .map_err(|e| Error::Mtp(e.to_string()))?;
            } else {
                // No draft this round: decode id_last alone and take the
                // greedy next token as a single non-verified token.
                batch.clear();
                batch
                    .add(id_last, n_past, &[seq_id], true)
                    .map_err(|e| Error::Decode(e.to_string()))?;
                self.decode_and_process(&mut batch)?;
                decodes += 1;
                if let Some(g) = self.greedy_at(0) {
                    pending.push(g);
                }
                n_accept = 0;
            }

            // KV cache bookkeeping: id_last was decoded at position n_past,
            // followed by `n_accept` accepted drafts. Any draft positions
            // beyond the accepted prefix are junk and must be cleared so the
            // next decode continues at the corrected token position.
            let valid_after = n_past + n_accept + 1;
            if valid_after <= self.n_ctx {
                self.mtp
                    .target_context_mut()
                    .kv_cache_seq_rm(seq_id, Some(valid_after as u32), None)
                    .map_err(|e| Error::Decode(e.to_string()))?;
                // The draft context mirrors these positions; clear the
                // rejected tail there too so the next draft/process cycle
                // starts from a strictly increasing position.
                // (mirrors server-context.cpp slot.mem.seq_rm(pos_next, -1))
                self.mtp
                    .draft_context_mut()
                    .kv_cache_seq_rm(seq_id, Some(valid_after as u32), None)
                    .map_err(|e| Error::Decode(e.to_string()))?;
            }

            // Emit pending tokens.
            for tok in &pending {
                if opts.stop_at_eos && model.inner.is_eog_token(*tok) {
                    n_gen += 1;
                    break;
                }
                n_gen += 1;

                let piece = match model.inner.token_to_piece(*tok, &mut decoder, false, None) {
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
            }

            // Advance state: accepted real tokens are id_last + the matched
            // drafts; the final pending token (corrected/single) is the next
            // id_last and is NOT yet in the KV cache.
            if let Some(last) = pending.last() {
                real.push(id_last);
                real.extend(&pending[..pending.len().saturating_sub(1)]);
                id_last = *last;
                n_past += pending.len() as i32;
            } else {
                id_last = self.eos;
            }

            if n_gen >= opts.max_tokens || n_past >= self.n_ctx {
                break;
            }
        }

        if let Some(m) = metrics.as_mut() {
            m.tokens_generated = n_gen;
            m.wall_time_ms = start.elapsed().as_millis() as u64;
            m.ttft_ms = ttft_ms.or(m.ttft_ms);
            m.eval_ms = m.wall_time_ms.saturating_sub(m.prompt_ms);
            m.decode_count = decodes;
        }
        Ok(running_output)
    }

    /// Generate with streaming + metrics.
    pub fn generate_stream<F>(
        &mut self,
        model: &Model,
        prompt: &str,
        opts: &crate::safe::GenerateOptions,
        on_chunk: F,
    ) -> Result<(String, InferenceMetrics)>
    where
        F: FnMut(&str),
    {
        let mut metrics = InferenceMetrics::default();
        let mut cb = on_chunk;
        let s = self.generate(model, prompt, opts, Some(&mut cb), Some(&mut metrics))?;
        Ok((s, metrics))
    }
}

/// Expose the inner draft context for advanced (cache) operations.
impl<'a> MtpSession<'a> {
    /// Access the draft context (e.g. to inspect or reset its KV cache).
    pub fn draft_context_mut(&mut self) -> &mut LlamaContext<'a> {
        self.mtp.draft_context_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtp_params_defaults() {
        let p = MtpParams::default();
        assert_eq!(p.n_max, 3);
        assert_eq!(p.n_min, 0);
        assert!((p.p_min - 0.0).abs() < 1e-6);
    }

    #[test]
    fn mtp_params_builder() {
        let p = MtpParams::new().with_n_max(5).with_n_min(1).with_p_min(0.5);
        assert_eq!(p.n_max, 5);
        assert_eq!(p.n_min, 1);
        assert!((p.p_min - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mtp_params_eq() {
        assert_eq!(MtpParams::default(), MtpParams::new());
        assert_ne!(MtpParams::default(), MtpParams::new().with_n_max(2));
    }
}
