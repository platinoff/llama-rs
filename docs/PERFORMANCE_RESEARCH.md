# Performance research: Qwen 27B on the 5500U box (2026-09-09)

Research into why `Qwen3.8-27B-UD-IQ2_XXS.gguf` decodes at **~0.03–0.036 tok/s**
and what Rust-side fixes are worth implementing. Triggered by the aborted
`tg_32` benchmark run (killed by a Windows Update reboot, 2026-09-09 04:00).
Covers root-cause analysis, the levers llama.rs already exposes, and a
concrete Rust plan.

## TL;DR

1. **The CPU is not the bottleneck.** A dense 27B should decode at ~1 tok/s on a
   6-core CPU (reference: i7-8700 dense 30B ≈ 1.16 tok/s). We measured ~0.036
   tok/s — a **~30× gap** — because the machine thrashes: **7.4 GB total RAM**,
   free memory dropped to **0.3–0.6 GiB** during the run (Chrome + OpenCode
   resident), so `mmap` weight pages are evicted to the pagefile and must be
   re-read from disk on every decode step.
2. **llama.rs already exposes all the tuning knobs** (mmap/mlock staged presets,
   `n_batch`/`n_threads`, generation metrics). The real *missing* Rust pieces are
   a **fast single-pass bench bin** and a **RAM preflight warning** — not new
   inference features.
3. **Model path is the strongest lever:** a MoE model (`Qwen3-30B-A3B` IQ2,
   `Qwen3-8B-A3B` Q4) decodes ~5–10× faster than a dense 27B at the same file
   size. `qwen3moe` arch is already supported by the bundled llama.cpp.

## Root cause: memory thrash, not compute

| Quantity | Value | Source |
|---|---|---|
| Machine total RAM | **7.4 GB** | Windows property (≠ “16 GB” in earlier docs) |
| Free RAM at bench peak | 0.3 GB (95.5% used) | Windows perf |
| Measured `tg_32` | ~1020 s/iter → **0.031 tok/s** | `docs/BENCHMARKS.md` (2026-08-30) |
| Estimated `tg_32` sample | 8808.9 s / 10 → **0.036 tok/s** | `target/bench-qwen-2026-09-09-tg.log` (2026-09-09) |
| Theoretical dense 27B, 6-core | ~1.0–1.2 tok/s | i7-8700 dense 30B = 1.16 tok/s (web) |
| 27B GGUF size (mmap) | ~6.8–6.9 GiB | file size |

Why `mmap` hurts under pressure: the model is memory-mapped, so inference relies
on the file pages staying cached (Standby). With 0.3 GiB free, every decode step
refaults weight pages from the pagefile/disk. Two known llama.cpp cases:

- [ggml-org/llama.cpp#27840](https://github.com/ggml-org/llama.cpp/issues/27840)
  — Windows `mmap` causes **constant disk writes** under memory pressure (refault storm).
- [ggml-org/llama.cpp#24037](https://github.com/ggml-org/llama.cpp/discussions/24037)
  — “mmap kills performance”: on a 397B model `--mmap` cut `tg` from 11 → 1 t/s;
  `--no-mmap` restored it. Root cause: mmap page refaults.

Threads/batch are **not** the issue: llama.cpp already auto-tunes threads to
physical cores, `n_batch`/`n_ubatch` were 512, flash-attention was on, and GDN /
DeepSeek fused ops were enabled in the run log.

## What llama.rs already exposes (verified against source)

| Lever | Where | Notes |
|---|---|---|
| mmap on/off | `ModelParams::builder().with_use_mmap(bool)` | llama-cpp-2 `src/model/params.rs:538` |
| mlock on/off | `ModelParams::builder().with_use_mlock(bool)` | llama-cpp-2 `src/model/params.rs:545` |
| Staged load presets `mmap`/`resident`/`pinned` | `StagedLoadOptions` in `src/safe/staged.rs`; `Model::load_staged` | table in `docs/SIZING.md:35` |
| CLI `--no-mmap` / `--mlock` | `src/main.rs:144–153` | already wired |
| `n_ctx` / `n_batch` / `n_ubatch` / `n_threads`(+batch) | `ContextParams::builder()` | llama-cpp-2 `src/context/params/get_set.rs:21/53/83/172/202` |
| Context presets `low_memory()` / `max_speed()` | `context_presets` in `src/safe/context.rs:196` | SIZING.md:24 |
| pp/tg/TTFT metrics + JSON | `metrics` feature → `InferenceMetrics::to_json()` | `src/metrics.rs:5`; `generate_with_metrics` in `src/safe/generate.rs:40` |

So all the *performance* plumbing exists. The problem is the **measurement
harness** (criterion `--sample-size ≥ 10` × 15–17 min/iter = hours, and Windows
Update can kill it) and the **absent guardrails** (no RAM preflight).

## Rust plan (what to implement)

### Phase 1 — fast single-pass bench bin: `src/bin/llama_speed.rs`

llama-bench style, replaces criterion for speed measuring. One process loads the
model once, runs a fixed prompt (default 128/256 tokens) and a fixed generation
count (default 32), then prints `InferenceMetrics::to_json()`.

- Uses the existing `metrics` feature (`generate_with_metrics`), no new inference code.
- Runtime target: **1–2 min** even for a 27B model (one pass, not 10 samples).
- CLI: `--model <path> --gen-tokens 32 --mmap/--no-mmap --mlock --n-ctx --n-batch --threads --json`.
- Fixes the two failures of the current setup: criterion's `sample_size >= 10`
  hard minimum and multi-hour per-run cost.
- **Implemented 2026-09-09** (`src/bin/llama_speed.rs`, `Cargo.toml:27`
  `required-features = ["metrics"]`; `cargo run --bin llama_speed --features metrics -- --model <path> --json`).
  Criterion benches stay (keep `hello_llama_rust` baseline), but
  `tg_32`/`ttft`/`pp_256` move behind this bin.

### Phase 2 — RAM preflight + auto preset

- Before `Model::load`, estimate resident need (`file_size` for `resident`,
  plus KV/buffers) and compare against free RAM; **warn** when free < need
  (advise closing Chrome/OpenCode) and when `mmap` mode is selected with high
  memory pressure (fall back to `resident` if it fits, else refuse).
- Add a `cpu_low_ram` preset: `resident` load + `n_ctx 2048` + `n_batch 512`
  (vs current `mmap` default on 7.4 GB with apps open).
- Optional: set `n_batch 2048` for prefill (`max_speed` preset).

### Phase 3 — model-path recommendation (docs + optional helper)

Dense 27B is the wrong model shape for 7.4 GB. `qwen3moe` arch is supported by
llama-cpp-2 0.1.154's bundled llama.cpp
(`llama.cpp/src/llama-arch.cpp:37`, `models/qwen3moe.cpp`), so llama.rs can load
MoE GGUFs today. Best candidates (decode ~5–10× faster than dense 27B):

| Model | File size class | Expected decode (6-core CPU) |
|---|---|---|
| Qwen3-30B-A3B IQ2/IQ3 | ~8–9 GB (mmap-ok when RAM is free) | ~5–10 tok/s (MoE: only ~3.3B active) |
| Qwen3-8B-A3B Q4_K_M | ~5–6 GB (fits resident) | similar tok/s, lower quality |

## Operational rules (cheap, immediate)

- **Free RAM before any bench/inference:** close Chrome + OpenCode (frees ~1.5+ GiB),
  then re-check free memory before starting.
- Model that is entirely resident (or `pinned` w/ mlock) cannot be refaulted;
  on this box with apps closed, `resident` needs ~7 GB free on top of ~0.5 GB OS/Rust buffers.
- Keep the pagefile on an SSD (HDD pagefile turns the refault storm into a dead stop).
- Prefill (pp) is compute-bound; raising `n_batch` to 2048 reduces its step count —
  independent of the tg problem above.

## Measurement plan (post Phase 1)

| Condition | Model | Expected tg tok/s |
|---|---|---|
| 2026-09-09 baseline (measured, `llama_speed`, release) | Qwen 27B IQ2, mmap, apps open | **0.045** (32 tg, TTFT 16.4 s, one pass ~12 min, `docs/BENCHMARKS.md`) |
| criterion baseline (already have) | Qwen 27B IQ2, mmap, apps open | 0.031 |
| `llama_speed` + RAM free (~2.5 GiB) | Qwen 27B IQ2, mmap | ~0.5–1 (theoretical, untested) |
| `llama_speed` + `resident` (if fits w/ apps closed) | Qwen 27B IQ2 | ~0.5–1 |
| `llama_speed` (benchmark to run) | Qwen3-30B-A3B IQ2 / 8B-A3B Q4 | 5–10 |

## References

- llama.cpp performance tuning (ggml-org docs): CPU-only optimal
  `--threads $(nproc) --batch-size 512 --mlock`; low-RAM advice: Q4/Q3,
  `ctx-size 512`, no mlock.
- MoE-on-CPU benchmark blog: Qwen3-30B-A3B Q4_K_M ≈ **10.6 tok/s** on an
  i7-8700 (≈ speed of dense 3B; dense 30B ≈ 1.16 tok/s) — ~9× faster.
- llama.cpp #27840 (Windows mmap write storm) and #24037 (`--no-mmap` restores tg).
- Prior artifacts: `docs/BENCHMARKS.md`, `target/bench-qwen-2026-09-09-tg.log`.