//! Inference metrics for logging and telemetry (optional feature `metrics`).

/// Metrics collected during one generation run.
#[derive(Clone, Debug, Default)]
pub struct InferenceMetrics {
    /// Number of new tokens generated (excluding prompt).
    pub tokens_generated: u32,
    /// Number of decode steps (prompt decode + one per generated token).
    pub decode_count: u32,
    /// Wall-clock time for the full generation in milliseconds.
    pub wall_time_ms: u64,
}
