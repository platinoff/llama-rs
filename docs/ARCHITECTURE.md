# Llama-RS Architecture

## Overview

Llama-RS is built as a **Rust-first** layer on top of the llama.cpp C API. The architecture is split into layers with a minimal amount of unsafe code.

## Layers

```
┌─────────────────────────────────────────────────────────┐
│  CLI / Application (main.rs, safe Rust)                 │
├─────────────────────────────────────────────────────────┤
│  Public API (lib.rs, safe Rust)                          │
│  - Model loading, context, sampling, batching            │
├─────────────────────────────────────────────────────────┤
│  Safe wrappers (src/safe/)                              │
│  - Backend, Model, Context, GenerateOptions, generate   │
├─────────────────────────────────────────────────────────┤
│  llama-cpp-2 crate (FFI to llama.cpp)                  │
├─────────────────────────────────────────────────────────┤
│  llama.cpp (C/C++) — built/linked by llama-cpp-sys-2    │
└─────────────────────────────────────────────────────────┘
```

## Modules

| Module    | Purpose |
|-----------|---------|
| `lib.rs`  | Public API (safe Rust): Backend, Model, Context, generate, Error, Result. |
| `error.rs`| Unified Error and Result; conversions from llama-cpp-2 errors. |
| `safe/`   | Safe wrappers: Backend, Model, Context, GenerateOptions, generate (pure Rust loop). |

FFI is confined to the **llama-cpp-2** dependency; no unsafe code in this repository.

## Data flow (future inference)

1. **Model load** — GGUF path → FFI `llama_load_model_from_file` → safe `Model`.
2. **Context** — `Model` + params → `llama_new_context_with_model` → safe `Context`.
3. **Decode** — Rust builds `llama_batch`, calls `llama_decode` → logits returned to Rust.
4. **Sampling** — logits → sampler API → next token; loop in Rust.

## Build dependencies

- **Cargo.toml**: `llama-cpp-2` (with optional `sampler` feature); the crate builds/links llama.cpp.
- No custom `build.rs` in this repo; 100% of our code is Rust.

## Target platform

- **x86_64-pc-windows-msvc** — release artifact: a single 64-bit `llama_rs.exe`.

This document will be updated as new modules and llama.cpp integration are added.
