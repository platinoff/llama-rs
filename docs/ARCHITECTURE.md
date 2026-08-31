# llama.rs Architecture

## Overview

**llama.rs** is **pure Rust** (99.46% ratio, `gsv-loc-audit --stretch-96` ≥96%). Public API, inference loop, staged loading, streaming, options, errors are Rust. Non-Rust is only the **llama.cpp** backend (linked via `llama-cpp-2`). Shell/YAML replaced with `cargo xtask` where logical; GSV-live compatible. **llama.rs = Rust; llama.cpp = backend.**

## Layers

```
┌─────────────────────────────────────────────────────────┐
│  CLI / Application (main.rs — Rust, clap)               │
│  --mmap/--no-mmap --mlock --progress                    │
├─────────────────────────────────────────────────────────┤
│  Public API (lib.rs — Rust)                              │
│  - Model, Context, generate, generate_stream, embed      │
│  - StagedLoadOptions, Model::load_staged (staged)       │
├─────────────────────────────────────────────────────────┤
│  llama.rs logic (src/safe/ — Rust)                       │
│  - Backend, Model, Context, GenerateOptions, generate,   │
│    staged.rs (disk→RAM: mmap/mlock/progress)            │
│  - embed, metrics (pure Rust loops)                      │
├─────────────────────────────────────────────────────────┤
│  llama-cpp-2 (FFI) — LlamaModelParams with_progress_.. │
│  use_mmap/use_mlock/no_alloc + progress_callback 0.0..1.0 │
├─────────────────────────────────────────────────────────┤
│  llama.cpp (C/C++) — backend, built by llama-cpp-sys-2   │
└─────────────────────────────────────────────────────────┘
```

## Modules

| Module           | Purpose |
|------------------|---------|
| `lib.rs`         | Public API: Backend, Model, Context, ModelParams, ContextParams, GenerateOptions, StagedLoadOptions, generate, generate_stream, Error, Result. |
| `error.rs`       | Unified Error/Result; conversions from llama-cpp-2. |
| `safe/backend.rs`| Safe Backend init. |
| `safe/model.rs`  | Safe Model (`load_from_file`, `load_staged`). |
| `safe/staged.rs` | **Staged loading** — `StagedLoadOptions { use_mmap, use_mlock, on_progress }`, `LoadStage` (Mmap/Mlock/Done), progress 0.0..1.0 + abort. Controls disk→RAM ступенями. |
| `safe/context.rs`| Safe Context (decode, reset) + GenerateOptions builder. |
| `safe/generate.rs`| Pure Rust generate loop (tokenize→decode→sample). |
| `safe/embed.rs`  | Embeddings (feature-gated). |
| `metrics.rs`     | InferenceMetrics. |

No unsafe/C++ in repo; orchestration is Rust. GSV live is thin glue (optional `GSV_LIVE` → `127.0.0.1:9999`).

## Data flow

1. **Model load (staged)** — GGUF path → `Model::load_staged(backend, path, StagedLoadOptions { use_mmap:true, use_mlock:false, on_progress:|p|{...} })` → `LlamaModelParams::with_progress_callback` + `load_mode` → safe `Model`. Stages: `Mmap` (paged, 6.9 GiB mapped, ~0.6G RAM free OK) → optional `Mlock` (pin) → `Done`. `false` from callback aborts.
2. **Context** — `model.new_context(backend, ctx_params)` → `Context` (with `reset()` for hybrid M-RoPE).
3. **Generate** — `generate(&model, &mut ctx, prompt, &opts)` / `generate_stream(..., |chunk|)` — Rust loop: batch decode → sampler (temp/top_k/top_p/dist) → token.
4. **Sampling** — LlamaSampler chain applied in Rust.

## Build dependencies

- **Cargo.toml**: `llama-cpp-2` (`sampler`), `clap`, `encoding_rs`, `thiserror`. Crate builds/links llama.cpp; `.cargo/config.toml` pins `LIBCLANG_PATH`, `CMAKE`, `static-libstdc++`.
- No custom `build.rs`; 100% Rust. `cargo xtask` (future) replaces shell where logical.

## Target platform

- **x86_64-pc-windows-gnu** (release `target/release/llama_rs.exe`, 11M); `MSVC` also works if `LIBCLANG_PATH` set. GSV live optional.

Rule: **llama.rs = Rust; llama.cpp = backend; staged disk→RAM in Rust.**
