# llama.rs · Llama in Rust

**llama.rs** is a **Rust-native** implementation of Llama inference: API, orchestration, and control flow are written in Rust. The compute backend is [llama.cpp](https://github.com/ggml-org/llama.cpp) (via the `llama-cpp-2` crate), but **this codebase is 100% Rust** — no C/C++ in the repo.

## Why Rust all the way?

- **llama.rs, not llama.cpp** — All code you see here is **Rust**. The inference loop (tokenize → decode → sample → accept) lives in `src/safe/`; only the heavy math runs in the linked llama.cpp backend.
- **Zero-cost abstractions** — Thin wrappers (Backend, Model, Context, generate) add no extra allocation on the hot path.
- **64-bit native** — Release builds produce a single `llama_rs.exe` (x86_64-pc-windows-msvc).
- **Safe by default** — Idiomatic `Result` and `Error`; no `unsafe` in this repository.

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
- **Backend:** llama.cpp is built and linked automatically by the `llama-cpp-2` dependency; see [docs/PLAN.md](docs/PLAN.md) and [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Project layout

| Path         | Description |
|-------------|-------------|
| `src/lib.rs`  | Public API (Rust) |
| `src/main.rs` | CLI (64-bit exe) |
| `src/safe/`   | Backend, Model, Context, generate, generate_stream, embed |
| `src/error.rs`| Error and Result types |
| `src/metrics.rs` | InferenceMetrics (optional feature) |
| `docs/`       | Plan, architecture, guides |
| `tests/`      | Integration tests |
| `benches/`    | Benchmarks |

## Documentation

- [docs/PLAN.md](docs/PLAN.md) — Project plan and phases.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — Architecture and module layout.
- [docs/CONCEPT.md](docs/CONCEPT.md) — Design concepts (Rust-first, safety, speed).
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) — Build, test, benchmark (rustc, cargo, git).
- [docs/NEXT_STEPS.md](docs/NEXT_STEPS.md) — Prioritized roadmap.
- [docs/BENCHMARKS.md](docs/BENCHMARKS.md) — Benchmarks and metrics.
- [docs/SIZING.md](docs/SIZING.md) — n_ctx / n_batch and memory.
- [docs/GITHUB_SETUP.md](docs/GITHUB_SETUP.md) — GitHub repo and push.

## Support the developer

If you find llama.rs useful and want to support its development, you can send **Solana (SOL)** to:

```
GcdgNtdE8NEk3z9sQ5jXv2tqguZjSYqPqNAtjsjPNJx8
```

Thank you.

## License

MIT — see [LICENSE](LICENSE).

## Contributing

See `docs/DEVELOPMENT.md` for build and test instructions. Use `cargo test` and `cargo bench` to verify correctness and ultra-speed.
