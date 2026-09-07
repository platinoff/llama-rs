//! CLI entry point for llama.rs (64-bit release binary).
//!
//! Usage:
//!   llama_rs                      — print greeting
//!   llama_rs <model.gguf> [prompt] — load model and generate
//!   llama_rs --max-tokens 64 --temperature 0.5 model.gguf "Hello"
//!   llama_rs --system "You are helpful." model.gguf "Explain this"
//!   llama_rs --help               — show all options

use clap::Parser;
use llama_rs::{Backend, ContextParams, GenerateOptions, Model, StagedLoadOptions};
use std::io::Write as _;
use std::path::Path;
use std::time::Duration;

/// Best-effort push of staged progress to GSV live (`GSV_LIVE=1` → `127.0.0.1:9999`).
/// Pure Rust std only, no extra deps, 150ms timeout, ignore errors.
fn gsv_report_progress(p: f32) {
    if std::env::var_os("GSV_LIVE").is_none() {
        return;
    }
    let body = format!(
        r#"{{"staged_progress":{:.3},"ts":{}}}"#,
        p,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let req = format!(
        "POST /api/ingest HTTP/1.1\r\nHost: 127.0.0.1:9999\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    if let Ok(mut s) = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], 9999)),
        Duration::from_millis(120),
    ) {
        let _ = s.set_write_timeout(Some(Duration::from_millis(80)));
        let _ = s.write_all(req.as_bytes());
    }
}

/// Heartbeat file path: `LLAMA_HEARTBEAT_PATH` override, else `target/live/llama_heartbeat.json`
/// (repo-relative). Same env/DSN the GSV `keep_live` box reads (band 225/226).
fn llama_heartbeat_path() -> std::path::PathBuf {
    std::env::var_os("LLAMA_HEARTBEAT_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("target/live/llama_heartbeat.json"))
}

/// Heartbeat is written when `GSV_LIVE=1` **or** `LLAMA_RS_HEARTBEAT=1`.
fn llama_heartbeat_enabled() -> bool {
    std::env::var_os("GSV_LIVE").is_some() || std::env::var_os("LLAMA_RS_HEARTBEAT").is_some()
}

/// Minimal JSON string escaping for the model field (backslash / quote only).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

/// Serialize `target/live/llama_heartbeat.json` — the shape GSV's `LlamaHeartbeat` reads:
/// `{pid, model, epoch_secs, bin_version}` (fresh when age ≤ 60s).
fn llama_heartbeat_body(model: &str) -> String {
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!(
        r#"{{"pid":{},"model":"{}","epoch_secs":{},"bin_version":"{}"}}"#,
        std::process::id(),
        json_escape(model),
        epoch,
        env!("CARGO_PKG_VERSION")
    )
}

/// Atomic write of the heartbeat (temp file + rename), creates parent dirs.
fn write_llama_heartbeat(path: &std::path::Path, model: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, llama_heartbeat_body(model))?;
    std::fs::rename(&tmp, path)
}

/// Spawn the background heartbeat thread (15s tick) when enabled; writes once up front.
/// Returns `None` when disabled (no heartbeat). Thread dies with the process — the CLI is
/// one-shot, and a fresh file during staged load + inference is exactly what keep-live wants.
fn spawn_llama_heartbeat(model: &str) -> Option<std::thread::JoinHandle<()>> {
    if !llama_heartbeat_enabled() {
        return None;
    }
    let path = llama_heartbeat_path();
    let model = model.to_string();
    let _ = write_llama_heartbeat(&path, &model);
    Some(std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(15));
        let _ = write_llama_heartbeat(&path, &model);
    }))
}

#[derive(Parser, Debug)]
#[command(name = "llama_rs")]
#[command(about = "llama.rs — Llama in Rust (backend: llama.cpp, staged disk→RAM)")]
struct Args {
    /// Path to the GGUF model file.
    #[arg(index = 1)]
    model: Option<String>,

    /// Prompt to complete (default: "Hello"). Ignored if --system is used without prompt.
    #[arg(index = 2)]
    prompt: Option<String>,

    /// Maximum new tokens to generate.
    #[arg(long, default_value_t = 256)]
    max_tokens: u32,

    /// Sampling temperature (0 = greedy, >0 for sampling).
    #[arg(long, default_value_t = 0.7)]
    temperature: f32,

    /// Random seed (omit for non-deterministic).
    #[arg(long)]
    seed: Option<u32>,

    /// Do not stop at end-of-sequence token.
    #[arg(long)]
    no_eos: bool,

    /// System or prefix prompt (prepended to the main prompt with a newline).
    #[arg(long)]
    system: Option<String>,

    /// Use mmap for model load (default true; low-RAM, paged). Use --no-mmap to fully resident.
    #[arg(long, default_value_t = true)]
    mmap: bool,

    /// Disable mmap (fully read into RAM, needs ~8 GiB for 27B).
    #[arg(long, default_value_t = false)]
    no_mmap: bool,

    /// Pin model pages with mlock (needs privilege + RAM, default false).
    #[arg(long, default_value_t = false)]
    mlock: bool,

    /// Show staged load progress (0..100%).
    #[arg(long, default_value_t = false)]
    progress: bool,
}

fn main() {
    let args = Args::parse();

    if args.model.is_none() {
        println!("{}", llama_rs::hello_llama_rust());
        return;
    }

    let model_arg = args.model.as_ref().unwrap();
    let path = Path::new(model_arg);
    if !path.exists() {
        eprintln!("error: model file not found: {}", path.display());
        std::process::exit(1);
    }

    // Heartbeat (band 226): `LLAMA_RS_HEARTBEAT=1` or `GSV_LIVE=1` writes
    // target/live/llama_heartbeat.json every 15s so GSV keep-live sees llama_rs up.
    let _heartbeat = spawn_llama_heartbeat(model_arg);

    let backend = match Backend::init() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: backend init failed: {}", e);
            std::process::exit(1);
        }
    };

    let use_mmap = if args.no_mmap { false } else { args.mmap };
    let staged = StagedLoadOptions::new()
        .with_mmap(use_mmap)
        .with_mlock(args.mlock);
    // Staged loading with optional progress (pure Rust, controls disk→RAM).
    let model = if args.progress {
        let mut last_pct = 0u32;
        match Model::load_staged_with_progress(&backend, path, staged, &mut |p: f32| {
            let pct = (p * 100.0) as u32;
            gsv_report_progress(p);
            if pct != last_pct && pct.is_multiple_of(5) {
                eprintln!("loading {}% (mmap={}, mlock={})", pct, use_mmap, args.mlock);
                last_pct = pct;
            }
            true
        }) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: failed to load model (staged): {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match Model::load_staged(&backend, path, staged, None::<fn(f32) -> bool>) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: failed to load model (staged): {}", e);
                std::process::exit(1);
            }
        }
    };

    let ctx_params = ContextParams::default();
    let mut context = match model.new_context(&backend, ctx_params) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to create context: {}", e);
            std::process::exit(1);
        }
    };

    let prompt = match (&args.system, &args.prompt) {
        (Some(s), Some(p)) => format!("{}\n{}", s, p),
        (Some(s), None) => s.clone(),
        (None, Some(p)) => p.clone(),
        (None, None) => "Hello".to_string(),
    };

    let mut opts_builder = GenerateOptions::builder()
        .max_tokens(args.max_tokens)
        .temperature(args.temperature)
        .stop_at_eos(!args.no_eos);
    if let Some(s) = args.seed {
        opts_builder = opts_builder.seed(s);
    }
    let opts = opts_builder.build();

    match llama_rs::generate(&model, &mut context, &prompt, &opts) {
        Ok(out) => print!("{}", out),
        Err(e) => {
            eprintln!("error: generation failed: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_body_shape_matches_gsv_schema() {
        let body = llama_heartbeat_body("models/Qwen.gguf");
        assert!(body.starts_with('{') && body.ends_with('}'));
        assert!(body.contains(&format!(r#""pid":{}"#, std::process::id())));
        assert!(body.contains(r#""model":"models/Qwen.gguf""#));
        // epoch_secs is an integer timestamp parseable as u64
        let ts = body
            .split("\"epoch_secs\":")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap();
        let ts: u64 = ts.parse().expect("epoch_secs numeric");
        assert!(
            ts > 1_700_000_000,
            "epoch must be a sane unix seconds, got {ts}"
        );
        assert!(body.contains(&format!(r#""bin_version":"{}""#, env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn heartbeat_body_escapes_json_in_model_path() {
        let body = llama_heartbeat_body(r#"models\Q\"wen.gguf"#);
        assert!(body.contains(r##""model":"models\\Q\\\"wen.gguf""##));
        assert!(
            !body.contains("Q\"wen.gguf\"}\""),
            "quote inside model must be escaped"
        );
    }

    #[test]
    fn heartbeat_enabled_flags() {
        if llama_heartbeat_enabled() {
            // either env present — nothing to assert beyond the flag
            assert!(
                std::env::var_os("GSV_LIVE").is_some()
                    || std::env::var_os("LLAMA_RS_HEARTBEAT").is_some()
            );
        } else {
            assert!(
                std::env::var_os("GSV_LIVE").is_none()
                    && std::env::var_os("LLAMA_RS_HEARTBEAT").is_none()
            );
        }
    }

    #[test]
    fn write_heartbeat_creates_file_atomic() {
        let dir = std::env::temp_dir().join(format!("llama_rs_hb_test_{}", std::process::id()));
        let path = dir.join("target/live/llama_heartbeat.json");
        let _ = std::fs::remove_dir_all(&dir);
        write_llama_heartbeat(&path, "models/Qwen.gguf").expect("write ok");
        assert!(path.is_file(), "heartbeat file must exist");
        let text = std::fs::read_to_string(&path).expect("readable");
        assert!(text.contains("\"model\":\"models/Qwen.gguf\""));
        // temp file must be gone (atomic rename)
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
