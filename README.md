# Llama-RS · Ultra-fast & safe Llama on Rust

**Llama-RS** is a Rust-first wrapper and CLI around [llama.cpp](https://github.com/ggml-org/llama.cpp), built for **speed**, **safety**, and **64-bit Windows** releases.

## Why ultra-fast?

- **Rust all the way** — Control flow, batching, and orchestration run in Rust; only the heavy inference stays in the native llama.cpp layer.
- **Zero-cost abstractions** — Minimal allocations on the hot path; design avoids unnecessary copies where possible.
- **64-bit native** — Release builds produce a single `llama_rs.exe` (x86_64-pc-windows-msvc) for maximum performance on modern hardware.
- **Safe by default** — A thin FFI layer wraps the C API; the rest of the API is written in **safe Rust** for memory and thread safety.

## Quick start

```bash
# Clone and build (requires Rust toolchain)
cd llama-rs-project
cargo build --release

# Run
.\target\release\llama_rs.exe
```

You should see: **Hello, Llama Rust!**

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
