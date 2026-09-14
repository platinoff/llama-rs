//! `llama_edge` — poolAI edge worker agent for llama shards (pure Rust).
//!
//! Mirrors `poolAI/src/bin/poolai-worker.rs` (register → heartbeat →
//! pool/join → poll → complete, job-lease renew) but speaks plain blocking
//! HTTP over std `TcpStream` (no tokio/reqwest) so it builds anywhere,
//! including Raspberry Pi aarch64 and Termux. Registers with
//! `origin=telegram_edge` + `role=virtual_node` (join requires the role)
//! and an ed25519-signed capability document (unsigned docs are rejected
//! for telegram_edge); phones bind via Telegram, tensors stay on
//! ggml-rpc-server (see `docs/DISTRIBUTED.md`).
//!
//! ```text
//! cargo run --bin llama_edge -- --coordinator http://127.0.0.1:8091 \
//!   --worker-id edge-test-01 --rpc-endpoint 192.168.1.10:50052 \
//!   --telegram-id 999001
//! ```

use clap::Parser;
use std::io::{Read, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// File log for hidden runs (same pattern as `llama_serve`).
static LOG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

fn init_log(path: Option<&str>) {
    if let Some(p) = path {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            Ok(f) => {
                let _ = LOG_FILE.set(Mutex::new(f));
            }
            Err(e) => eprintln!("warning: cannot open log file {p}: {e}"),
        }
    }
}

fn slog(msg: &str) {
    let line = format!(
        "[{}] {msg}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    eprintln!("{line}");
    if let Some(m) = LOG_FILE.get() {
        if let Ok(mut f) = m.lock() {
            let _ = writeln!(f, "{line}");
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "llama_edge")]
#[command(about = "llama shard edge worker for the poolAI coordinator")]
struct Args {
    /// Coordinator base URL (poolAI serves :8091 here; :8080 is llama_serve).
    #[arg(long, default_value = "http://127.0.0.1:8091")]
    coordinator: String,

    /// Worker peer id (must match poolAI worker-id rules).
    #[arg(long, default_value = "llama-edge-01")]
    worker_id: String,

    /// This worker's ggml-rpc-server endpoint to advertise + probe.
    #[arg(long, default_value = "127.0.0.1:50052")]
    rpc_endpoint: String,

    /// Optional Telegram user id: bound to this peer after register.
    #[arg(long)]
    telegram_id: Option<String>,

    /// Heartbeat period in seconds.
    #[arg(long, default_value_t = 15)]
    heartbeat_secs: u64,

    /// Task poll period in seconds.
    #[arg(long, default_value_t = 5)]
    poll_secs: u64,

    /// Max RAM to advertise in MiB (0 = auto-detect via sysinfo).
    #[arg(long, default_value_t = 0)]
    max_memory_mb: usize,

    /// Ed25519 signing key hex for the capability document (dev default:
    /// 32×0x07 matching poolAI's dev verify key; override via
    /// LLAMA_EDGE_SIGNING_KEY for anything beyond LAN tests).
    #[arg(long)]
    signing_key: Option<String>,

    /// Append log lines to this file too (for hidden runs with no console).
    #[arg(long)]
    log_file: Option<String>,
}

fn signing_key_bytes(args: &Args) -> Result<[u8; 32], String> {
    let hex = args
        .signing_key
        .clone()
        .or_else(|| std::env::var("LLAMA_EDGE_SIGNING_KEY").ok())
        .unwrap_or_else(|| "07".repeat(32));
    let raw = hex::decode(hex.trim()).map_err(|e| format!("signing key hex: {e}"))?;
    raw.try_into()
        .map_err(|_| "signing key must be 32 bytes".to_string())
}

/// Days → civil date (Howard Hinnant's algorithm, public domain).
fn unix_to_rfc3339(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// llama_serve base URL for `llama_chat` execution.
fn serve_url() -> String {
    std::env::var("LLAMA_SERVE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string())
}

/// Minimal blocking HTTP: returns (status_code, body). Follows the same
/// shape as `llama_serve`'s parser (single response, Connection: close).
/// `timeout_secs` covers connect + full read (chat needs minutes).
fn http_json(
    method: &str,
    url: &str,
    body: Option<&str>,
    timeout_secs: u64,
) -> Result<(u16, String), String> {
    let (host_port, path) = split_url(url)?;
    let timeout = Duration::from_secs(timeout_secs.max(1));
    let mut stream = std::net::TcpStream::connect_timeout(&parse_addr(&host_port)?, timeout)
        .map_err(|e| format!("connect {host_port}: {e}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;
    let len = body.map(str::len).unwrap_or(0);
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{}",
        body.unwrap_or("")
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("send: {e}"))?;
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
            Err(e) => return Err(format!("recv: {e}")),
        }
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    let status: u16 = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| "bad status line".to_string())?;
    let body = text
        .split_once("\r\n\r\n")
        .map(|x| x.1.to_string())
        .unwrap_or_default();
    Ok((status, body))
}

fn split_url(url: &str) -> Result<(String, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("only http:// URLs: {url}"))?;
    match rest.split_once('/') {
        Some((h, p)) => Ok((h.to_string(), format!("/{p}"))),
        None => Ok((rest.to_string(), "/".to_string())),
    }
}

fn parse_addr(host_port: &str) -> Result<std::net::SocketAddr, String> {
    let (host, port) = host_port
        .rsplit_once(':')
        .ok_or_else(|| format!("bad host:port: {host_port}"))?;
    let port: u16 = port.parse().map_err(|_| format!("bad port: {host_port}"))?;
    use std::net::ToSocketAddrs as _;
    (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("resolve {host_port}: {e}"))?
        .next()
        .ok_or_else(|| format!("no addr: {host_port}"))
}

fn total_ram_mb() -> usize {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    (sys.total_memory() / 1024 / 1024) as usize
}

fn cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// TCP probe of the worker's own ggml-rpc-server (2s budget).
fn rpc_reachable(endpoint: &str) -> bool {
    let addr = match endpoint
        .rsplit_once(':')
        .and_then(|(h, p)| p.parse::<u16>().ok().map(|p| (h, p)))
    {
        Some((h, p)) => format!("{h}:{p}"),
        None => return false,
    };
    use std::net::ToSocketAddrs as _;
    let Ok(mut addrs) = addr.to_socket_addrs() else {
        return false;
    };
    let Some(sock) = addrs.next() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&sock, Duration::from_secs(2)).is_ok()
}

/// Sign `"{peer}:{caps,}"` with the dev/provided key (poolAI dev-verify compatible).
fn sign_capability(key: &[u8; 32], peer: &str, caps: &[String]) -> Result<String, String> {
    use ed25519_dalek::Signer as _;
    let sk = ed25519_dalek::SigningKey::from_bytes(key);
    let msg = format!("{}:{}", peer.trim(), caps.join(","));
    Ok(hex::encode(sk.sign(msg.as_bytes()).to_bytes()))
}

fn capability_doc(
    key: &[u8; 32],
    peer: &str,
    caps: &[String],
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "peer_id": peer,
        "capabilities": caps,
        "signature": sign_capability(key, peer, caps)?,
        "expires_at": unix_to_rfc3339(now_secs() + 30 * 86400),
    }))
}

fn post(coordinator: &str, path: &str, body: &serde_json::Value) -> Result<(u16, String), String> {
    let url = format!("{}{path}", coordinator.trim_end_matches('/'));
    http_json("POST", &url, Some(&body.to_string()), 15)
}

fn get(coordinator: &str, path: &str) -> Result<(u16, String), String> {
    let url = format!("{}{path}", coordinator.trim_end_matches('/'));
    http_json("GET", &url, None, 15)
}

fn check(status: u16, body: &str, what: &str) -> Result<(), String> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(format!(
            "{what} HTTP {status}: {}",
            body.chars().take(200).collect::<String>()
        ))
    }
}

fn main() {
    let args = Args::parse();
    init_log(args.log_file.as_deref());
    if let Err(e) = run(args) {
        slog(&format!("llama_edge error: {e}"));
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let key = signing_key_bytes(&args)?;
    let mem_mb = if args.max_memory_mb > 0 {
        args.max_memory_mb
    } else {
        total_ram_mb()
    };
    let caps = vec!["inference:cpu".to_string(), "llama_shard".to_string()];
    let coord = args.coordinator.clone();
    let peer = args.worker_id.clone();

    slog(&format!(
        "llama_edge {peer} -> {coord} (rpc {})",
        args.rpc_endpoint
    ));

    // 1–3. register + bind + join (idempotent, re-runs on 404).
    join_once(&args, &key, &caps, mem_mb, &coord, &peer)?;

    // 4. heartbeat + poll loop. A 404 means the coordinator forgot us
    // (restart wipes in-memory registrations) — re-join instead of
    // haunting the loop as a ghost.
    loop {
        if !heartbeat_once(&coord, &peer, mem_mb) {
            slog("heartbeat lost registration; re-joining");
            if join_once(&args, &key, &caps, mem_mb, &coord, &peer).is_err() {
                std::thread::sleep(Duration::from_secs(30));
                continue;
            }
        }
        poll_once(&coord, &peer, &args);
        std::thread::sleep(Duration::from_secs(args.poll_secs.max(1)));
    }
}

/// Register + telegram bind + pool join (steps 1–3). Idempotent: safe to
/// re-run after the coordinator restarts.
fn join_once(
    args: &Args,
    key: &[u8; 32],
    caps: &[String],
    mem_mb: usize,
    coord: &str,
    peer: &str,
) -> Result<(), String> {
    // 1. register-remote (telegram_edge + virtual_node role + signed doc).
    let doc = capability_doc(key, peer, caps)?;
    let (st, body) = post(
        coord,
        "/api/v1/discovery/register-remote",
        &serde_json::json!({
            "peer_id": peer,
            "address": "127.0.0.1",
            "port": 0,
            "capabilities": {
                "cpu_cores": cpu_cores(),
                "memory_mb": mem_mb,
                "gpu_devices": [],
                "supports_tensor_parallelism": false,
                "supports_pipeline_parallelism": false,
            },
            "metadata": {
                "origin": "telegram_edge",
                "role": "virtual_node",
                "channel": "telegram",
                "rpc_endpoint": args.rpc_endpoint,
            },
            "capability_document": doc,
        }),
    )?;
    check(st, &body, "register-remote")?;
    slog(&format!("registered: {body}"));

    // 2. optional Telegram bind.
    if let Some(tg) = &args.telegram_id {
        let (st, body) = post(
            coord,
            "/api/v1/virtual-nodes/telegram/bind",
            &serde_json::json!({"telegram_user_id": tg, "peer_id": peer}),
        )?;
        check(st, &body, "telegram bind")?;
        slog(&format!("telegram bound: {body}"));
    }

    // 3. pool/join.
    let (st, body) = post(
        coord,
        &format!("/api/v1/virtual-nodes/{peer}/pool/join"),
        &serde_json::json!({"max_memory_mb": mem_mb, "max_concurrent_requests": 4}),
    )?;
    check(st, &body, "pool join").or_else(|e| {
        // 503 = pool not ready yet: stay discovery-only like poolai-worker.
        if e.contains("503") {
            slog("pool not ready; discovery-only");
            Ok(())
        } else {
            Err(e)
        }
    })
}

/// Returns false when the coordinator forgot us (404) so the caller re-joins.
fn heartbeat_once(coord: &str, peer: &str, mem_mb: usize) -> bool {
    static mut LAST: u64 = 0;
    let now = now_secs();
    // Heartbeat every ~15s regardless of poll cadence.
    let due = unsafe {
        if now.saturating_sub(LAST) < 15 {
            false
        } else {
            LAST = now;
            true
        }
    };
    if !due {
        return true;
    }
    match post(
        coord,
        "/api/v1/discovery/heartbeat-remote",
        &serde_json::json!({
            "peer_id": peer,
            "capabilities": {
                "cpu_cores": cpu_cores(),
                "memory_mb": mem_mb,
                "gpu_devices": [],
                "supports_tensor_parallelism": false,
                "supports_pipeline_parallelism": false,
                "active_requests": 0,
                "capacity": 4,
                "current_load": 0.0,
            },
        }),
    ) {
        Ok((st, _)) if (200..300).contains(&st) => true,
        Ok((404, _)) => false,
        Ok((st, body)) => {
            slog(&format!("heartbeat HTTP {st}: {body}"));
            true
        }
        Err(e) => {
            slog(&format!("heartbeat failed: {e}"));
            true
        }
    }
}

fn poll_once(coord: &str, peer: &str, args: &Args) {
    let (st, body) = match get(coord, &format!("/api/v1/virtual-nodes/{peer}/tasks/poll")) {
        Ok(v) => v,
        Err(e) => {
            slog(&format!("poll failed: {e}"));
            return;
        }
    };
    if !(200..300).contains(&st) {
        slog(&format!("poll HTTP {st}"));
        return;
    }
    let task_id = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("task")?.get("id")?.as_str().map(str::to_string));
    let task_type = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("task")?
                .get("task_type")?
                .as_str()
                .map(str::to_string)
        });
    let payload = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("task")?.get("payload").cloned())
        .unwrap_or(serde_json::Value::Null);
    let (Some(id), Some(kind)) = (task_id, task_type) else {
        return;
    };
    slog(&format!("task {id} ({kind})"));
    let (status, detail) = execute_task(&kind, &payload, args);
    match post(
        coord,
        &format!("/api/v1/virtual-nodes/{peer}/tasks/{id}/complete"),
        &serde_json::json!({"status": status, "detail": detail}),
    ) {
        Ok((st, _)) if (200..300).contains(&st) => slog(&format!(
            "task {id} {status}: {}",
            detail.chars().take(120).collect::<String>()
        )),
        Ok((st, b)) => slog(&format!("complete HTTP {st}: {b}")),
        Err(e) => slog(&format!("complete failed: {e}")),
    }
}

/// Tier routing: `fast` = interactive small model, anything else = deep 27B.
pub fn serve_endpoint(tier: &str) -> (String, String) {
    if tier.trim().eq_ignore_ascii_case("fast") {
        let base = std::env::var("LLAMA_SERVE_FAST_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:8082".to_string());
        (base, "lama-1.5".to_string())
    } else {
        (serve_url(), "lama-2.8".to_string())
    }
}

/// Build the llama_serve chat body for a `llama_chat` payload.
/// `payload.model`: `"fast"` (default `deep` when absent).
fn build_chat_body(payload: &serde_json::Value) -> (String, serde_json::Value) {
    let prompt = payload
        .get("prompt")
        .and_then(|p| p.as_str())
        .unwrap_or("Hello")
        .to_string();
    let max_tokens = payload
        .get("max_tokens")
        .and_then(|n| n.as_u64())
        .map(|n| n.clamp(1, 512))
        .unwrap_or(64);
    let tier = payload
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("deep");
    let (base, model) = serve_endpoint(tier);
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": 0,
    });
    (base, body)
}

/// Pull the assistant text out of a chat-completions body.
fn extract_answer(body: &str) -> Result<String, String> {
    serde_json::from_str::<serde_json::Value>(body)
        .map_err(|e| format!("chat JSON: {e}"))?
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .ok_or_else(|| "chat: no choices[0].message.content".to_string())
}

/// Parse a shard range `"start-end"` (e.g. `"0-16"`). Strict: both numbers,
/// start < end, end within a sane layer cap.
pub fn parse_layers(s: &str) -> Result<(u32, u32), String> {
    let (a, b) = s
        .trim()
        .split_once('-')
        .ok_or_else(|| format!("bad layers (want start-end): {s}"))?;
    let start: u32 = a
        .trim()
        .parse()
        .map_err(|_| format!("bad layers start: {s}"))?;
    let end: u32 = b
        .trim()
        .parse()
        .map_err(|_| format!("bad layers end: {s}"))?;
    if start >= end {
        return Err(format!("bad layers (start >= end): {s}"));
    }
    if end > 4096 {
        return Err(format!("bad layers (end > 4096): {s}"));
    }
    Ok((start, end))
}

/// Execute task types this agent owns; returns (status, detail).
/// Unknown types are left for other workers (poolAI convention).
fn execute_task(kind: &str, payload: &serde_json::Value, args: &Args) -> (String, String) {
    match kind {
        "ping" => ("completed".to_string(), "pong from llama_edge".to_string()),
        "llama_shard" => {
            // v1 contract: {model?, layers: "start-end", rpc_endpoint?}.
            // Records the assignment even when the tensor worker is down
            // (reachable flag tells the truth); bad layers fail the task.
            let model = payload
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or("lama-2.8")
                .to_string();
            let layers_raw = payload.get("layers").and_then(|l| l.as_str()).unwrap_or("");
            let (start, end) = match parse_layers(layers_raw) {
                Ok(v) => v,
                Err(e) => return ("failed".to_string(), e),
            };
            let endpoint = payload
                .get("rpc_endpoint")
                .and_then(|e| e.as_str())
                .unwrap_or(&args.rpc_endpoint)
                .to_string();
            let ok = rpc_reachable(&endpoint);
            let detail = serde_json::json!({
                "model": model,
                "assigned_layers": format!("{start}-{end}"),
                "rpc_endpoint": endpoint,
                "rpc_reachable": ok,
                "worker": args.worker_id,
            })
            .to_string();
            ("completed".to_string(), detail)
        }
        "llama_chat" => {
            let (base, chat) = build_chat_body(payload);
            let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
            match http_json("POST", &url, Some(&chat.to_string()), 600) {
                Ok((st, body)) if (200..300).contains(&st) => match extract_answer(&body) {
                    Ok(text) => ("completed".to_string(), text),
                    Err(e) => ("failed".to_string(), e),
                },
                Ok((st, body)) => (
                    "failed".to_string(),
                    format!(
                        "llama_serve HTTP {st}: {}",
                        body.chars().take(200).collect::<String>()
                    ),
                ),
                Err(e) => ("failed".to_string(), format!("llama_serve: {e}")),
            }
        }
        _ => (
            "completed".to_string(),
            format!("llama_edge: task type {kind} ignored"),
        ),
    }
}

/// Lease-renew ticker state for job tasks (poolAI PH-S116 pattern).
#[allow(dead_code)]
struct LeaseGuard {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[allow(dead_code)]
impl LeaseGuard {
    fn start(coord: String, job_id: String, epoch: u64, interval_secs: u64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(interval_secs.max(1)));
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                let url = format!(
                    "{}/api/v1/jobs/{job_id}/lease/renew",
                    coord.trim_end_matches('/')
                );
                match http_json(
                    "POST",
                    &url,
                    Some(&serde_json::json!({"lease_epoch": epoch}).to_string()),
                    15,
                ) {
                    Ok((st, _)) if (200..300).contains(&st) => {}
                    Ok((409, _)) => break,
                    _ => eprintln!("lease renew failed for {job_id} (retry)"),
                }
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_shapes() {
        // 2026-09-13T00:00:00Z == 1789257600
        assert_eq!(unix_to_rfc3339(1789257600), "2026-09-13T00:00:00Z");
        assert_eq!(unix_to_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn dev_key_matches_poolai_fixture() {
        // poolAI tests/fixtures/capability/dev_pubkey.hex
        let key = [7u8; 32];
        let sk = ed25519_dalek::SigningKey::from_bytes(&key);
        assert_eq!(
            hex::encode(sk.verifying_key().to_bytes()),
            "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c"
        );
    }

    #[test]
    fn capability_doc_signs_poolai_message() {
        let key = [7u8; 32];
        let caps = vec!["inference:cpu".to_string(), "llama_shard".to_string()];
        let doc = capability_doc(&key, "edge-test-01", &caps).expect("doc");
        assert_eq!(doc["peer_id"], "edge-test-01");
        assert!(doc["signature"].as_str().is_some_and(|s| s.len() == 128));
        assert!(doc["expires_at"].as_str().is_some());
        // poolAI signs "{peer}:{caps,}".
        let sig_hex = doc["signature"].as_str().unwrap();
        let vk = ed25519_dalek::SigningKey::from_bytes(&key).verifying_key();
        let sig_bytes = hex::decode(sig_hex).expect("hex");
        let sig = ed25519_dalek::Signature::from_slice(&sig_bytes).expect("sig");
        vk.verify_strict(b"edge-test-01:inference:cpu,llama_shard", &sig)
            .expect("verify");
    }

    #[test]
    fn parse_layers_shapes() {
        assert_eq!(parse_layers("0-16").unwrap(), (0, 16));
        assert_eq!(parse_layers(" 32 - 64 ").unwrap(), (32, 64));
        assert!(parse_layers("16-16").is_err());
        assert!(parse_layers("20-10").is_err());
        assert!(parse_layers("abc").is_err());
        assert!(parse_layers("0-99999").is_err());
        assert!(parse_layers("").is_err());
    }

    #[test]
    fn shard_arm_contract() {
        let args = Args::parse_from([
            "llama_edge",
            "--worker-id",
            "t",
            "--rpc-endpoint",
            "127.0.0.1:9",
        ]);
        // Valid layers, dead endpoint → completed with reachable=false.
        let (st, detail) = execute_task(
            "llama_shard",
            &serde_json::json!({"model": "lama-2.8", "layers": "0-16"}),
            &args,
        );
        assert_eq!(st, "completed");
        let d: serde_json::Value = serde_json::from_str(&detail).expect("json");
        assert_eq!(d["assigned_layers"], "0-16");
        assert_eq!(d["rpc_reachable"], false);
        // Bad layers → failed (contract violation).
        let (st, _) = execute_task("llama_shard", &serde_json::json!({"layers": "x"}), &args);
        assert_eq!(st, "failed");
        // Unknown types stay untouched for other workers.
        let (st, _) = execute_task("nope", &serde_json::json!({}), &args);
        assert_eq!(st, "completed");
    }

    #[test]
    fn split_url_shapes() {
        assert_eq!(
            split_url("http://127.0.0.1:8091/api/x").unwrap(),
            ("127.0.0.1:8091".to_string(), "/api/x".to_string())
        );
    }

    #[test]
    fn chat_body_defaults_and_caps() {
        let (base, v) = build_chat_body(&serde_json::json!({}));
        assert_eq!(v["model"], "lama-2.8");
        assert_eq!(v["max_tokens"], 64);
        assert!(base.ends_with(":8080"));
        let (_base, v) = build_chat_body(&serde_json::json!({"prompt": "hi", "max_tokens": 99999}));
        assert_eq!(v["max_tokens"], 512);
        assert_eq!(v["messages"][0]["content"], "hi");
        let (base, v) = build_chat_body(&serde_json::json!({"model": "fast"}));
        assert_eq!(v["model"], "lama-1.5");
        assert!(base.ends_with(":8082"));
    }

    #[test]
    fn extract_answer_shapes() {
        let ok = r#"{"choices":[{"message":{"content":"hey"}}]}"#;
        assert_eq!(extract_answer(ok).unwrap(), "hey");
        assert!(extract_answer(r#"{"choices":[]}"#).is_err());
        assert!(extract_answer("not json").is_err());
    }
}
