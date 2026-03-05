# Llama-RS Development Guide

Step-by-step guide for Rust developers: build, test, benchmarks, git.

## Prerequisites

- **Rust**: installed via [rustup](https://rustup.rs).
- **64-bit Windows target**:
  ```bash
  rustup default stable-x86_64-pc-windows-msvc
  ```
  Building the MSVC target requires **Build Tools for Visual Studio** with the **"Desktop development with C++"** workload (provides `link.exe`).
- **Clang / libclang**: the `llama-cpp-2` dependency uses **bindgen** at build time and needs **libclang**. Either:
  - **Option A:** In Visual Studio Installer → Modify your Build Tools → **Individual components** → search **"Clang"** and install **"C++ Clang tools for Windows"** (or **"C++ Clang compiler for Windows"**), then ensure the Clang `bin` (containing `libclang.dll`) is on `PATH` or set `LIBCLANG_PATH` to that `bin` folder.
  - **Option B:** Install [LLVM](https://releases.llvm.org/) (Windows installer) and set `LIBCLANG_PATH` to the LLVM `bin` directory (e.g. `C:\Program Files\LLVM\bin`).
- **llama.cpp**: built automatically by the `llama-cpp-2` crate from its bundled source; no separate clone required.

## Build

**Windows (MSVC):** Run the build from a **Developer Command Prompt** so that `link.exe` and the C++ toolchain are on `PATH`. Set `LIBCLANG_PATH` so bindgen finds libclang (e.g. after installing "C++ Clang tools for Windows"):

```powershell
$env:LIBCLANG_PATH = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin"
cmd /c "`"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat`" -arch=amd64 && cd /d s:\rust\llama-rs\llama-rs-project && cargo build --release"
```

Or open **"x64 Native Tools Command Prompt for VS 2026"** from the Start menu, then:

```cmd
set LIBCLANG_PATH=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin
cd /d s:\rust\llama-rs\llama-rs-project
cargo build --release
```

Plain build (if your shell already has the VS environment):

```bash
cd llama-rs-project
cargo build
```

Release (optimized 64-bit exe):

```bash
cargo build --release
```

Artifact: `target\release\llama_rs.exe`.

## Tests

```bash
cargo test
```

- Unit tests in `src/lib.rs`.
- Integration tests in `tests/`.

## Benchmarks (ultra-speed)

```bash
cargo bench
```

Benchmarks are defined in `benches/`. After integrating with llama.cpp you can add metrics such as tokens/sec and time to first token.

## Git and first commit

1. Initialize (if not already done):
   ```bash
   cd llama-rs-project
   git init
   ```

2. Stage and create the first commit:
   ```bash
   git add .
   git commit -m "hello llama rust"
   ```

3. Remote repository and push (with gittoken):
   - Create a repo on GitHub/GitLab etc.
   - Add remote:
     ```bash
     git remote add origin https://github.com/YOUR_USER/llama-rs-project.git
     ```
   - Use a **Personal Access Token** (gittoken) instead of a password. Example with token in URL:
     ```bash
     git remote set-url origin https://YOUR_TOKEN@github.com/YOUR_USER/llama-rs-project.git
     git push -u origin master
     ```
     Or enter the token as the password when running `git push` if using a credential manager.
   - If your default branch is `main`: run `git branch -M main` then `git push -u origin main`.

## Useful commands

| Action      | Command                 |
|------------|-------------------------|
| Build      | `cargo build --release` |
| Test       | `cargo test`            |
| Benchmarks | `cargo bench`           |
| Check      | `cargo check`          |
| Lint       | `cargo clippy`          |
| Format     | `cargo fmt`             |

See [BENCHMARKS.md](BENCHMARKS.md) for speed benchmarks and inference metrics.

## API usage example

```rust
use llama_rs::{Backend, Model, Context, GenerateOptions, generate};
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::context::params::LlamaContextParams;
use std::path::Path;

let backend = Backend::init()?;
let model = Model::load_from_file(&backend, Path::new("model.gguf"), &LlamaModelParams::default())?;
let mut context = model.new_context(&backend, LlamaContextParams::default())?;
let out = generate(&model, &mut context, "Hello", &GenerateOptions::default())?;
```

Architecture and plan documentation: [PLAN.md](PLAN.md), [ARCHITECTURE.md](ARCHITECTURE.md), [CONCEPT.md](CONCEPT.md).
