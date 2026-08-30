# Llama-RS Handoff

- **Root**: `S:/rust/llama-rs` (the Cargo project is the repo root).
- **llama.cpp**: vendored upstream via `llama-cpp-sys-2 0.1.138` (globally cached in the Cargo registry, not a checkout in this repo). Model download / update workflow: `docs/LLAMA_UPDATE.md`.
- **Status**: Registered in GSV VDT kit (`S:/rust/GSV/docs/gsv/PRODUCTS.md`).
- **Build**: release binary builds (`target/release/llama_rs.exe`) on the Windows GNU toolchain; llava config in `.cargo/config.toml` (local `.llvm/libclang` for bindgen, forward-slash `-isystem` includes, `_WIN32_WINNT=0x0A00`). CMake 3.31.x (official, not MSYS 4.x) must be on PATH for `llama-cpp-sys-2`; three registry `build.rs` patches are required on Windows-GNU (`.a` lib discovery, `cpp-httplib` static link, `advapi32`).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (all green as of 2026-08-29: 10 lib unit + 5 integration).
- **Docs**: `docs/ROADMAP.md`, `docs/PLAN.md`, `docs/BENCHMARKS.md`.