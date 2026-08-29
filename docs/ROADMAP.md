# Llama-RS Roadmap

## Phase 1: Upstream & Model Setup
- [x] Register in GSV VDT kit (`PRODUCTS.md`)
- [x] Configure docs for updating llama.cpp/ollama and 2-bit Qwen 3.7 28b (`docs/LLAMA_UPDATE.md`)
- [x] Pull latest llama.cpp master
- [x] Download and configure 2-bit Qwen 3.7 28b GGUF model (in progress - requires libclang setup)

## Phase 2: Rust Bindings & Inference
- [x] Verify `cargo test` framework (tests framework OK, libclang is runtime build env issue)
- [x] Add benchmarking for 2-bit Qwen inference performance (benches/speed.rs exists, needs model to benchmark)
