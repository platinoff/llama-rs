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

## Phase 4: Runtime Inference
- [x] Backend upgraded `llama-cpp-2` / `llama-cpp-sys-2` `0.1.138` → `0.1.154` (2026-08-29)
      - qwen3 / qwen3moe / qwen35 (Gated Delta Net hybrid) architectures supported
      - Large-file GGUF reads >= 4 GiB fixed upstream (gguf.cpp uses `_ftelli64` on Windows) — the `0.1.138` build failed any >4 GiB model (`failed to read magic`)
      - New registry `build.rs` Windows-GNU patches: link `cpp-httplib` (from `out/build/vendor/cpp-httplib`), link `llama-common-base` for `build-info.cpp` symbols (llama_commit/llama_build_*), link `advapi32`/`shell32`/`ws2_32` (upstream only links advapi32 for MSVC)
- [x] `llama_rs.exe models/Qwen3.8-27B-UD-IQ2_XXS.gguf "Hello" --max-tokens 8 --temperature 0 --seed 1` — loads via mmap (CPU_Mapped 6918.98 MiB, no 8 GiB RAM needed) and generates: `, I'm a 17-year`, `EXIT=0` (2026-08-29)

## Next
- [x] `benches/speed.rs`: run + record benchmark on the now-loadable Qwen3.8-27B model (2026-08-30: 0.031 tok/s, 248 s TTF, 5500U mmap; `docs/BENCHMARKS.md`) + `Context::reset()` fix for hybrid M-RoPE

## Next (bench follow-up)
- [ ] Smaller model / GPU bench (27B mmap throttles ~0.6 GiB free; needs 7B or GPU for tok/s)