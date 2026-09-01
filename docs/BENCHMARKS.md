# Benchmarks (ultra-speed)

## Running benchmarks

From the project root (with [build environment](DEVELOPMENT.md#build) set):

```bash
cargo bench
```

## Current benchmarks (llama-bench style: pp/tg/TTFT)

- **`hello_llama_rust`** — baseline, no model.
- **`pp_256`** (when `LLAMA_RS_BENCH_MODEL` set) — prompt processing / prefill, 256-token prompt, 1 token gen; `pp tok/s = prompt_tokens / prompt_ms`.
- **`tg_32`** (when `LLAMA_RS_BENCH_MODEL` set) — token generation, 32 tokens; `tg tok/s = 32 / eval_s` (memory-bandwidth bound).
- **`ttft`** (when `LLAMA_RS_BENCH_MODEL` set) — time to first token, ms from start until first decoded piece.

All three share one `Backend+Model` (Backend::init once per process). Swap model via env: `LLAMA_RS_BENCH_MODEL=/path/to/Qwen.gguf` locally, `.../Nemotron.gguf` from OpenCode.

## InferenceMetrics (Rust API)

`src/metrics.rs:5` `InferenceMetrics { prompt_tokens, tokens_generated, decode_count, wall_time_ms, prompt_ms, eval_ms, ttft_ms }` + `tokens_per_sec()` / `prompt_tokens_per_sec()` / `to_json()` for GSV ingest. Collected via `generate_with_metrics` / `generate_stream_with_metrics` (`metrics` feature, `src/lib.rs:21`). `src/safe/context.rs:236` fills `prompt_ms` (prefill), `ttft_ms` (first chunk), `eval_ms = wall - prompt`.

## Results (2026-08-30 — Qwen, 2026-09-01 — expanded metrics)

Hardware: AMD Ryzen 5 5500U (6c/12t), 16 GB RAM (≈0.6 GiB free during run),
Windows 10, release profile, `n_ctx_default = 512`.

Model Qwen: `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` (27B Gated Delta Net / M-RoPE,
threads = physical cores, `use_mmap true`).

Run: `LLAMA_RS_BENCH_MODEL=models/Qwen3.8-27B-UD-IQ2_XXS.gguf cargo bench
--bench speed -- --sample-size 10`.

| Benchmark | Median | Notes |
|---|---|---|
| `hello_llama_rust` | 1.01 ns (now) / 1.53 ns (2026-08-30) | baseline; no model load |
| `tg_32` (ex `inference_tokens_per_sec`) | 1020.1 s / iter | 32-token generation; ≈ 0.031 tok/s |
| `ttft` (ex `time_to_first_token`) | 248.25 s | first decoded token after prefill |
| `pp_256` | — | new in this bench (prefill 256 tokens) |

> `tg_32` ≈ 32 / 1020.1 s ≈ **0.031 tokens/s** on this
> 27B IQ2_XXS model with mmap on a 5500U. Expect seconds-scale numbers only
> with a smaller / quantized model or a GPU. Memory pressure (≈0.6 GiB free)
> throttles the run heavily. For Nemotron 30B-A3B expect similar or slower CPU numbers; on GPU `tg` scales with VRAM bandwidth (see web research: RTX 4090 ~125 tok/s for 7B Q4).

## Verification

Qwen locally (default):
```cmd
set LLAMA_RS_BENCH_MODEL=S:\rust\llama-rs\models\Qwen3.8-27B-UD-IQ2_XXS.gguf
cargo bench --bench speed
```
Nemotron via OpenCode (swap, Qwen stays on disk):
```cmd
set LLAMA_RS_BENCH_MODEL=S:\path\to\Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf
cargo bench --bench speed -- tg_32
cargo test --features metrics -- --nocapture  # prints InferenceMetrics::to_json()
```
Metrics JSON (GSV ingest):
```rust
let (out, m) = generate_with_metrics(&model, &mut ctx, "Hi", &opts)?;
println!("{}", m.to_json()); // -> {"tokens_generated":32,"prompt_tokens":2,"ttft_ms":248000,...}
```

- Release build is 64-bit: `target\release\llama_rs.exe` (x86_64-pc-windows-msvc).
- For consistent numbers, use `cargo bench` with `--release` (default for bench) and close other heavy applications.

See [SIZING.md](SIZING.md) for `n_ctx` / `n_batch` and memory vs throughput.
