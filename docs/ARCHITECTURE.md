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
│  Safe wrappers (optional: src/safe/)                     │
│  - RAII, Result, idiomatic types                        │
├─────────────────────────────────────────────────────────┤
│  FFI layer (optional: src/ffi/)                         │
│  - bindgen-generated + thin unsafe wrappers             │
├─────────────────────────────────────────────────────────┤
│  llama.cpp (C/C++) — libllama static                     │
│  - Built from ../llama.cpp-master                        │
└─────────────────────────────────────────────────────────┘
```

## Modules (future structure)

| Module   | Purpose |
|----------|---------|
| `lib.rs` | Public API, re-exports, `hello_llama_rust` and future functions |
| `ffi`    | Low-level C API calls (unsafe), generated via bindgen |
| `safe`   | Safe wrappers (Context, Model, Sampler, etc.) |

## Data flow (future inference)

1. **Model load** — GGUF path → FFI `llama_load_model_from_file` → safe `Model`.
2. **Context** — `Model` + params → `llama_new_context_with_model` → safe `Context`.
3. **Decode** — Rust builds `llama_batch`, calls `llama_decode` → logits returned to Rust.
4. **Sampling** — logits → sampler API → next token; loop in Rust.

## Build dependencies

- **build.rs**: Resolves path to llama.cpp (env or relative), invokes cmake/cc to build libllama, and bindgen if needed.
- **Cargo.toml**: Dependencies such as `cmake`, `bindgen` (if using custom bindings); alternatively the `llama-cpp-2` crate with a path to a local build.

## Target platform

- **x86_64-pc-windows-msvc** — release artifact: a single 64-bit `llama_rs.exe`.

This document will be updated as new modules and llama.cpp integration are added.
