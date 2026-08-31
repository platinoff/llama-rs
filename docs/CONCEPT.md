# llama.rs Concept

## Idea

**llama.rs = Llama in Rust.** All application code, API, and orchestration are in Rust. Tensor ops / KV cache are delegated to **llama.cpp** via `llama-cpp-2` — single 64-bit exe via cargo, **maximum Rust**, minimal FFI. No smaller GGUF needed: 27B runs via mmap on 5500U; we control RAM **staged** (disk → RAM).

**Rust share:** 100% of this repo is Rust (99.46% product ratio via `gsv-loc-audit --stretch-96`). Non-Rust is only the linked `llama.cpp` built by `llama-cpp-sys-2`. So **llama.rs = Rust side; llama.cpp = backend.** Non-RS files (shell/YAML) are replaced with `cargo xtask` (Rust) where logical.

## Principles

1. **Safe by default** — Unsafe only in isolated FFI layer; rest is safe Rust.
2. **Zero-cost abstractions** — No overhead in release.
3. **Ultra-speed** — Minimal allocs on inference path, batching, zero-copy, `tokens_per_sec` / `time_to_first_token` benches.
4. **Staged loading** — Model load is `mmap` (paged) → optional `mlock`/`prefault`, with `progress_callback` (0.0..1.0, abort). Controls disk→RAM without needing 7B.
5. **GSV-live compatible** — Product registered in `S:/rust/GSV/docs/gsv/PRODUCTS.md`; `abrakadabra` drain, one-commit-per-session, live vision at `127.0.0.1:9999` optional.
6. **Git-friendly + MIT** — Clean layout, no binaries in repo.

## Data source

- **llama.cpp** built by `llama-cpp-2` crate during `cargo build` (no manual clone). Params `use_mmap`/`use_mlock`/`no_alloc` + `with_progress_callback` control staged loading.

## Development tools

- **rustc** via cargo, **cargo** (build/test/bench/`xtask`), **git** (+ gittoken), **GSV live** (`http://127.0.0.1:9999` for `abrakadabra`).

This concept is implemented in [PLAN.md](PLAN.md) and [ARCHITECTURE.md](ARCHITECTURE.md); priorities in [NEXT_STEPS.md](NEXT_STEPS.md).
