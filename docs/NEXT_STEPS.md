# Next Steps — Development Priorities (Rust Architect View)

Prioritized roadmap after Phase 1–4. Order: **stability → API ergonomics → performance → features**.

---

## P0 — Stability and correctness

| # | Step | Why |
|---|------|-----|
| 1 | **Eliminate deprecation** | Replace `token_to_str` with proper use of `token_to_piece` (decoder + options) so upgrades of `llama-cpp-2` don’t break. |
| 2 | **Error context** | Add path to `ModelLoad` when converting from `LlamaModelLoadError` (if the upstream error carries it), and consider `.source()` / `#[cause]` for chaining. |
| 3 | **CI** | GitHub Actions: cargo check, test, build --release on Windows (MSVC + LIBCLANG_PATH).

**Outcome:** Solid base, no known tech debt, green CI.

---

## P1 — API ergonomics and clarity

| # | Step | Why |
|---|------|-----|
| 4 | ~~Builder for options~~ Done | GenerateOptions::builder().max_tokens(64).temperature(0.5).build().
| 5 | **Typed params** | Re-export or wrap `LlamaModelParams` / `LlamaContextParams` with Rust-friendly defaults and docs (e.g. `ModelParams::default()`, `ContextParams::default()`) so users don’t need to touch llama-cpp-2 types for common use. |
| 6 | ~~Streaming~~ Done | generate_stream(model, context, prompt, opts, |chunk|) yields each piece; returns full string.

**Outcome:** Pleasant, self-explanatory API for embedding and CLI.

---

## P2 — Performance and observability

| # | Step | Why |
|---|------|-----|
| 7 | **Benchmark time-to-first-token** | Add a bench that reports time from first `decode` to first sampled token (user-visible latency). |
| 8 | **Structured metrics** | Optional `#[cfg(feature = "metrics")]` or a small module that records tokens generated, decode count, and wall time for logging/telemetry. |
| 9 | **Batch size and context** | Document (and optionally validate) `n_batch` / `n_ctx` vs. memory and throughput; consider helpers or presets (e.g. “low memory”, “max speed”). |

**Outcome:** Measurable, tunable performance and clear docs for sizing.

---

## P3 — Features and polish

| # | Step | Why |
|---|------|-----|
| 10 | **Optional local llama.cpp** | If needed: env or config to point at a local llama.cpp master for custom build (see Phase 2 optional in PLAN.md). |
| 11 | **Stop sequences** | Support stop strings (e.g. `["\n\n", "Human:"]`) in `GenerateOptions` and stop generation when any match is found. |
| 12 | **CLI flags** | Extend CLI: `--max-tokens`, `--temperature`, `--seed`, `--no-eos` (stop_at_eos), optional `--system` prompt. |
| 13 | **Embeddings API** | If useful: expose `encode()`-based API for embedding a string (batch encode → return slice or Vec<f32>) behind a feature. |

**Outcome:** Feature-complete CLI and optional embeddings for downstream apps.

---

## Summary order

```
P0: deprecation fix → error context → CI
P1: GenerateOptions builder → typed params → generate_stream
P2: time-to-first-token bench → metrics feature → n_batch/n_ctx docs
P3: local llama.cpp (optional) → stop sequences → CLI flags → embeddings (optional)
```

Update this list as items are done or reprioritized.
