//! Std-only rolling file sink for `llama_serve` / `llama_edge`.
//!
//! No new deps: crate builds offline by policy. `target/live/logs/<name>.log`
//! is capped at [`MAX_BYTES`]; when full it is rolled to `.1`..`.7` (oldest
//! dropped, like rusty-engine). Use `LLAMA_RS_LOG_DIR` to override the crate's
//! `target/live/logs` default.
//!
//! Explicit `--log-file <path>` on the CLI still wins and bypasses rotation
//! (keeps the historical hidden-run workflow intact).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Rotate when active file reaches this size (4 MiB).
pub const MAX_BYTES: u64 = 4 * 1024 * 1024;
/// Rolled files to keep.
pub const KEEP: usize = 7;

/// Sink directory: env wins, else crate's `target/live/logs`.
pub fn log_dir() -> PathBuf {
    std::env::var_os("LLAMA_RS_LOG_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/live/logs"))
}

fn roll(dir: &std::path::Path, active: &std::path::Path, name: &str) {
    let _ = fs::remove_file(dir.join(format!("{name}.log.{KEEP}")));
    for i in (1..KEEP).rev() {
        let from = dir.join(format!("{name}.log.{i}"));
        let to = dir.join(format!("{name}.log.{}", i + 1));
        if from.exists() {
            let _ = fs::rename(from, to);
        }
    }
    let _ = fs::rename(active, dir.join(format!("{name}.log.1")));
}

/// Open the rolling sink (rolling over when big) as a plain append handle.
pub fn rolling_open(name: &str) -> Option<std::fs::File> {
    let dir = log_dir();
    fs::create_dir_all(&dir).ok()?;
    let active = dir.join(format!("{name}.log"));
    if let Ok(md) = fs::metadata(&active) {
        if md.len() >= MAX_BYTES {
            roll(&dir, &active, name);
        }
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&active)
        .ok()
}

fn epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Append `line` to the rolling sink for `name`; explicit `--log-file` path
/// wins when provided (plain append, no rotation — legacy behavior).
pub fn log_line(explicit: Option<&str>, name: &str, msg: &str) {
    let line = format!("[{}] {msg}\n", epoch());
    if let Some(p) = explicit {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(p) {
            let _ = f.write_all(line.as_bytes());
            return;
        }
        eprintln!("warning: explicit log-file {p} unavailable — falling back to rolling");
    }
    match rolling_open(name) {
        Some(mut f) => {
            let _ = f.write_all(line.as_bytes());
        }
        None => eprintln!("{}", line.trim_end()),
    }
}
