# Benchmarks (ultra-speed)

## Running benchmarks

From the project root (with [build environment](DEVELOPMENT.md#build) set):

```bash
cargo bench
```

## Current benchmarks

- **`hello_llama_rust`** — measures the cost of the greeting helper (baseline; no model load).
- **`inference_tokens_per_sec`** (when `LLAMA_RS_BENCH_MODEL` is set) — time per short generation (32 tokens); tokens/sec ≈ 32 / time_s.
- **`time_to_first_token`** (when `LLAMA_RS_BENCH_MODEL` is set) — latency from start of generation to first decoded token (user-visible time-to-first-token).

## Adding inference metrics

When `LLAMA_RS_BENCH_MODEL` is set to a GGUF path, the **`inference_tokens_per_sec`** benchmark runs:

- Loads the model once, then measures time per short generation (32 tokens, stop_at_eos).
- Approximate tokens/sec = 32 / (time per iteration in seconds).

## Results (2026-08-30)

Hardware: AMD Ryzen 5 5500U (6c/12t), 16 GB RAM (≈0.6 GiB free during run),
Windows 10, release profile, `n_ctx_default = 512`.

Model: `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` (27B Gated Delta Net / M-RoPE,
threads = physical cores).

Run: `LLAMA_RS_BENCH_MODEL=models/Qwen3.8-27B-UD-IQ2_XXS.gguf cargo bench
--bench speed -- --sample-size 10`.

| Benchmark | Median | Notes |
|---|---|---|
| `hello_llama_rust` | 1.53 ns | baseline; no model load |
| `inference_tokens_per_sec` | 1020.1 s / iter | 32-token generation (stop_at_eos); ≈ 0.031 tok/s |
| `time_to_first_token` | 248.25 s | first decoded token after prefill |

> `inference_tokens_per_sec` ≈ 32 / 1020.1 s ≈ **0.031 tokens/s** on this
> 27B IQ2_XXS model with mmap on a 5500U. Expect seconds-scale numbers only
> with a smaller / quantized model or a GPU. Memory pressure (≈0.6 GiB free)
> throttles the run heavily.

## Verification

Example with a real model:

```cmd
set LLAMA_RS_BENCH_MODEL=S:\path\to\model.gguf
cargo bench --bench speed
```

Document your hardware and results in this file or in release notes.

- Release build is 64-bit: `target\release\llama_rs.exe` (x86_64-pc-windows-msvc).
- For consistent numbers, use `cargo bench` with `--release` (default for bench) and close other heavy applications.

See [SIZING.md](SIZING.md) for `n_ctx` / `n_batch` and memory vs throughput.
