# llama.rs Project Plan

## Goal

**llama.rs** — a Rust project for Llama inference: **maximum Rust**, minimal FFI. All API and orchestration in Rust; [llama.cpp](https://github.com/ggml-org/llama.cpp) is the linked backend. Result: a 64-bit `.exe` built with cargo.

---

## 1. Requirements

| Requirement | Solution |
|-------------|----------|
| Llama source | Folder `../llama.cpp-master` (or `llama.cpp-master/llama.cpp-master` if nested) |
| Language | Maximum Rust; llama.cpp only as linked backend |
| Safety | Safe Rust API; unsafe code only in a thin bindings layer |
| Speed | Zero-copy where possible, batching, minimal allocations on the inference path |
| Build output | `target/release/llama_rs.exe` (x86_64-pc-windows-msvc) |
| License | MIT |
| VCS | Git-friendly layout, single repository |

---

## 2. Project Structure (Git-friendly)

```
llama-rs-project/
├── .gitignore
├── Cargo.toml              # workspace or lib+bin package
├── LICENSE                  # MIT
├── README.md
├── build.rs                 # build llama.cpp and/or bindings
├── src/
│   ├── lib.rs               # public API (safe Rust)
│   ├── main.rs              # CLI (exe)
│   ├── ffi/                 # low-level bindings to llama.cpp (optional mod)
│   └── safe/                # high-level safe wrappers (optional mod)
├── docs/
│   ├── PLAN.md              # this plan
│   ├── ARCHITECTURE.md      # architecture and diagrams
│   ├── CONCEPT.md           # concept and design decisions
│   └── DEVELOPMENT.md       # guide for Rust developers
├── tests/                   # integration tests
├── benches/                 # speed benchmarks (ultra-speed)
└── llama.cpp-master/        # optional: git submodule pointing at master
```

- Do not commit binaries or build artifacts (`target/`, `llama.cpp/build/`).
- A **git submodule** to the official llama.cpp master can be used instead of a copy in `../llama.cpp-master`.

---

## 3. Implementation Phases

### Phase 1 — Hello Llama Rust (first commit)

- [x] Cargo project setup (lib + bin).
- [x] `docs/PLAN.md` — plan.
- [x] `README.md` — project description as ultra-fast.
- [x] MIT `LICENSE`, `.gitignore`.
- [x] First commit: "hello llama rust"; first push (optional, with gittoken/remote).

### Phase 2 — Integration with master folder

- [x] Use **llama-cpp-2** crate for FFI (builds/links llama.cpp; our code stays 100% Rust).
- [x] Safe wrappers in `src/safe/`: Backend, Model, Context, GenerateOptions, generate.
- [x] Idiomatic Error and Result in `src/error.rs`; all public API in Rust.
- [x] No local llama.cpp build — we keep 100% Rust; backend from `llama-cpp-2` only.

### Phase 3 — Documentation and architecture

- [x] `docs/ARCHITECTURE.md` — modules, dependencies, data flow.
- [x] `docs/CONCEPT.md` — concept (Rust-first, safety, speed).
- [x] `docs/DEVELOPMENT.md` — how to build, test, benchmark (rustc, cargo).

### Phase 4 — Tests and ultra-speed

- [x] Unit tests for safe API (lib.rs, GenerateOptions, Error).
- [x] Integration tests (greeting, GenerateOptions; optional model test via env).
- [x] `benches/` — speed benchmark (hello); inference metrics documented in `docs/BENCHMARKS.md`.
- [x] 64-bit exe build verified (release).

### Phase 5 — Rustification & GSV live (pure Rust, no YAML/TOML shells)

- [ ] **Ratio 95–100% Rust**: keep `gsv-loc-audit --stretch-96` ≥96% (now 99.4%). Replace any needed non-RS file with RS where logical: shell → `cargo xtask` (Rust), config handled in Rust, CI logic in Rust.
- [ ] **GSV live integration**: support `GSV_LIVE=1` / `http://127.0.0.1:9999` — progress / metrics can be reported to GSV `vision` / `speed` endpoints; `abrakadabra` ticket flow compatible (`PRODUCTS.md` already registered). No Python, no extra daemons — thin Rust glue.
- [ ] **Speed**: keep `cargo bench --bench speed` for `time_to_first_token` + `tokens_per_sec`; zero-copy paths, no mid-inference allocs.

### Phase 6 — Staged model loading (disk → RAM ступенями)

- [ ] Research `llama_cpp_2::model::params::LlamaModelParams` flags: `use_mmap` / `use_mlock` / `no_alloc` + `with_progress_callback(|p: f32| -> bool)`.
- [ ] Implement `Model::load_staged` / `StagedLoadOptions` in `src/safe/staged.rs`: stages — `Mmap` (file stays on disk, paged), `Mlock` (pin to RAM), `Prefault` (touch pages), with progress `0.0..1.0` and abort support. Pure Rust orchestration; backend stays `llama-cpp-2`.
- [ ] CLI: `--mmap/--no-mmap --mlock` + `--progress` flag; `generate` already streams.
- [ ] Docs + bench: `docs/SIZING.md` loading modes vs RAM, `docs/BENCHMARKS.md` staged times.

---

## Next steps (prioritized)

See **[NEXT_STEPS.md](NEXT_STEPS.md)** for the Rust-architect roadmap: P0 staged loading → P1 rustification+GSV → P2 ratio/speed. Smaller 7B model not needed — focus is Rust & staged RAM control on existing 27B mmap.

---

## 4. Tools

- **rustc** — compiler (via `cargo`).
- **cargo** — build, test, bench; `cargo xtask` (Rust) replaces shell where possible (MSYS2 bash only when needed).
- **git** — version control; for push use **gittoken** (Personal Access Token or credential helper).
- **GSV live** — `http://127.0.0.1:9999` (optional, for `abrakadabra` / vision).

---

## 5. Target Platform

- **OS:** Windows (MSYS2 bash for `cargo`/`git`; `stable-x86_64-pc-windows-gnu`).
- **Target:** `x86_64-pc-windows-gnu` (GNU; `MSVC` also works if `LIBCLANG_PATH` set) for a 64-bit `.exe`.
- **Model:** `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` (mmap, `CPU_Mapped` 6.9 GiB; staged load via `use_mmap`/`use_mlock`).

Verification:

```bash
rustup default stable-x86_64-pc-windows-msvc
cargo build --release
# Output: target/release/llama_rs.exe (or package name from Cargo.toml)
```

---

## 6. First Commit

- Message: **hello llama rust**
- Contents: plan in `docs/`, README, LICENSE, `.gitignore`, minimal `src/lib.rs` and `src/main.rs` that print the greeting. Then the first `git push` (with gittoken if needed).

This plan is a living document and may be updated in `docs/` as the project evolves.
