# Llama-RS Development Guide

Step-by-step guide for Rust developers: build, test, benchmarks, git.

## Prerequisites

- **Rust**: installed via [rustup](https://rustup.rs).
- **64-bit Windows target**:
  ```bash
  rustup default stable-x86_64-pc-windows-msvc
  ```
  Building the MSVC target requires **Build Tools for Visual Studio** (the "Desktop development with C++" component); otherwise you will see `link.exe not found`. Alternative: GNU target — `rustup default stable-x86_64-pc-windows-gnu` (requires MinGW-w64).
- **llama.cpp**: clone or copy of the master branch in a sibling folder (e.g. `../llama.cpp-master`). The exact path is configured in `build.rs` or via an environment variable.

## Build

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

Architecture and plan documentation: [PLAN.md](PLAN.md), [ARCHITECTURE.md](ARCHITECTURE.md), [CONCEPT.md](CONCEPT.md).
