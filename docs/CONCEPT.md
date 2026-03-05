# llama.rs Concept

## Idea

**llama.rs = Llama in Rust.** The project is a **Rust implementation** of Llama inference: all application code, API, and orchestration are in Rust. The actual model evaluation (tensor ops, KV cache) is delegated to **llama.cpp** via the `llama-cpp-2` crate — so we get a single 64-bit exe built with cargo, with **maximum Rust** and minimal FFI.

**Rust share:** 100% of the code in this repository is Rust. There is no C/C++ in the repo. The only non-Rust is the linked llama.cpp library built by the `llama-cpp-2` dependency. So **llama.rs is the Rust side; llama.cpp is the backend.**

## Principles

1. **Safe by default** — Unsafe code only in an isolated FFI layer; everything else is safe Rust.
2. **Zero-cost abstractions** — Abstractions add no overhead in release builds.
3. **Ultra-speed** — Minimal allocations on the inference path, batching, zero-copy where possible.
4. **Git-friendly** — Clean layout, submodule or external path to llama.cpp, no large binaries in the repo.
5. **MIT** — Permissive license for use and distribution.

## Data source

- **llama.cpp** is built automatically by the `llama-cpp-2` crate during `cargo build` (no manual clone required). Optionally, a local llama.cpp path can be used if the crate supports it.

## Development tools

- **rustc** (via cargo) — compilation.
- **cargo** — build, test, bench.
- **git** — version control; for push use **gittoken** (token or credential helper).

This document describes the overall concept; implementation details are in [PLAN.md](PLAN.md) and [ARCHITECTURE.md](ARCHITECTURE.md).
