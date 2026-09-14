//! `llama_serve` — OpenAI-compatible HTTP server for llama-rs (BunkeRock).
//!
//! Serves the GGUF loaded by llama-rs on `127.0.0.1:8080` with `/v1` routes so
//! the GSV OmniRouter hub (`POST /api/omni/v1/chat/completions`,
//! `provider=bunke-rock`, base `http://127.0.0.1:8080/v1`) has a live upstream,
//! and Cursor / OpenCode can point at the hub instead of the bare port.
//!
//! Routes (all local, no auth):
//! - `GET /v1/models` (alias `GET /models`, `GET /health`) — model list
//! - `POST /v1/chat/completions` (alias `POST /chat/completions`) — chat
//!   (`{"model","messages":[{"role","content"}],"max_tokens","temperature","stream"}`;
//!   `stream:true` returns minimal SSE: one `data:` chunk + `data: [DONE]`).
//!
//! Single-threaded by design: one 27B mmap inference at a time on a low-RAM
//! box (a second request waits). Each chat gets a fresh context.
//! Without a model file the server still answers `/v1/models` (smoke test)
//! but chat returns `503 model not loaded`.
//!
//! Usage:
//! ```text
//! cargo run --bin llama_serve -- models/Qwen3.8-27B-UD-IQ2_XXS.gguf --port 8080
//! cargo run --bin llama_serve -- --port 18081   # smoke test, no model
//! ```

use clap::Parser;
use llama_rs::{Backend, ContextParams, GenerateOptions, Model, StagedLoadOptions};
use std::io::{Read, Write as _};
use std::net::TcpListener;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static REQ_ID: AtomicU64 = AtomicU64::new(1);

/// File log for hidden runs (no console window): every `slog` line goes to
/// stderr AND, when `--log-file` is set, appended to that file.
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
                slog(&format!("log file: {p}"));
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
#[command(name = "llama_serve")]
#[command(about = "llama-rs OpenAI-compatible server (BunkeRock, /v1 on :8080)")]
struct Args {
    /// Path to the GGUF model file (optional: serve /v1/models degraded without it).
    #[arg(index = 1)]
    model: Option<String>,

    /// Bind host (default 127.0.0.1, local only).
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Bind port (BunkeRock canon is 8080).
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Model id advertised in /v1/models (GSV catalog: lama-2.8).
    #[arg(long, default_value = "lama-2.8")]
    model_name: String,

    /// Default max new tokens when the client omits it (client value wins, capped).
    #[arg(long, default_value_t = 256)]
    max_tokens: u32,

    /// Default temperature when the client omits it.
    #[arg(long, default_value_t = 0.7)]
    temperature: f32,

    /// Disable mmap (fully read into RAM, needs ~8 GiB for 27B).
    #[arg(long, default_value_t = false)]
    no_mmap: bool,

    /// Pin model pages with mlock (needs privilege + RAM, default false).
    #[arg(long, default_value_t = false)]
    mlock: bool,

    /// Append log lines to this file too (for hidden runs with no console).
    #[arg(long)]
    log_file: Option<String>,

    /// GGML RPC worker(s) `host:port` (repeatable): phones via Termux,
    /// Raspberry Pi, spare PCs running `ggml-rpc-server` on the LAN.
    /// Needs an RPC build: `GGML_RPC=ON cargo ... --features rpc`.
    #[arg(long = "rpc")]
    rpc: Vec<String>,
}

/// Heartbeat path: `LLAMA_HEARTBEAT_PATH` override, else
/// `target/live/llama_heartbeat.json` (what GSV keep-live reads).
fn llama_heartbeat_path() -> std::path::PathBuf {
    std::env::var_os("LLAMA_HEARTBEAT_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("target/live/llama_heartbeat.json"))
}

fn heartbeat_enabled() -> bool {
    std::env::var_os("GSV_LIVE").is_some() || std::env::var_os("LLAMA_RS_HEARTBEAT").is_some()
}

fn heartbeat_body(model: &str) -> String {
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let safe: String = model
        .chars()
        .flat_map(|c| match c {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            _ => vec![c],
        })
        .collect();
    format!(
        r#"{{"pid":{},"model":"{}","epoch_secs":{},"bin_version":"{}"}}"#,
        std::process::id(),
        safe,
        epoch,
        env!("CARGO_PKG_VERSION")
    )
}

fn spawn_heartbeat(model: String) {
    if !heartbeat_enabled() {
        return;
    }
    let path = llama_heartbeat_path();
    let write = move |p: &std::path::Path| {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, heartbeat_body(&model)).is_ok() {
            let _ = std::fs::rename(&tmp, p);
        }
    };
    write(&path);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(15));
        write(&llama_heartbeat_path());
    });
}

/// Flatten OpenAI `messages` (string or `[{type:text}]` parts) into one prompt:
/// system lines first, then the rest in order, joined with newline.
fn messages_to_prompt(messages: &serde_json::Value) -> String {
    let arr = match messages.as_array() {
        Some(a) => a,
        None => return String::new(),
    };
    fn content_text(c: &serde_json::Value) -> String {
        match c {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(parts) => parts
                .iter()
                .filter_map(|p| {
                    if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                        p.get("text").and_then(|t| t.as_str())
                    } else if p.get("text").is_some() {
                        p.get("text").and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        }
    }
    let mut system = Vec::new();
    let mut rest = Vec::new();
    for m in arr {
        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        let text = m.get("content").map(content_text).unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        if role == "system" {
            system.push(text);
        } else if role == "assistant" {
            rest.push(format!("assistant: {text}"));
        } else {
            rest.push(text);
        }
    }
    system
        .into_iter()
        .chain(rest)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rough token estimate (no tokenizer here): ~4 chars per token.
fn estimate_tokens(s: &str) -> u64 {
    (s.len() as u64).div_ceil(4)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn models_body(model_name: &str, rpc_workers: &[String]) -> String {
    serde_json::json!({
        "object": "list",
        "data": [{
            "id": model_name,
            "object": "model",
            "owned_by": "bunke-rock",
            "created": 0,
        }],
        // Swarm state for the GSV hub operator (extra OpenAI field).
        "rpc_workers": rpc_workers,
    })
    .to_string()
}

fn chat_body(model_name: &str, text: &str, prompt_len: u64) -> String {
    let id = REQ_ID.fetch_add(1, Ordering::Relaxed);
    let comp = estimate_tokens(text);
    serde_json::json!({
        "id": format!("chatcmpl-llama-{id}"),
        "object": "chat.completion",
        "created": now_secs(),
        "model": model_name,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": text},
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": prompt_len,
            "completion_tokens": comp,
            "total_tokens": prompt_len + comp,
        },
    })
    .to_string()
}

fn sse_body(model_name: &str, text: &str) -> String {
    let id = REQ_ID.fetch_add(1, Ordering::Relaxed);
    let chunk = serde_json::json!({
        "id": format!("chatcmpl-llama-{id}"),
        "object": "chat.completion.chunk",
        "created": now_secs(),
        "model": model_name,
        "choices": [{
            "index": 0,
            "delta": {"role": "assistant", "content": text},
            "finish_reason": null,
        }],
    });
    format!("data: {}\n\ndata: [DONE]\n\n", chunk)
}

fn err_body(message: &str) -> String {
    serde_json::json!({
        "object": "error",
        "error": {"message": message, "type": "server_error"},
    })
    .to_string()
}

fn respond(stream: &mut std::net::TcpStream, status: u16, reason: &str, ctype: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

/// One inference job for the worker thread. The reply carries the generated
/// text or a short error tag (`model not loaded`, `context failed: …`,
/// `generation failed: …`).
struct InferenceJob {
    prompt: String,
    max_tokens: u32,
    temperature: f32,
    reply: std::sync::mpsc::Sender<Result<String, String>>,
}

/// Single-threaded inference owner: the 27B `Model` (and its contexts) never
/// leave this thread, so no `Send` bounds are needed and long generations
/// never block status endpoints (`GET /v1/models` answers from the accept
/// loop while a chat is generating).
struct InferenceWorker {
    backend: Backend,
    model: Option<Model>,
    jobs: std::sync::mpsc::Receiver<InferenceJob>,
}

impl InferenceWorker {
    fn run(self) {
        for job in self.jobs {
            let out = match &self.model {
                None => Err("model not loaded".to_string()),
                Some(model) => {
                    let mut ctx = match model.new_context(&self.backend, ContextParams::default()) {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = job.reply.send(Err(format!("context failed: {e}")));
                            continue;
                        }
                    };
                    let opts = GenerateOptions::builder()
                        .max_tokens(job.max_tokens)
                        .temperature(job.temperature)
                        .build();
                    match llama_rs::generate(model, &mut ctx, &job.prompt, &opts) {
                        Ok(out) => Ok(out),
                        Err(e) => Err(format!("generation failed: {e}")),
                    }
                }
            };
            let _ = job.reply.send(out);
        }
    }
}

struct Server {
    jobs: std::sync::mpsc::Sender<InferenceJob>,
    model_name: String,
    default_max_tokens: u32,
    default_temperature: f32,
    rpc_workers: Vec<String>,
}

impl Server {
    fn handle(&self, stream: &mut std::net::TcpStream) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        // Read headers.
        let mut raw = Vec::with_capacity(8192);
        let mut buf = [0u8; 4096];
        let header_end = loop {
            match stream.read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    raw.extend_from_slice(&buf[..n]);
                    if let Some(p) = find_header_end(&raw) {
                        break p;
                    }
                    if raw.len() > 1_048_576 {
                        respond(
                            stream,
                            413,
                            "Payload Too Large",
                            "application/json",
                            &err_body("headers too large"),
                        );
                        return;
                    }
                }
                Err(_) => return,
            }
        };
        let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or("");
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");
        let content_len: usize = lines
            .filter_map(|l| {
                let (k, v) = l.split_once(':')?;
                (k.trim().eq_ignore_ascii_case("content-length"))
                    .then(|| v.trim().parse::<usize>().ok())?
            })
            .next()
            .unwrap_or(0);
        // Read body.
        let mut body = raw[header_end..].to_vec();
        while body.len() < content_len {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => body.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
            if body.len() > 8_388_608 {
                respond(
                    stream,
                    413,
                    "Payload Too Large",
                    "application/json",
                    &err_body("body too large"),
                );
                return;
            }
        }
        body.truncate(content_len);

        match (method, path) {
            ("GET", "/v1/models" | "/models" | "/health" | "/api/health") => {
                respond(
                    stream,
                    200,
                    "OK",
                    "application/json",
                    &models_body(&self.model_name, &self.rpc_workers),
                );
                slog(&format!("GET {path} -> 200"));
            }
            ("POST", "/v1/chat/completions" | "/chat/completions") => {
                self.handle_chat(stream, &body);
            }
            ("OPTIONS", _) => {
                let head = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(head.as_bytes());
            }
            _ => {
                respond(
                    stream,
                    404,
                    "Not Found",
                    "application/json",
                    &err_body("not found"),
                );
                slog(&format!("{method} {path} -> 404"));
            }
        }
    }

    fn handle_chat(&self, stream: &mut std::net::TcpStream, body: &[u8]) {
        let v: serde_json::Value = match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                respond(
                    stream,
                    400,
                    "Bad Request",
                    "application/json",
                    &err_body(&format!("invalid JSON: {e}")),
                );
                return;
            }
        };
        let prompt = if let Some(p) = v.get("prompt").and_then(|p| p.as_str()) {
            p.to_string()
        } else {
            match v.get("messages") {
                Some(m) => messages_to_prompt(m),
                None => {
                    respond(
                        stream,
                        400,
                        "Bad Request",
                        "application/json",
                        &err_body("missing messages[] or prompt"),
                    );
                    return;
                }
            }
        };
        if prompt.is_empty() {
            respond(
                stream,
                400,
                "Bad Request",
                "application/json",
                &err_body("empty prompt"),
            );
            return;
        }
        let max_tokens = v
            .get("max_tokens")
            .or_else(|| v.get("max_completion_tokens"))
            .and_then(|n| n.as_u64())
            .map(|n| n.clamp(1, 4096) as u32)
            .unwrap_or(self.default_max_tokens);
        let temperature = v
            .get("temperature")
            .and_then(|n| n.as_f64())
            .map(|t| t as f32)
            .unwrap_or(self.default_temperature);
        let stream_mode = v.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

        // Hand off to the worker thread; status endpoints stay live meanwhile.
        let (tx, rx) = std::sync::mpsc::channel();
        if self
            .jobs
            .send(InferenceJob {
                prompt: prompt.clone(),
                max_tokens,
                temperature,
                reply: tx,
            })
            .is_err()
        {
            respond(
                stream,
                500,
                "Internal Server Error",
                "application/json",
                &err_body("inference worker gone"),
            );
            return;
        }
        match rx.recv() {
            Ok(Ok(out)) => {
                let prompt_toks = estimate_tokens(&prompt);
                if stream_mode {
                    let sse = sse_body(&self.model_name, &out);
                    respond(stream, 200, "OK", "text/event-stream", &sse);
                } else {
                    respond(
                        stream,
                        200,
                        "OK",
                        "application/json",
                        &chat_body(&self.model_name, &out, prompt_toks),
                    );
                }
                slog(&format!(
                    "POST chat (tokens={max_tokens}) -> 200, {} chars",
                    out.len()
                ));
            }
            Ok(Err(e)) if e == "model not loaded" => {
                respond(
                    stream,
                    503,
                    "Service Unavailable",
                    "application/json",
                    &err_body("model not loaded"),
                );
                slog("POST chat -> 503 (no model)");
            }
            Ok(Err(e)) => {
                respond(
                    stream,
                    500,
                    "Internal Server Error",
                    "application/json",
                    &err_body(&e),
                );
                slog(&format!("POST chat -> 500 ({e})"));
            }
            Err(_) => {
                respond(
                    stream,
                    500,
                    "Internal Server Error",
                    "application/json",
                    &err_body("inference worker gone"),
                );
            }
        }
    }
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

fn main() {
    let args = Args::parse();
    init_log(args.log_file.as_deref());
    slog(&format!(
        "llama_serve starting (model={:?}, {}:{})",
        args.model, args.host, args.port
    ));

    let backend = match Backend::init() {
        Ok(b) => b,
        Err(e) => {
            slog(&format!("error: backend init failed: {e}"));
            std::process::exit(1);
        }
    };
    slog("backend init ok");

    // RPC workers register after backend init, BEFORE model load (devices
    // must exist when weights spread). parse_endpoint validates always,
    // even on non-RPC builds; register_servers errors without the feature.
    let mut rpc_workers = Vec::new();
    for raw in &args.rpc {
        match llama_rs::parse_endpoint(raw) {
            Ok(ep) => rpc_workers.push(ep),
            Err(e) => {
                slog(&format!("error: {e}"));
                std::process::exit(1);
            }
        }
    }
    if !rpc_workers.is_empty() {
        match llama_rs::register_servers(&rpc_workers) {
            Ok(n) => slog(&format!(
                "rpc: {n} worker(s) registered: {}",
                rpc_workers.join(",")
            )),
            Err(e) => {
                slog(&format!("error: rpc: {e}"));
                std::process::exit(1);
            }
        }
    }

    let model_label = args
        .model
        .clone()
        .unwrap_or_else(|| "(no-model)".to_string());
    let model = match &args.model {
        Some(path) => {
            let p = std::path::Path::new(path);
            if !p.exists() {
                slog(&format!("error: model file not found: {}", p.display()));
                std::process::exit(1);
            }
            let staged = StagedLoadOptions::new()
                .with_mmap(!args.no_mmap)
                .with_mlock(args.mlock);
            slog(&format!(
                "loading {} (mmap={}, mlock={}) ...",
                p.display(),
                !args.no_mmap,
                args.mlock
            ));
            match Model::load_staged(&backend, p, staged, None::<fn(f32) -> bool>) {
                Ok(m) => {
                    slog("model loaded");
                    Some(m)
                }
                Err(e) => {
                    slog(&format!("error: failed to load model (staged): {e}"));
                    std::process::exit(1);
                }
            }
        }
        None => {
            slog("warning: no model file — /v1/models works, chat returns 503");
            None
        }
    };

    spawn_heartbeat(model_label);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            slog(&format!("error: bind {addr} failed: {e}"));
            std::process::exit(1);
        }
    };
    slog(&format!(
        "llama_serve: OpenAI-compat on http://{addr}/v1 (model {})",
        args.model_name
    ));
    // Inference owns backend+model on its own thread; the accept loop stays
    // responsive (GET /v1/models answers mid-generation).
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(|| {
        InferenceWorker {
            backend,
            model,
            jobs: rx,
        }
        .run()
    });
    let server = std::sync::Arc::new(Server {
        jobs: tx,
        model_name: args.model_name,
        default_max_tokens: args.max_tokens,
        default_temperature: args.temperature,
        rpc_workers,
    });
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let server = server.clone();
                std::thread::spawn(move || {
                    let mut s = s;
                    server.handle(&mut s);
                });
            }
            Err(e) => slog(&format!("accept failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_joins_system_then_user() {
        let msgs = json!([
            {"role": "user", "content": "hi"},
            {"role": "system", "content": "Be brief."},
            {"role": "user", "content": "2+2?"},
        ]);
        assert_eq!(messages_to_prompt(&msgs), "Be brief.\nhi\n2+2?");
    }

    #[test]
    fn prompt_flattens_part_arrays() {
        let msgs = json!([
            {"role": "user", "content": [
                {"type": "text", "text": "a"},
                {"type": "text", "text": "b"},
            ]},
        ]);
        assert_eq!(messages_to_prompt(&msgs), "ab");
    }

    #[test]
    fn models_body_lists_bunke_rock() {
        let v: serde_json::Value =
            serde_json::from_str(&models_body("lama-2.8", &[])).expect("valid json");
        assert_eq!(v["object"], "list");
        assert_eq!(v["data"][0]["id"], "lama-2.8");
        assert_eq!(v["data"][0]["owned_by"], "bunke-rock");
        assert_eq!(v["rpc_workers"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn models_body_reports_rpc_workers() {
        let workers = vec!["192.168.1.10:50052".to_string()];
        let v: serde_json::Value =
            serde_json::from_str(&models_body("lama-2.8", &workers)).expect("valid json");
        assert_eq!(v["rpc_workers"][0], "192.168.1.10:50052");
    }

    #[test]
    fn chat_body_is_openai_shaped() {
        let v: serde_json::Value =
            serde_json::from_str(&chat_body("lama-2.8", "hello", 8)).expect("valid json");
        assert_eq!(v["object"], "chat.completion");
        assert_eq!(v["choices"][0]["message"]["content"], "hello");
        assert_eq!(v["usage"]["prompt_tokens"], 8);
    }

    #[test]
    fn worker_without_model_replies_503_tag() {
        let backend = Backend::init().expect("backend init");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(|| {
            InferenceWorker {
                backend,
                model: None,
                jobs: rx,
            }
            .run()
        });
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        tx.send(InferenceJob {
            prompt: "hi".to_string(),
            max_tokens: 8,
            temperature: 0.0,
            reply: reply_tx,
        })
        .expect("send");
        assert_eq!(
            reply_rx.recv().expect("reply"),
            Err("model not loaded".to_string())
        );
    }

    #[test]
    fn sse_body_ends_with_done() {
        let s = sse_body("lama-2.8", "hi");
        assert!(s.starts_with("data: "), "{s}");
        assert!(s.ends_with("data: [DONE]\n\n"), "{s}");
    }
}
