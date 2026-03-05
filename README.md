# Llama-RS · Ultra-fast & safe Llama on Rust

**Llama-RS** is a Rust-first wrapper and CLI around [llama.cpp](https://github.com/ggml-org/llama.cpp), built for **speed**, **safety**, and **64-bit Windows** releases.

## Why ultra-fast?

- **Rust all the way** — The entire public API and inference loop are written in **safe Rust** (>90% of this repo). FFI is confined to the `llama-cpp-2` dependency; no `unsafe` in our codebase.
- **Zero-cost abstractions** — Thin wrappers (Backend, Model, Context, generate) add no extra allocation on the hot path.
- **64-bit native** — Release builds produce a single `llama_rs.exe` (x86_64-pc-windows-msvc) for maximum performance.
- **Safe by default** — Idiomatic `Result` and `Error` types; all orchestration (batching, sampling, token loop) in Rust.

## Quick start

```bash
# Build (requires Rust; for 64-bit Windows MSVC also install "Desktop development with C++")
cargo build --release

# Print greeting
.\target\release\llama_rs.exe

# Run with a GGUF model (and optional prompt)
.\target\release\llama_rs.exe path\to\model.gguf "Your prompt"
```

## Requirements

- **Rust** 1.70+ (`rustup default stable-x86_64-pc-windows-msvc` for 64-bit Windows).
- **llama.cpp** — Master branch used from a sibling folder (e.g. `../llama.cpp-master`); see [docs/PLAN.md](docs/PLAN.md) and [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Project layout

| Path        | Description                          |
|------------|--------------------------------------|
| `src/lib.rs` | Core library and public API          |
| `src/main.rs`| CLI entry point (64-bit exe)         |
| `docs/`      | Plan, architecture, and dev guides   |
| `tests/`    | Integration tests                    |
| `benches/`  | Performance benchmarks               |

## Documentation

- [docs/PLAN.md](docs/PLAN.md) — Project plan and phases.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — Architecture and module layout.
- [docs/CONCEPT.md](docs/CONCEPT.md) — Design concepts (Rust-first, safety, speed).
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) — Build, test, benchmark (rustc, cargo, git).

## License

MIT — see [LICENSE](LICENSE).

## Contributing

See `docs/DEVELOPMENT.md` for build and test instructions. Use `cargo test` and `cargo bench` to verify correctness and ultra-speed.
