# llama-rs — FUNCTION_MANAGEMENT (симбіоз)

**Companion:** `S:/rust/GSV/docs/gsv/GSV_FUNCTION_MANAGEMENT.md` · [`GSV_ECOSYSTEM_SYMBIOSIS.md`](../../GSV/docs/gsv/GSV_ECOSYSTEM_SYMBIOSIS.md) · [`GSV_AGI_PATH.md`](../../GSV/docs/gsv/GSV_AGI_PATH.md)  
**Шар:** plugin (engine) · **Repo:** `https://github.com/platinoff/llama-rs` · **Branch:** `master`

## 1. Оголошені функції

| Функція | Модулі/шляхи | Статус | Примітка |
|---------|--------------|--------|----------|
| llama.cpp-backed Rust inference API | `src/lib.rs`, `src/*` | active | Rust-native API; staged disk→RAM patterns |
| `llama_serve` / edge worker binaries | `src/bin/llama_serve.rs`, `src/bin/llama_edge.rs`, `llama_agent/` | active | `bunke-rock` local lane provider in GSV omni catalog |
| GGUF model serving (local) | `models/` (local data), serve config | active | Heartbeat/file-gated in GSV omni; weights not committed |
| Distributed/RPC/Vulkan feature spikes | Cargo features `rpc`, `vulkan` | experimental/feature-gated | Owner/hardware gated; do not assume default |
| Bench/xtask/build scripts | `scripts/`, `xtask/`, benches | active | MSYS2 bash; Rust-first |

## 2. Звірка / gates

- Standard Rust gates as applicable: `cargo fmt --all -- --check`, `cargo clippy ...`, `cargo test`/targeted checks (use CI/appropriate scope; heavy builds on constrained box with care).
- Pi 4 build/run evidence tracked in mesh-node/llama-rs docs (see `PI_LLAMA_BUILD.md` in mesh-node, llama-rs handoff/docs).
- Model files in `models/` are local assets; never commit.

## 3. Дублі / межі (посилання на master §3)

- **D1** OpenAI-compat/inference serve: intentional parallel engine plugin (with rusty-engine). GSV does not host inference.
- **D7** Rolling logging: intentional per-repo.
- llama-rs provides engine/runtime; GSV provides omni catalog routing/heartbeat gates only.

## 4. Owner-рішення / наступні кроки

- ECO-8.2 sys bump remains separate tracked work (do not mix with unrelated feature work).
- Keep local model layout aligned with GSV omni catalog; no secrets/weights in git.
- Update skeleton when feature flags/surfaces become default or deprecated.
