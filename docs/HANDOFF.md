# Llama-RS Handoff

- **Root**: `S:/rust/llama-rs` (the Cargo project is the repo root).
- **llama.cpp**: vendored upstream via `llama-cpp-sys-2 0.1.154` (globally cached in the Cargo registry, not a checkout in this repo). Model download / update workflow: `docs/LLAMA_UPDATE.md`. The older `0.1.138` (May-2024 backend) could not load >4 GiB GGUFs (`ftell`/`long` = 32-bit on MinGW) nor Qwen3 arch; `0.1.154` fixes both.
- **Status**: Registered in GSV VDT kit (`S:/rust/GSV/docs/gsv/PRODUCTS.md`).
- **State 2026-08-29**: **READY TO TEST** — `target/release/llama_rs.exe` loads `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` and generates (`EXIT=0`); regression scan green (fmt/clippy/10 unit + 5 integration). Next-session ticket work (run via GSV `gsv_tickets_*` MCP, product `llama-rs`):
  1. API E2E: `LLAMA_RS_TEST_MODEL=models/Qwen3.8-27B-UD-IQ2_XXS.gguf cargo test generate_with_model_if_env_set -- --nocapture`
  2. Speed bench: `LLAMA_RS_BENCH_MODEL=models/Qwen3.8-27B-UD-IQ2_XXS.gguf cargo bench --bench speed` (slow on this 5500U; cap with `-- --sample-size 10`); record into `docs/BENCHMARKS.md`
  3. Optional: `cargo test` full, `cargo xtask`-style scan duplication: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`
- **Build**: release binary builds (`target/release/llama_rs.exe`) on the Windows GNU toolchain; env captured in `.cargo/config.toml` (local `.llvm/libclang` for bindgen, forward-slash `-isystem` includes, `_WIN32_WINNT=0x0A00`). CMake 3.31.x (official, not MSYS 4.x) must be on PATH for `llama-cpp-sys-2`; four registry `build.rs` patches are required on Windows-GNU for `0.1.154` (`.a` lib discovery, `cpp-httplib` static link, `llama-common-base` link for build-info symbols, `advapi32`/`shell32`/`ws2_32` import libs). Undone registry patches ⇒ keep the patched copies in the Cargo registry local.
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test` (all green as of 2026-08-29: 10 lib unit + 5 integration).
- **Inference**: verified working with `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` (mmap load, `EXIT=0`, generates tokens; model is a Gated-Delta-Net hybrid requiring `0.1.154`+). Low-RAM machine OK thanks to `CPU_Mapped` (no full-resident weights).
- **Docs**: `docs/ROADMAP.md`, `docs/PLAN.md`, `docs/BENCHMARKS.md`.