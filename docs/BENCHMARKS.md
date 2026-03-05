# Benchmarks (ultra-speed)

## Running benchmarks

From the project root (with [build environment](DEVELOPMENT.md#build) set):

```bash
cargo bench
```

## Current benchmarks

- **`hello_llama_rust`** — measures the cost of the greeting helper (baseline; no model load).

## Adding inference metrics

When `LLAMA_RS_BENCH_MODEL` is set to a GGUF path, the **`inference_tokens_per_sec`** benchmark runs:

- Loads the model once, then measures time per short generation (32 tokens, stop_at_eos).
- Approximate tokens/sec = 32 / (time per iteration in seconds).

## Verification

Example with a real model:

```cmd
set LLAMA_RS_BENCH_MODEL=S:\path\to\model.gguf
cargo bench --bench speed
```

Document your hardware and results in this file or in release notes.

- Release build is 64-bit: `target\release\llama_rs.exe` (x86_64-pc-windows-msvc).
- For consistent numbers, use `cargo bench` with `--release` (default for bench) and close other heavy applications.
