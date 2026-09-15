# Benchmarks (ultra-speed)

## Running benchmarks

From the project root (with [build environment](DEVELOPMENT.md#build) set):

```bash
cargo bench                                  # criterion (hello_llama_rust baseline)
cargo run --bin llama_speed --features metrics -- \
  --model models/Qwen3.8-27B-UD-IQ2_XXS.gguf --json     # fast one-pass pp/tg/TTFT
```

`llama_speed` (`src/bin/llama_speed.rs`, Phase 1 of `PERFORMANCE_RESEARCH.md`) is
the preferred speed tool: one pass, minutes not hours, JSON line for GSV ingest.
Model also resolves from `LLAMA_RS_BENCH_MODEL`. Flags: `--gen-tokens`, `--n-ctx`,
`--n-batch`, `--threads`, `--mmap/--no-mmap`, `--mlock`, `--progress`.

## Current benchmarks (llama-bench style: pp/tg/TTFT)

- **`hello_llama_rust`** — baseline, no model.
- **`pp_256`** (when `LLAMA_RS_BENCH_MODEL` set) — prompt processing / prefill, 256-token prompt, 1 token gen; `pp tok/s = prompt_tokens / prompt_ms`.
- **`tg_32`** (when `LLAMA_RS_BENCH_MODEL` set) — token generation, 32 tokens; `tg tok/s = 32 / eval_s` (memory-bandwidth bound).
- **`ttft`** (when `LLAMA_RS_BENCH_MODEL` set) — time to first token, ms from start until first decoded piece.

All three share one `Backend+Model` (Backend::init once per process). Swap model via env: `LLAMA_RS_BENCH_MODEL=/path/to/Qwen.gguf` locally, `.../Nemotron.gguf` from OpenCode.

## InferenceMetrics (Rust API)

`src/metrics.rs:5` `InferenceMetrics { prompt_tokens, tokens_generated, decode_count, wall_time_ms, prompt_ms, eval_ms, ttft_ms }` + `tokens_per_sec()` / `prompt_tokens_per_sec()` / `to_json()` for GSV ingest. Collected via `generate_with_metrics` / `generate_stream_with_metrics` (`metrics` feature, `src/lib.rs:21`). `src/safe/context.rs:236` fills `prompt_ms` (prefill), `ttft_ms` (first chunk), `eval_ms = wall - prompt`.

## Results (2026-08-30 — Qwen, 2026-09-01 — expanded metrics)

Hardware: AMD Ryzen 5 5500U (6c/12t), **7.4 GB RAM** (≈0.3–0.6 GiB free during
run), Windows 10, release profile, `n_ctx_default = 512`. See
[PERFORMANCE_RESEARCH.md](PERFORMANCE_RESEARCH.md) for the root-cause analysis
of the ~30× gap vs theoretical (~1 tok/s for dense 27B) and the Rust plan.

Model Qwen: `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` (27B Gated Delta Net / M-RoPE,
threads = physical cores, `use_mmap true`).

Run: `LLAMA_RS_BENCH_MODEL=models/Qwen3.8-27B-UD-IQ2_XXS.gguf cargo bench
--bench speed -- --sample-size 10`.

| Benchmark | Median | Notes |
|---|---|---|
| `hello_llama_rust` | 1.01 ns (now) / 1.53 ns (2026-08-30) | baseline; no model load |
| `tg_32` (ex `inference_tokens_per_sec`) | 1020.1 s / iter | 32-token generation; ≈ 0.031 tok/s |
| `ttft` (ex `time_to_first_token`) | 248.25 s | first decoded token after prefill |
| `pp_256` | — | new in this bench (prefill 256 tokens) |

> `tg_32` ≈ 32 / 1020.1 s ≈ **0.031 tokens/s** on this
> 27B IQ2_XXS model with mmap on a 5500U. Expect seconds-scale numbers only
> with a smaller / quantized model or a GPU. Memory pressure (≈0.6 GiB free)
> throttles the run heavily. For Nemotron 30B-A3B expect similar or slower CPU numbers; on GPU `tg` scales with VRAM bandwidth (see web research: RTX 4090 ~125 tok/s for 7B Q4).

### Results (2026-09-09 — `llama_speed` baseline, one pass)

Run: `cargo run --release --bin llama_speed --features metrics -- \
  --model models/Qwen3.8-27B-UD-IQ2_XXS.gguf --gen-tokens 32 --json`
(start free RAM 3.0 GiB → thrashed to 0.23 GiB). ~12 min one-pass, the same
order as the multi-hour criterion run (see `PERFORMANCE_RESEARCH.md`).

```json
{"pp_tokens":1,"pp_tokens_per_sec":0.061,"tg_tokens":32,"tg_tokens_per_sec":0.045,"ttft_ms":16402,"prompt_ms":16394,"eval_ms":709495,"wall_time_ms":725889,"decode_count":33,"model":"models/Qwen3.8-27B-UD-IQ2_XXS.gguf","use_mmap":true,"use_mlock":false}
```

| Benchmark | Value | Notes |
|---|---|---|
| `tg` (decode) | **0.045 tok/s** (32 tokens / 709.5 s) | same thrash order as criterion 0.031–0.036; ~20–25× below the ~1 tok/s dense-27B theoretical |
| `ttft` | **16.4 s** | prompt was 1 token |
| `pp` | n/a (1-token prompt) | use a real prompt text for prefill numbers |
| wall | 725.9 s | ≈ **12 min one pass** vs criterion's ~10 h estimate |

### Results (2026-09-14 — interactive tier, `llama_serve` :8082)

Serve: release `llama_serve` + `models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf`
(`--model-name lama-1.5`, mmap) beside the 27B on `:8080`. Chat
`max_tokens=8`: **8 tokens in 3.9 s ≈ 2 tok/s**, TTF seconds — ~65× the
27B mmap rate (0.031 tok/s, TTF 248 s). This is the fast tier for the
chat loop; 27B stays the deep tier (see `docs/DISTRIBUTED.md` tiers).

### Results (2026-09-14 — chat loop laptop↔A54 via channels)

Full loop Mini App/bot → poolAI task → `llama_edge` → `llama_serve` →
answer back (`max_tokens=8`, same prompt):

| Tier | Wall E2E | Notes |
|---|---|---|
| Fast (`lama-1.5` :8082) | **15.3 s** | poll-granularity bound; inference itself ~4 s |
| Deep (`lama-2.8` :8080) | **270.8 s** | ~0.03 tok/s effective, matches direct-serve baseline |

Phone needs nothing but Telegram (initData-gated enqueue).

### Results (2026-09-15 — MoE path: `Qwen3-30B-A3B-UD-IQ2_XXS`, `llama_speed` one pass)

Run: `./target/release/llama_speed.exe --model models/Qwen3-30B-A3B-UD-IQ2_XXS.gguf
--prompt "<1440 chars / 321 tok>" --gen-tokens 32 --n-ctx 512 --mmap --json`
(deep `:8080` stopped for the pass; fast `:8082` stayed live — service-first,
not a fully quiet box). File 10 362 262 080 B (≈9.65 GiB) > 7.4 GiB RAM.

```json
{"pp_tokens":321,"pp_tokens_per_sec":1.075,"tg_tokens":32,"tg_tokens_per_sec":0.497,"ttft_ms":298503,"prompt_ms":298493,"eval_ms":64413,"wall_time_ms":362906,"decode_count":33,"model":"models/Qwen3-30B-A3B-UD-IQ2_XXS.gguf","use_mmap":true,"use_mlock":false}
```

| Benchmark | Value | Notes |
|---|---|---|
| `tg` (decode) | **0.497 tok/s** (32 tok / 64.4 s) | **≈11–16× the dense-27B baseline** (0.031–0.045); MoE active 3.3B helps exactly as predicted, but the 9.65 GiB file still page-thrashes on 7.4 GiB RAM (expert slices re-read per token) |
| `pp` | **1.075 tok/s** (321 tok / 298.5 s) | cold first-touch: prefill streams most of the 9.65 GiB from disk — disk-bound, not compute |
| `ttft` | **298.5 s** | same cold page-in; a warm second pass would drop sharply |
| wall | 362.9 s | one pass ≈ 6 min |

Verdict: MoE-at-2-bit is the right lane (huge win vs dense 27B), yet the
2–6 tok/s research expectation needs either ~10 GiB RAM (resident experts)
or a smaller-than-RAM quant (IQ1_S ≈ 5.8 GiB / Q2_K ~7 GiB class) to stop
thrashing; no 8-bit MTP draft is required for this path.

### Results (2026-09-15 — Wave A: `UD-IQ1_S` aborts, 1-bit is NOT viable here)

`models/Qwen3-30B-A3B-UD-IQ1_S.gguf` (9 043 300 928 B) loads fine (metadata
OK) but every generate path dies at the ggml CPU guard:

```
Assertion failed: !isnan(x), file .../llama-cpp-sys-2-0.1.154/llama.cpp/ggml/src/ggml-cpu/ops.cpp, line 3234
```

reproduced on **both** 0.1.154 and 0.1.156 (`llama_speed` cold/warm + 4-prompt
coherence grid via `llama_rs.exe`, temp 0) → not a bump regression, not a file
corruption (resume-race excluded: same assert from a clean .154 binary):
IQ1_S + `qwen3moe` emits NaN activations on CPU dequant. 1-bit is out for this
stack; the resident lane needs IQ2_XS-class (≈8.2 GiB, still > RAM) or more RAM
/ a second ggml-rpc x86 host (Wave B). Interim posture: deep = IQ2_XXS async
(0.497), interactive = `:8082` fast tier.

## Verification

Qwen locally (default):
```cmd
set LLAMA_RS_BENCH_MODEL=S:\rust\llama-rs\models\Qwen3.8-27B-UD-IQ2_XXS.gguf
cargo bench --bench speed
```
Nemotron via OpenCode (swap, Qwen stays on disk):
```cmd
set LLAMA_RS_BENCH_MODEL=S:\path\to\Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf
cargo bench --bench speed -- tg_32
cargo test --features metrics -- --nocapture  # prints InferenceMetrics::to_json()
```
Metrics JSON (GSV ingest):
```rust
let (out, m) = generate_with_metrics(&model, &mut ctx, "Hi", &opts)?;
println!("{}", m.to_json()); // -> {"tokens_generated":32,"prompt_tokens":2,"ttft_ms":248000,...}
```

- Release build is 64-bit: `target\release\llama_rs.exe` (x86_64-pc-windows-msvc).
- For consistent numbers, use `cargo bench` with `--release` (default for bench) and close other heavy applications.

See [SIZING.md](SIZING.md) for `n_ctx` / `n_batch` and memory vs throughput.
