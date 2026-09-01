//! Inference metrics for logging and telemetry (llama-bench style: pp/tg/TTFT).

/// Metrics collected during one generation run. Compatible with `llama-bench` phases:
/// `pp` = prompt processing (prefill, compute-bound), `tg` = token generation (decode, memory-bound),
/// plus `TTFT` (time to first token, user-visible latency).
#[derive(Clone, Debug, Default)]
pub struct InferenceMetrics {
    /// Number of new tokens generated (excluding prompt). `tg` count.
    pub tokens_generated: u32,
    /// Number of prompt tokens (input). For `pp` throughput.
    pub prompt_tokens: u32,
    /// Number of decode steps (prompt decode + one per generated token).
    pub decode_count: u32,
    /// Wall-clock time for the full generation in milliseconds.
    pub wall_time_ms: u64,
    /// Time to first generated token in ms (from start until first `on_chunk`). `None` if no token.
    pub ttft_ms: Option<u64>,
    /// Prompt processing time (prefill) in ms.
    pub prompt_ms: u64,
    /// Token generation (decode) time in ms (wall - prompt).
    pub eval_ms: u64,
}

impl InferenceMetrics {
    /// Tokens per second for generation phase (`tg tok/s`): `tokens_generated / (eval_ms / 1000)`.
    #[must_use]
    pub fn tokens_per_sec(&self) -> f32 {
        if self.eval_ms == 0 {
            return 0.0;
        }
        self.tokens_generated as f32 / (self.eval_ms as f32 / 1000.0)
    }

    /// Prompt tokens per second (`pp tok/s`): `prompt_tokens / (prompt_ms / 1000)`.
    #[must_use]
    pub fn prompt_tokens_per_sec(&self) -> f32 {
        if self.prompt_ms == 0 {
            return 0.0;
        }
        self.prompt_tokens as f32 / (self.prompt_ms as f32 / 1000.0)
    }

    /// Compact JSON for GSV live ingest / bench logs. No serde dep.
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"tokens_generated":{},"prompt_tokens":{},"decode_count":{},"wall_time_ms":{},"ttft_ms":{},"prompt_ms":{},"eval_ms":{},"tokens_per_sec":{:.3},"prompt_tokens_per_sec":{:.3}}}"#,
            self.tokens_generated,
            self.prompt_tokens,
            self.decode_count,
            self.wall_time_ms,
            self.ttft_ms.map_or("null".to_string(), |v| v.to_string()),
            self.prompt_ms,
            self.eval_ms,
            self.tokens_per_sec(),
            self.prompt_tokens_per_sec()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_per_sec_calc() {
        let m = InferenceMetrics {
            tokens_generated: 32,
            eval_ms: 1000,
            ..Default::default()
        };
        assert!((m.tokens_per_sec() - 32.0).abs() < 1e-3);
    }

    #[test]
    fn prompt_tokens_per_sec_calc() {
        let m = InferenceMetrics {
            prompt_tokens: 512,
            prompt_ms: 1000,
            ..Default::default()
        };
        assert!((m.prompt_tokens_per_sec() - 512.0).abs() < 1e-3);
    }

    #[test]
    fn to_json_roundtrip() {
        let m = InferenceMetrics {
            tokens_generated: 4,
            prompt_tokens: 8,
            decode_count: 5,
            wall_time_ms: 100,
            ttft_ms: Some(20),
            prompt_ms: 30,
            eval_ms: 70,
        };
        let j = m.to_json();
        assert!(j.contains("tokens_generated"));
        assert!(j.contains("ttft_ms"));
    }
}
