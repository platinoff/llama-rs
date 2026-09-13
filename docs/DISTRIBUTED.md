# Distributed inference (swarm) — Rust-only plan

Coordinator: `llama_serve` on the PC (BunkeRock, `http://127.0.0.1:8080/v1`,
GSV hub routes `provider=bunke-rock` here). Workers hold model shards.

## Mechanism: ggml RPC (tensor offload, not HTTP)

`llama.cpp/tools/rpc/README.md`: each worker runs `ggml-rpc-server`, the
coordinator registers it and spreads weights + KV cache over local + remote
devices proportionally to free memory. An HTTP task split cannot parallelize
autoregressive decode — offload must go through ggml RPC.

- LAN only. RPC is plaintext; upstream marks it proof-of-concept.
- Workers never see the full prompt flow for free: intermediate activations
  leak prompt content (same class of issue as SwarmLLM's 88.4% inversion) —
  own devices only.
- Pipeline latency adds per hop: more devices can be *slower* (measure).

## Coordinator (this repo, pure Rust)

Proto pin: **5.0.0** (`GGML_OP_COUNT == 101`), upstream commit
`f5b9bd39b56c7a7839a9795a100b6a00b84ac961` (2026-07-29). Master is already
6.0 — workers built from master will NOT handshake with a 5.0 coordinator.

RPC build. The sys crate vendors no `ggml/src/ggml-rpc/*`, so the 4 files
(`CMakeLists.txt`, `ggml-rpc.cpp`, `transport.cpp`, `transport.h`) are
vendored from the pin commit into the local registry copy
(`llama-cpp-sys-2-0.1.154/llama.cpp/ggml/src/ggml-rpc/`, 6th documented
registry addition next to the 4 build.rs patches + MTP log.cpp edit). The
sys `build.rs` forwards `GGML_*` env to CMake, so no build-script patch:

```bash
export PATH="/c/Users/plati/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-gnu"
cd /s/rust/llama-rs && unset CARGO_TARGET_DIR
GGML_RPC=ON cargo build --bin llama_serve --features rpc
GGML_RPC=ON cargo run --bin llama_serve --features rpc -- \
  models/Qwen3.8-27B-UD-IQ2_XXS.gguf --rpc 192.168.1.10:50052 --rpc 192.168.1.11:50052
```

Verified 2026-09-13: links clean, `register_backend: registered backend
RPC`, bad endpoint → usage error, refused endpoint → `Failed to connect`
in ~2 s, no hang.

`--rpc` repeats per worker (`host:port`, validated by
`llama_rs::parse_endpoint`). Registration runs after backend init, before
model load (`src/safe/rpc.rs`, one manual FFI symbol so the registry
`wrapper.h` needs no edit). `/v1/models` reports `rpc_workers` for the hub.

## Workers (user-side builds)

| Device | Path | Note |
|---|---|---|
| Raspberry Pi 4 | checkout pin `f5b9bd39`, `cmake -DGGML_RPC=ON`, build `ggml-rpc-server`, run `./ggml-rpc-server -p 50052 --cache` | deterministic first worker, wired LAN preferred |
| Samsung A54 | Termux at the same pin: build `ggml-rpc-server` (aarch64 CPU device) | candidate #2, test after Pi |
| Redmi 9 | skip for 27B shards (3–4 GiB RAM, weak GPU) | revisit only as draft holder |

Worker check: coordinator log must show the worker's free memory at
registration; `/v1/models` reports `rpc_workers`. Give each worker a
static LAN IP.

## Control plane (mirrors poolAI, no invention)

`poolAI/src/bin/poolai-worker.rs`: `register-remote` → `heartbeat-remote`
with capabilities (cpu cores, memory) → `pool/join` → poll/complete tasks.
A future `llama_worker` phone agent repeats this cycle for discovery/health;
compute stays on ggml RPC.

## Browser alternative (not Rust, experiment only)

`Nehanth/swarmllm` (MIT): 27B across browser tabs (WebGPU + WebRTC
pipeline, 10 KB activation per token, demo 10.7 tok/s MacBook+iPhone).
Needs per-device WebGPU + GiBs of weights per tab — check
`chrome://gpu` on the A54 first. Keep local `:8080` as the working core;
swarm layers add on top, they don't replace it.
