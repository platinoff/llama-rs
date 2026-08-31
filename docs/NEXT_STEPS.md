# Next Steps — Development Priorities (Rust Architect View)

**Principles:** **100% Rust**, **ultra-fast**, **GSV-live compatible**. No smaller GGUF — we run 27B via mmap on 5500U; focus is staged RAM control + ratio/speed. No Python/Java, shell → `cargo xtask` where logical. Backend is `llama-cpp-2` only (vendored `llama-cpp-sys-2 0.1.154`).

Prioritized roadmap after Phase 1–4: **P0 staged loading → P1 rustification/GSV → P2 ratio/speed** (smaller model skipped).

---

## P0 — Staged model loading (disk → RAM ступенями) — NEW

| # | Step | Why |
|---|------|-----|
| 1 | **StagedLoad API** | `src/safe/staged.rs` + `Model::load_staged(backend, path, StagedLoadOptions { use_mmap, use_mlock, on_progress })` — wraps `LlamaModelParams::with_progress_callback` (0.0..1.0, abort). Pure Rust orchestration. Stages: `Mmap` (file on disk, paged), `Mlock` (pin), `NoAlloc` (defer). Benchmark vs RAM in `SIZING.md`. |
| 2 | **Progress + abort** | Callback `FnMut(f32) -> bool` reports per-stage; `false` aborts load. CLI `--progress` prints; optional GSV live push. |
| 3 | **Tests** | Unit: default options, progress callback round-trip; Integration: `LLAMA_RS_TEST_MODEL` with staged vs mmap. |

**Outcome:** Controlled RAM use without smaller model; 27B stays mmap, optional mlock/prefault.

---

## P1 — Rustification & GSV live (pure Rust, no smaller model)

| # | Step | Why |
|---|------|-----|
| 4 | **Ratio ≥96%** | `gsv-loc-audit --stretch-96` currently 99.46%; keep. Replace shell/YAML with `cargo xtask` (Rust) where logical; no Python/Java. Already 100% Rust crate (no `build.rs`). |
| 5 | **GSV live compat** | `PRODUCTS.md` lists `llama-rs`; `abrakadabra` drain works. Optional `GSV_LIVE=1` reports staged progress to `http://127.0.0.1:9999` (thin Rust glue, no extra daemon). |
| 6 | **Speed** | Keep `cargo bench --bench speed` for `time_to_first_token` / `tokens_per_sec`; zero-copy paths. |

**Outcome:** Pure-Rust, GSV-compatible, ultra-speed.

---

## P2 — Prior done (reference)

| # | Step | Status |
|---|------|--------|
| 7 | ~~Builder, Typed params, Streaming~~ | Done |
| 8 | ~~TTF bench, Metrics, SIZING~~ | Done |
| 9 | ~~Stop sequences, CLI flags, Embeddings~~ | Done |
| 10 | ~~No local llama.cpp (skipped) / deprecation fix~~ | Done/Skipped |

**Outcome:** Feature-complete CLI + embeddings; stable base.

---

## Summary order (new)

```
P0: StagedLoad API → progress+abort → tests (disk→RAM stages)
P1: Ratio ≥96% → GSV live compat → speed benches
P2: (prior) builders, streaming, TTF, metrics, SIZING — Done
```

Smaller model / GPU bench removed — we stay 27B mmap with staged RAM control.

---

## What to do next (100% Rust, ultra-fast, GSV-live)

1. **Maintenance**
    - Bump `llama-cpp-2`/`llama-cpp-sys-2` (now 0.1.154); keep 4 Windows-GNU `build.rs` patches in registry.
    - Keep CI green (Windows GNU, `LIBCLANG_PATH` + `CMAKE` pin, `stable-x86_64-pc-windows-gnu`), `cargo fmt/clippy/test` + `gsv-loc-audit --stretch-96`.

2. **Staged loading follow-ups**
    - `SIZING.md` table: `mmap` vs `mlock` vs `no_alloc` vs RAM.
    - `BENCHMARKS.md` staged timings (current: mmap 0.031 tok/s, 248s TTF on 5500U).
    - Optional: `load_staged` → GSV vision push (`GSV_LIVE` env).

3. **Polish (still 100% Rust)**
    - `ContextParams` presets `low_memory()` / `max_speed()`; chat/tool templates if upstream adds them; embedding norm.
    - Publish to crates.io if desired; keep `abrakadabra` ticket flow (one commit per drain).

