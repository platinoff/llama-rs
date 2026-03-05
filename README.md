# llama.rs · Llama in Rust

**llama.rs** is a **Rust-native** implementation of Llama inference: API, orchestration, and control flow are written in Rust. The compute backend is [llama.cpp](https://github.com/ggml-org/llama.cpp) (via the `llama-cpp-2` crate), but **this codebase is 100% Rust** — no C/C++ in the repo.

## Why Rust all the way?

- **llama.rs, not llama.cpp** — All code you see here is **Rust**. The inference loop (tokenize → decode → sample → accept) lives in `src/safe/`; only the heavy math runs in the linked llama.cpp backend.
- **Zero-cost abstractions** — Thin wrappers (Backend, Model, Context, generate) add no extra allocation on the hot path.
- **64-bit native** — Release builds produce a single `llama_rs.exe` (x86_64-pc-windows-msvc).
- **Safe by default** — Idiomatic `Result` and `Error`; no `unsafe` in this repository.

## Quick start

**1. Clone the repo**

```bash
git clone https://github.com/platinoff/llama-rs.git
cd llama-rs
```

(If you cloned to a different folder, use that path instead of `llama-rs` below.)

**2. Prerequisites (one-time)**

- **Rust** — [rustup](https://rustup.rs), then e.g. `rustup default stable-x86_64-pc-windows-msvc` for 64-bit Windows. Required to compile llama.rs and run the build via `cargo`.
- **Visual Studio Build Tools** — install the **"Desktop development with C++"** workload: provides `link.exe` and the MSVC environment; without it Rust cannot build the Windows binary.
- **Clang** — in VS Installer, add **"C++ Clang tools for Windows"**. The `llama-cpp-2` crate uses `libclang.dll` to parse C/C++ headers; without this the build will fail with an error about `LIBCLANG_PATH`.  
  Details: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#prerequisites).

**3. Build**

On **Windows** you need the MSVC environment and `LIBCLANG_PATH`. In PowerShell (adjust the Clang path if your VS version differs):

```powershell
cd llama-rs
$env:LIBCLANG_PATH = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin"
cmd /c "`"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat`" -arch=amd64 && cd /d %CD% && cargo build --release"
```

**What this does:** `LIBCLANG_PATH` tells the build where to find `libclang.dll`. `VsDevCmd.bat` sets up PATH and other variables for the MSVC linker. `cargo build --release` compiles the project in release mode; the result is `target\release\llama_rs.exe`.

Or open **"x64 Native Tools Command Prompt for VS"** from the Start menu and run:

```cmd
set "LIBCLANG_PATH=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin"
cd path\to\llama-rs
cargo build --release
```

**4. Run the binary**

```bash
.\target\release\llama_rs.exe
# with a GGUF model:
.\target\release\llama_rs.exe path\to\model.gguf "Your prompt"
```

Running with no arguments prints the greeting; with a model path it loads the model and generates text. More: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Cargo commands

Use these from the project root (on Windows, use the same environment as in step 3 above: `LIBCLANG_PATH` set and MSVC via VsDevCmd or **x64 Native Tools Command Prompt**).

| Command | Description |
|--------|--------------|
| `cargo build --release` | Build the release binary → `target/release/llama_rs.exe` |
| `cargo test` | Run unit tests (in `src/`) and integration tests (in `tests/*.rs`) |
| `cargo bench` | Run benchmarks (in `benches/`, release mode) |
| `.\target\release\llama_rs.exe` | Run the compiled CLI (no args = greeting) |
| `.\target\release\llama_rs.exe <model.gguf> "prompt"` | Run inference with a GGUF model |

**Tests** live in the `tests/` folder as `*.rs` files (e.g. `tests/integration.rs`). Run them with `cargo test`. You can also smoke-test the built exe manually: run `.\target\release\llama_rs.exe` to see the greeting, or pass a model path and prompt for a quick CLI check.

## Requirements

- **Rust** 1.70+ (e.g. `rustup default stable-x86_64-pc-windows-msvc` on Windows).
- **Windows (MSVC):** Build Tools with "Desktop development with C++" and "C++ Clang tools for Windows"; set `LIBCLANG_PATH` to the Clang `bin` folder when building.
- **Backend:** llama.cpp is built automatically by the `llama-cpp-2` dependency (no separate clone). See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for details.

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
