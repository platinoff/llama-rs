# Llama-RS Roadmap

## Phase 1: Upstream & Model Setup
- [x] Register in GSV VDT kit (`PRODUCTS.md`)
- [x] Configure docs for updating llama.cpp/ollama and 2-bit Qwen 3.7 28b (`docs/LLAMA_UPDATE.md`)
- [x] Pull latest llama.cpp (vendored via `llama-cpp-sys-2 0.1.138`)
- [x] Download and configure 2-bit Qwen 3.7 28b GGUF model (`models/Qwen3.8-27B-UD-IQ2_XXS.gguf`; libclang setup resolved in `.llvm/`)

## Phase 2: Rust Bindings & Inference
- [x] Verify `cargo test` framework (tests framework OK; full scan green 2026-08-29)
- [x] Add benchmarking for 2-bit Qwen inference performance (`benches/speed.rs`; needs model-load support to run)

## Phase 3: Reliable Windows Build
- [x] Windows GNU release build (`target/release/llama_rs.exe`, smoke-tested `--help`, 2026-08-29)
- [x] `cargo fmt -- --check` + `cargo clippy --all-targets` + `cargo test` green (2 clippy warnings fixed)
- [x] Build env captured in `.cargo/config.toml` (+ registry `build.rs` patches for Windows-GNU: `.a` libs, `cpp-httplib`, `advapi32`)

## Next
- [ ] Runtime inference against a supported GGUF (2024-era vendored llama.cpp cannot load Qwen3 arch; needs a newer backend or a compatible model)
- [ ] `benches/speed.rs`: run + record benchmark once a model loads