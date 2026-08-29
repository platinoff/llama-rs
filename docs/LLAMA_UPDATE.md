# Llama.cpp / Ollama & Qwen 3.7 28b Update Guide

## 1. Updating Llama.cpp (Master)

To update the embedded/submodule `llama.cpp` upstream to the latest master version:

```bash
cd /s/rust/llama-rs/llama.cpp-master/llama.cpp-master
git fetch origin
git checkout master
git pull origin master
```

Then rebuild or re-link the Rust bindings (`llama-cpp-2` / `llama_rs`).

## 2. Qwen 3.7 28b (2-bit GGUF) Setup

1. **Model Source**: Hugging Face (e.g. `Qwen/Qwen2.5-28B-Instruct-GGUF` or Qwen 3.7 28b variants).
2. **Quantization**: 2-bit GGUF quantization (`Q2_K`, `IQ2_XXS`, or `IQ2_XS`) for high compression and high-speed local inference on consumer hardware.
3. **Storage path**: Place the `.gguf` model file in `S:/rust/llama-rs/llama-rs-project/models/` (or update `config.toml` / CLI args to point to the model path).
4. **Download instruction (Python / huggingface-cli)**:
   ```bash
   huggingface-cli download Qwen/Qwen2.5-28B-Instruct-GGUF --include "*q2_k*.gguf" --local-dir models/
   ```
