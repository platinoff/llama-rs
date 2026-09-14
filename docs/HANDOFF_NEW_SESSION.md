# Llama-RS — HANDOFF (new session)

Canon journal continues in [`HANDOFF.md`](./HANDOFF.md) — this file is the
kit-canon entry point (`gsv_products_scan` / GSV `rules-check` registry gate).

- **Root**: `S:/rust/llama-rs` (Cargo project = repo root).
- **Backend**: `llama-cpp-sys-2 0.1.154` (vendored upstream; see
  [`LLAMA_UPDATE.md`](./LLAMA_UPDATE.md)); four registry `build.rs` patches + one
  MTP `common/log.cpp` edit must survive registry re-extraction — patched copies
  stay in the local Cargo registry.
- **State (2026-09-10)**: MTP Phase 4/5/6 green (`MtpSession`, `llama_speed --draft
  models/mtp-Qwen3.8-27B-Q8_0.gguf` exit 0); RAM preflight + `cpu_low_ram` preset
  landed (Phase 2 of [`PERFORMANCE_RESEARCH.md`](./PERFORMANCE_RESEARCH.md));
  heartbeat (`LLAMA_RS_HEARTBEAT=1` → `target/live/llama_heartbeat.json`) feeds GSV
  keep-live band 225.
- **Model**: `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` (GDN hybrid, mmap-only on the
  7.4 GiB box). Bench numbers: [`BENCHMARKS.md`](./BENCHMARKS.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`
  (or `cargo xtask check`); ratio via `cargo xtask loc`.
- **State (2026-09-13)**: `llama_serve` (OpenAI-compat `:8080`, hidden
  autostart via `cargo xtask serve-install`, `--log-file`, MinGW DLL staging)
  is UP with the 27B; GSV keep-live `llama_rs.alive=true`. Swarm path landed
  in Rust: `rpc` cargo feature + `src/safe/rpc.rs` (`--rpc host:port`,
  `/v1/models.rpc_workers`); GGML_RPC=ON links clean with vendored
  `ggml-rpc` sources at proto-5.0.0 pin `f5b9bd39` (6th registry addition).
  Plan: Pi 4 `ggml-rpc-server` at the same pin → A54 Termux (docs/DISTRIBUTED.md).
- **State (2026-09-14)**: ticket drain — chat-template safe API (`src/safe/chat.rs`,
  c450159) + crates.io prep (examples + `cargo package` clean, 57f558b). Board:
  bump `llama-cpp-2 -> 0.1.156` left **open** (re-apply 6 registry patches + full
  rebuild + serve downtime); swarm band stays blocked on Pi 4 / A54 hardware.
