# Changelog

All notable changes to `llama_rs` (pure Rust, `llama-cpp-2` backend).

## [Unreleased]

### Added
- **Fast bench bin** (`src/bin/llama_speed.rs`, `Cargo.toml:27` `required-features = ["metrics"]`): single-pass pp/tg/TTFT llama-bench-style measurement via `generate_with_metrics` + `InferenceMetrics::to_json`-compatible JSON line; minutes instead of criterion's multi-hour `--sample-size >= 10` runs (`docs/PERFORMANCE_RESEARCH.md` Phase 1)
- **RAM preflight** (`src/safe/preflight.rs`, Phase 2): `advise(free_mib, model_mib, &staged)` → `PreferResident` when mmap & free ≥ model + 1024 MiB headroom, `KeepMmap{deficit_mib}` when free < model (refault risk), `FallbackToMmap` when resident requested but doesn't fit; `low_ram_staged()` picks `resident` vs `mmap`; `warnings()` → human text. Free RAM via `sysinfo` 0.38 (safe), model size via `fs::metadata`. `llama_speed --skip-preflight` to silence stderr warnings (9 unit tests)
- **Preset** `context_presets::cpu_low_ram()` (`src/safe/context.rs`): `n_ctx=2048, n_batch=512` for < 8 GiB boxes, paired with preflight load-mode selection (`docs/SIZING.md`)
- **Research doc** (`docs/PERFORMANCE_RESEARCH.md`): root-cause analysis of the 0.03 tok/s vs ~1 tok/s gap (7.4 GB box memory thrash + mmap refault, llama.cpp #27840/#24037), verified llama-rs/exposed knobs, Rust plan Phases 1–3; Phase 2 marked implemented

### Fixed
- RAM facts corrected to **7.4 GB** total (was “16 GB”) in `docs/BENCHMARKS.md:26` and `docs/SIZING.md:26,37`; mmap thrashes with apps open
- Stale “16 GiB box” wording in `src/safe/staged.rs` module doc

### Verified
- `llama_speed` baseline (2026-09-09, release, mmap, apps open): **tg 0.045 tok/s, TTFT 16.4 s, one pass ≈12 min** vs criterion's ~10 h estimate for the same 27B (`docs/BENCHMARKS.md`)

## [0.1.0] - 2026-08-31

### Added
- **Core**: `Backend`, `Model::load_from_file`, `Context` (`n_ctx`/`n_batch`/`reset` for hybrid M-RoPE), `GenerateOptions` builder, `generate`/`generate_stream` pure Rust loops (`src/safe/*`, `src/lib.rs:25`)
- **Staged loading** (`src/safe/staged.rs:16`): `StagedLoadOptions {use_mmap, use_mlock}` presets `mmap/resident/pinned` + `Model::load_staged` / `load_staged_with_progress` (`progress_callback 0.0..1.0` abort, `load_mode` via `llama-cpp-2` 0.1.154)
- **Presets** (`src/safe/context.rs:196`): `context_presets::low_memory (2048/512)` / `max_speed (4096/2048)` (`docs/SIZING.md:24`)
- **Embeddings** (`src/safe/embed.rs:9`): `l2_norm`, `l2_normalize`, `mean_pool` (always), `embed`/`embed_normalized` (feature `embeddings`)
- **CLI** (`src/main.rs:46`): `--mmap/--no-mmap --mlock --progress`, `GSV_LIVE=1` → `127.0.0.1:9999` `gsv_report_progress` (thin `TcpStream`, `src/main.rs:19`)
- **xtask** (`xtask/src/main.rs:1`, `Cargo.toml:43`): `cargo xtask check|fmt|clippy|test|loc|sizing` pure Rust, `[alias] xtask` (`.cargo/config.toml:8`), no Python/Java
- **Docs**: `PLAN` Phase5/6, `ROADMAP` Phase1–6, `NEXT_STEPS` P0–2, `ARCHITECTURE` staged layer, `SIZING` staged table, `DEVELOPMENT` xtask table

### Fixed
- Hybrid Gated Delta Net / M-RoPE `Context::reset()` (`src/safe/context.rs:62`, `benches/speed.rs:41` `reset()` per iter)
- Windows-GNU build: `CMAKE` pin 3.31.9 + `static-libstdc++` (`.cargo/config.toml:2`), 4 registry `build.rs` patches (`.a` libs, `cpp-httplib`, `llama-common-base`, `advapi32/shell32/ws2_32`)
- `token_to_piece` decoder, `mmap` large-file >4GiB (`0.1.138`→`0.1.154`)

### Verified
- `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test` (16 unit +5 integration +1 doctest via `cargo xtask check`), `cargo xtask loc` `99.46% (43372/43609) stretch-96 meets`
- Bench `benches/speed.rs:38` `0.031 tok/s, 248s TTF` on 5500U mmap 27B IQ2_XXS (`docs/BENCHMARKS.md:24`), E2E `generate_with_model_if_env_set` 156s

### CI
- `.github/workflows/ci.yml:39` Windows GNU `stable-x86_64-pc-windows-gnu`, CMake 3.31.9, `cargo xtask check` + `cargo xtask loc`

### Publish
- `Cargo.toml` `repository`, `keywords`, `categories`, `exclude` for publish prepare; `CHANGELOG.md` added; `cargo publish --dry-run` verified (no breaking, `embed`/`metrics` features).
