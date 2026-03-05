# llama.rs Architecture

## Overview

**llama.rs** is implemented in **Rust**. The public API, inference loop (tokenize → decode → sample → generate), streaming, options, and error handling are all Rust. The only non-Rust is the **llama.cpp** library, which is linked as the **backend** for model evaluation (tensor ops, KV cache). So: **llama.rs = Rust; llama.cpp = backend.**

## Layers

```
┌─────────────────────────────────────────────────────────┐
│  CLI / Application (main.rs — Rust)                     │
├─────────────────────────────────────────────────────────┤
│  Public API (lib.rs — Rust)                              │
│  - Model, Context, generate, generate_stream, embed      │
├─────────────────────────────────────────────────────────┤
│  llama.rs logic (src/safe/ — Rust)                       │
│  - Backend, Model, Context, GenerateOptions, generate,   │
│    generate_stream, embed (orchestration in Rust)        │
├─────────────────────────────────────────────────────────┤
│  llama-cpp-2 (FFI bindings to backend)                  │
├─────────────────────────────────────────────────────────┤
│  llama.cpp (C/C++) — backend, built by llama-cpp-sys-2   │
└─────────────────────────────────────────────────────────┘
```

## Modules

| Module    | Purpose |
|-----------|---------|
| `lib.rs`  | Public API: Backend, Model, Context, ModelParams, ContextParams, GenerateOptions, generate, generate_stream, Error, Result. |
| `error.rs`| Unified Error and Result; conversions from llama-cpp-2 errors. |
| `safe/`   | Safe wrappers: Backend, Model, Context, GenerateOptions + builder, generate, generate_stream (pure Rust loop). |

FFI is confined to the **llama-cpp-2** dependency; **no unsafe code and no C/C++ in this repository.** All orchestration is Rust.

## Data flow (inference)

1. **Model load** — GGUF path → `Model::load_from_file(backend, path, params)` → safe `Model`.
2. **Context** — `model.new_context(backend, ctx_params)` → safe `Context`.
3. **Generate** — `generate(&model, &mut context, prompt, &opts)` or `generate_stream(..., |chunk| { ... })`; both run in Rust: tokenize → batch decode → sampler → accept token → repeat until EOS or max_tokens.
4. **Sampling** — LlamaSampler (temp, top_k, top_p, dist) applied in Rust; single token decoded per step via llama-cpp-2.

## Build dependencies

- **Cargo.toml**: `llama-cpp-2` (with optional `sampler` feature); the crate builds/links llama.cpp.
- No custom `build.rs` in this repo; 100% of our code is Rust.

## Target platform

- **x86_64-pc-windows-msvc** — release artifact: a single 64-bit `llama_rs.exe`.

This document will be updated as new modules are added. The rule remains: **llama.rs = Rust; llama.cpp = backend.**
