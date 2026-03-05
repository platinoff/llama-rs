# Llama-RS Project Plan

## Goal

Define the architecture of a Rust project that works with **llama.cpp** (master), with maximum Rust code for ultra-fast and safe operation, producing a 64-bit `.exe`.

---

## 1. Requirements

| Requirement | Solution |
|-------------|----------|
| Llama source | Folder `../llama.cpp-master` (or `llama.cpp-master/llama.cpp-master` if nested) |
| Language | Maximum Rust, minimum FFI to C/C++ |
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
- [ ] Optional: env or path to local llama.cpp master for custom build (see crate docs).

### Phase 3 — Documentation and architecture

- [ ] `docs/ARCHITECTURE.md` — modules, dependencies, data flow.
- [ ] `docs/CONCEPT.md` — concept (Rust-first, safety, speed).
- [ ] `docs/DEVELOPMENT.md` — how to build, test, benchmark (rustc, cargo).

### Phase 4 — Tests and ultra-speed

- [ ] Unit tests for safe API.
- [ ] Integration tests (with minimal model or mock).
- [ ] `benches/` — benchmarks (e.g. tokens/sec, time to first token).
- [ ] Document results in `docs/` and verify 64-bit exe build.

---

## 4. Tools

- **rustc** — compiler (via `cargo`).
- **cargo** — build, test, bench.
- **git** — version control; for push use **gittoken** (Personal Access Token or credential helper).

---

## 5. Target Platform

- **OS:** Windows (per paths like S:\rust\...).
- **Target:** `x86_64-pc-windows-msvc` for a 64-bit `.exe`.

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
