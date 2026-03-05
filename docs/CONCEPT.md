# llama.rs Concept

## Idea

Maximum code in **Rust** for speed and safety; minimal, stable FFI to llama.cpp. The final product is a single 64-bit exe built with cargo.

**Rust share:** All code in this repository is Rust. The only non-Rust is the `llama-cpp-2` (and its sys) dependency, which wraps and builds llama.cpp. So this project is **>90% Rust** by design (effectively 100% of *our* code).

## Principles

1. **Safe by default** — Unsafe code only in an isolated FFI layer; everything else is safe Rust.
2. **Zero-cost abstractions** — Abstractions add no overhead in release builds.
3. **Ultra-speed** — Minimal allocations on the inference path, batching, zero-copy where possible.
4. **Git-friendly** — Clean layout, submodule or external path to llama.cpp, no large binaries in the repo.
5. **MIT** — Permissive license for use and distribution.

## Data source

- The working **llama.cpp** version is taken from the **master** folder (e.g. `S:\rust\llama-rs\llama.cpp-master` or nested `llama.cpp-master/llama.cpp-master`).
- Building libllama is done during `cargo build` (build.rs) or manually with subsequent linking.

## Development tools

- **rustc** (via cargo) — compilation.
- **cargo** — build, test, bench.
- **git** — version control; for push use **gittoken** (token or credential helper).

This document describes the overall concept; implementation details are in [PLAN.md](PLAN.md) and [ARCHITECTURE.md](ARCHITECTURE.md).
