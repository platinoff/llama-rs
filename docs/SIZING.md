# Context and batch sizing

This document explains `n_ctx` and `n_batch` and how they affect memory and throughput.

## Parameters

| Parameter | Where | Meaning |
|-----------|--------|---------|
| **n_ctx** | [ContextParams](https://docs.rs/llama-cpp-2/latest/llama_cpp_2/context/params/struct.LlamaContextParams.html) (e.g. `context_params.n_ctx`) | Maximum context length in tokens. The model can attend to up to this many tokens (prompt + generated). |
| **n_batch** | Same | Maximum number of tokens to process in one decode call. Often set to 512 or 2048. |

You get the actual values from a [Context](crate::Context) with [Context::n_ctx](crate::Context::n_ctx) and [Context::n_batch](crate::Context::n_batch) after creation.

## Memory

- **Larger n_ctx** → more KV cache memory. Roughly proportional to `n_ctx * n_layer * head_dim * 2` (fp16 or similar). Reducing `n_ctx` is the main lever for “low memory” setups.
- **n_batch** affects temporary buffers per decode; typically much smaller than the KV cache. Setting `n_batch` lower than `n_ctx` saves some RAM but may require more decode steps for long prompts.

## Throughput

- **Larger n_batch** (up to `n_ctx`) → fewer decode steps for a long prompt (faster prefill). For generation, we decode one new token per step, so generation speed is mostly independent of `n_batch`.
- **n_ctx** does not directly change tokens/sec once the context is allocated; it caps how long the prompt + continuation can be.

## Presets (implemented in `src/safe/context.rs:196`)

- **Low memory:** `llama_rs::context_presets::low_memory()` → `n_ctx=2048, n_batch=512` (small KV cache, fits 16 GiB + 27B mmap).
- **Max speed (prefill):** `llama_rs::context_presets::max_speed()` → `n_ctx=4096, n_batch=2048` (batch≈ctx, fewer prefill steps).

Configure via [ContextParams](crate::ContextParams) (`LlamaContextParams::with_n_ctx`/`with_n_batch`) when creating the context; presets are pure Rust helpers, defaults via upstream.

## Staged model loading (disk → RAM ступенями)

Controls how the GGUF file is brought to RAM (`StagedLoadOptions` in `src/safe/staged.rs`, `Model::load_staged`).

| Mode | `use_mmap` | `use_mlock` | RAM | Start | Use case |
|------|------------|-------------|-----|-------|----------|
| **mmap** (default) | true | false | ~0.6 GiB free OK (27B mapped 6.9 GiB, paged) | fast | 5500U 16 GiB, low-RAM — current `BENCHMARKS.md` (0.031 tok/s, 248s TTF) |
| **resident** | false | false | ~8 GiB resident (full read) | slower | Enough RAM, no paging |
| **pinned** | true | true | pinned (mlock) | fast, no swap | Privilege + RAM, avoids swap |

Progress: `Model::load_staged(backend, path, opts, Some(|p: f32| { /* 0.0..1.0 */ true }))`; `false` aborts. CLI: `--mmap/--no-mmap --mlock --progress` + `GSV_LIVE=1` optional. Pure Rust; backend `llama-cpp-2` `load_mode` + `with_progress_callback`.
