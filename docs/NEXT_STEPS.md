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
| 5 | ~~Typed params~~ Done | ModelParams, ContextParams re-exported in lib. Re-export or wrap `LlamaModelParams` / `LlamaContextParams` with Rust-friendly defaults and docs (e.g. `ModelParams::default()`, `ContextParams::default()`) so users don’t need to touch llama-cpp-2 types for common use. |
| 6 | ~~Streaming~~ Done | generate_stream(model, context, prompt, opts, |chunk|) yields each piece; returns full string.

**Outcome:** Pleasant, self-explanatory API for embedding and CLI.

---

## P2 — Performance and observability

| # | Step | Why |
|---|------|-----|
| 7 | ~~Benchmark time-to-first-token~~ Done | bench time_to_first_token when LLAMA_RS_BENCH_MODEL set.
| 8 | ~~Structured metrics~~ Done | Feature metrics: InferenceMetrics, generate_with_metrics.
| 9 | ~~Batch size and context~~ Done | docs/SIZING.md. Document (and optionally validate) `n_batch` / `n_ctx` vs. memory and throughput; consider helpers or presets (e.g. “low memory”, “max speed”). |

**Outcome:** Measurable, tunable performance and clear docs for sizing.

---

## P3 — Features and polish

| # | Step | Why |
|---|------|-----|
| 10 | **Optional local llama.cpp** | If needed: env or config to point at a local llama.cpp master for custom build (see Phase 2 optional in PLAN.md). |
| 11 | ~~Stop sequences~~ Done | GenerateOptions.stop_sequences, builder .stop_sequence(s), match after each token.
| 12 | ~~CLI flags~~ Done | clap: --max-tokens, --temperature, --seed, --no-eos, --system.
| 13 | ~~Embeddings API~~ Done | Feature embeddings: embed(model, context, text) -> Vec<f32>.

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

---

## What to do next (in order)

1. **P3.10 — Optional local llama.cpp** (only remaining item)  
   If you need to build against a custom llama.cpp checkout: add env or config (e.g. `LLAMA_CPP_PATH`) and wire it in build or via llama-cpp-2 docs. Skip if the bundled build is enough.

2. **Maintenance and polish**
   - Bump `llama-cpp-2` when a new version is released; run tests and fix any breaking changes.
   - Add more integration tests or examples if you add new use cases.
   - Optionally: publish crate to crates.io (`cargo publish`), add a short CHANGELOG.

3. **Possible future work**
   - **Multi-platform CI** — add Linux/macOS to GitHub Actions.
   - **Chat / tool-calling** — if llama-cpp-2 adds chat templates or tools, expose them in the API.
   - **Embedding normalization** — optional mean-pool or L2 norm for `embed()` output when using embedding models.
   - **Presets** — `ContextParams` helpers like `low_memory()` / `max_speed()` (see SIZING.md).
