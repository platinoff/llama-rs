//! xtask — pure Rust task runner for llama-rs (replaces shell/YAML where logical).
//! Usage: `cargo xtask <task>` where task = check | fmt | clippy | test | loc | sizing | help
//! No Python/Java, no extra deps, delegates to `cargo`/`gsv-loc-audit` via `std::process`.

use std::env;
use std::process::{Command, ExitStatus};

fn run(cmd: &str, args: &[&str]) -> ExitStatus {
    eprintln!("$ {} {}", cmd, args.join(" "));
    let st = Command::new(cmd)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {cmd}: {e}"));
    if !st.success() {
        eprintln!("command failed: {} {:?} -> {}", cmd, args, st);
    }
    st
}

fn help() {
    println!(
        r#"xtask — llama-rs pure Rust task runner
Usage: cargo xtask <task>

Tasks:
  check   fmt --check + clippy --all-targets + test (full scan)
  fmt     cargo fmt
  clippy  cargo clippy --all-targets
  test    cargo test
  loc     gsv-loc-audit --stretch-96 (99.46% now)
  sizing  show SIZING.md staged table
  serve-install [MODEL] [PORT] [release|debug]  persist llama_serve hidden (HKCU Run, no cmd window)
  serve-uninstall                 remove the persisted llama_serve task
  edge-install [WORKER_ID] [COORDINATOR]  persist llama_edge hidden (HKCU Run, no cmd window)
  edge-uninstall                  remove the persisted llama_edge task
  help    this help

Examples:
  cargo xtask check
  cargo xtask loc
  cargo xtask test
  cargo xtask serve-install
  cargo xtask serve-install models/Qwen3.8-27B-UD-IQ2_XXS.gguf 8080
  cargo xtask edge-install edge-pc-01 http://127.0.0.1:8091
"#
    );
}

fn main() {
    let task = env::args().nth(1).unwrap_or_else(|| "help".to_string());
    let repo_root = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    // When run via `cargo xtask`, cwd is repo root; when run via `cargo run -p xtask`, also.
    // For `loc`, we need GSV manifest path; try sibling GSV.
    let gsv_manifest = r"S:/rust/GSV/Cargo.toml";
    let (ok, code) = match task.as_str() {
        "check" => {
            let a = run("cargo", &["fmt", "--", "--check"]);
            if !a.success() {
                (false, a.code().unwrap_or(1))
            } else {
                let b = run("cargo", &["clippy", "--all-targets"]);
                if !b.success() {
                    (false, b.code().unwrap_or(1))
                } else {
                    let c = run("cargo", &["test"]);
                    (c.success(), c.code().unwrap_or(0))
                }
            }
        }
        "fmt" => {
            let s = run("cargo", &["fmt"]);
            (s.success(), s.code().unwrap_or(0))
        }
        "clippy" => {
            let s = run("cargo", &["clippy", "--all-targets"]);
            (s.success(), s.code().unwrap_or(0))
        }
        "test" => {
            let s = run("cargo", &["test"]);
            (s.success(), s.code().unwrap_or(0))
        }
        "loc" => {
            // gsv-loc-audit --repo-root S:/rust/llama-rs --stretch-96
            let s = run(
                "cargo",
                &[
                    "run",
                    "--manifest-path",
                    gsv_manifest,
                    "--bin",
                    "gsv-loc-audit",
                    "--",
                    "--repo-root",
                    repo_root.to_str().unwrap_or("S:/rust/llama-rs"),
                    "--stretch-96",
                ],
            );
            (s.success(), s.code().unwrap_or(0))
        }
        "sizing" => {
            let path = repo_root.join("docs/SIZING.md");
            match std::fs::read_to_string(&path) {
                Ok(c) => {
                    println!("{}", c);
                    (true, 0)
                }
                Err(e) => {
                    eprintln!("failed to read {}: {}", path.display(), e);
                    (false, 1)
                }
            }
        }
        "serve-install" => {
            // Optional: cargo xtask serve-install [MODEL] [PORT] [release|debug]
            let rest: Vec<String> = env::args().skip(2).collect();
            let model = rest.first().map(String::as_str).unwrap_or("");
            let port = rest.get(1).map(String::as_str).unwrap_or("");
            let profile = rest.get(2).map(String::as_str).unwrap_or("");
            match serve_install(&repo_root, model, port, profile) {
                Ok(msg) => {
                    println!("{msg}");
                    (true, 0)
                }
                Err(e) => {
                    eprintln!("serve-install failed: {e}");
                    (false, 1)
                }
            }
        }
        "serve-uninstall" => match serve_uninstall() {
            Ok(msg) => {
                println!("{msg}");
                (true, 0)
            }
            Err(e) => {
                eprintln!("serve-uninstall failed: {e}");
                (false, 1)
            }
        },
        "edge-install" => {
            // Optional: cargo xtask edge-install [WORKER_ID] [COORDINATOR]
            let rest: Vec<String> = env::args().skip(2).collect();
            let worker = rest.first().map(String::as_str).unwrap_or("");
            let coord = rest.get(1).map(String::as_str).unwrap_or("");
            match edge_install(&repo_root, worker, coord) {
                Ok(msg) => {
                    println!("{msg}");
                    (true, 0)
                }
                Err(e) => {
                    eprintln!("edge-install failed: {e}");
                    (false, 1)
                }
            }
        }
        "edge-uninstall" => match edge_uninstall() {
            Ok(msg) => {
                println!("{msg}");
                (true, 0)
            }
            Err(e) => {
                eprintln!("edge-uninstall failed: {e}");
                (false, 1)
            }
        },
        "help" | "--help" | "-h" => {
            help();
            (true, 0)
        }
        other => {
            eprintln!("unknown task: {other}");
            help();
            (false, 2)
        }
    };
    if !ok {
        std::process::exit(code);
    }
}

/// Find a built `llama_serve` exe. `profile` forces release/debug;
/// otherwise debug first, then release.
fn serve_exe(repo_root: &std::path::Path, profile: &str) -> Option<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if profile == "release" {
        dirs.push("target/release/llama_serve.exe");
    } else if profile == "debug" {
        dirs.push("target/debug/llama_serve.exe");
    } else {
        dirs.push("target/debug/llama_serve.exe");
        dirs.push("target/release/llama_serve.exe");
    }
    for dir in dirs {
        let p = repo_root.join(dir);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// MinGW runtime DLLs the exe needs beside itself (hidden/logon launches have
/// no ucrt64 on PATH; the loader checks the exe dir first). Missing DLL =
/// instant 0xC0000135 with no window and no log.
const SERVE_DLLS: [&str; 4] = [
    "libgomp-1.dll",
    "libstdc++-6.dll",
    "libgcc_s_seh-1.dll",
    "libwinpthread-1.dll",
];

/// Copy missing runtime DLLs from a local MSYS2 ucrt64 install next to the exe.
fn ensure_serve_dlls(exe: &std::path::Path) -> Result<Vec<String>, String> {
    let dir = exe
        .parent()
        .ok_or_else(|| "serve exe has no parent dir".to_string())?;
    let mut done = Vec::new();
    for dll in SERVE_DLLS {
        if dir.join(dll).is_file() {
            continue;
        }
        let mut src: Option<std::path::PathBuf> = None;
        for root in [
            r"C:\msys64\ucrt64\bin",
            r"C:\tools\msys64\ucrt64\bin",
            r"S:\msys64\ucrt64\bin",
        ] {
            let cand = std::path::Path::new(root).join(dll);
            if cand.is_file() {
                src = Some(cand);
                break;
            }
        }
        let src = src
            .ok_or_else(|| format!("{dll} missing next to serve exe and no MSYS2 ucrt64 found"))?;
        std::fs::copy(&src, dir.join(dll)).map_err(|e| format!("copy {dll}: {e}"))?;
        done.push(dll.to_string());
    }
    Ok(done)
}

fn win_path(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('/', "\\")
}

/// Hidden task command with zero console windows on every path (schtasks
/// ONLOGON and HKCU Run): hidden powershell re-spawns the server hidden.
/// A bare `cmd /c` TR would still flash a console window from HKCU Run.
fn serve_task_tr(root: &std::path::Path, exe: &std::path::Path, model: &str, port: &str) -> String {
    let win_root = win_path(root);
    let win_exe = win_path(exe);
    let port = if port.is_empty() { "8080" } else { port };
    let model = if model.is_empty() {
        "models/Qwen3.8-27B-UD-IQ2_XXS.gguf".to_string()
    } else {
        model.replace('/', "\\")
    };
    let log = format!("{win_root}\\target\\live\\llama_serve.log");
    format!(
        "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe -WindowStyle Hidden -ExecutionPolicy Bypass -Command \"[Environment]::SetEnvironmentVariable('LLAMA_RS_HEARTBEAT','1','Process'); Start-Process -FilePath '{win_exe}' -ArgumentList '{model}','--port','{port}','--log-file','{log}' -WorkingDirectory '{win_root}' -WindowStyle Hidden\""
    )
}

/// Persist `llama_serve` across reboot (current user, no cmd window).
/// Prefers schtasks ONLOGON, falls back to HKCU Run (same mirror as GSV watchdog).
fn serve_install(
    repo_root: &std::path::Path,
    model: &str,
    port: &str,
    profile: &str,
) -> Result<String, String> {
    let exe = serve_exe(repo_root, profile)
        .ok_or_else(|| "no built llama_serve (run: cargo build --bin llama_serve)".to_string())?;
    let dlls = ensure_serve_dlls(&exe)?;
    if !dlls.is_empty() {
        println!("serve-install: staged runtime DLLs: {}", dlls.join(", "));
    }
    let tr = serve_task_tr(repo_root, &exe, model, port);
    if try_schtasks(&tr) {
        return Ok(format!(
            "llama-serve-install: schtasks llama-serve (ONLOGON, hidden)\nTR={tr}"
        ));
    }
    if try_hkcu_run(&tr) {
        return Ok(format!(
            "llama-serve-install: HKCU Run llama-serve (hidden)\nTR={tr}"
        ));
    }
    Err("could not persist (need schtasks or reg.exe)".into())
}

fn serve_uninstall() -> Result<String, String> {
    let mut notes = Vec::new();
    let st = Command::new(r"C:\Windows\System32\schtasks.exe")
        .args(["/Delete", "/TN", "llama-serve", "/F"])
        .status()
        .map_err(|e| format!("schtasks: {e}"))?;
    notes.push(format!("schtasks delete: {}", st.success()));
    let reg = Command::new(r"C:\Windows\System32\reg.exe")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "llama-serve",
            "/f",
        ])
        .status()
        .map_err(|e| format!("reg: {e}"))?;
    notes.push(format!("hkcu run delete: {}", reg.success()));
    Ok(format!("llama-serve-uninstall: {}", notes.join(", ")))
}

/// Hidden task command for the edge executor (same zero-window pattern).
fn edge_task_tr(
    root: &std::path::Path,
    exe: &std::path::Path,
    worker: &str,
    coord: &str,
) -> String {
    let win_root = win_path(root);
    let win_exe = win_path(exe);
    let worker = if worker.is_empty() {
        "edge-pc-01"
    } else {
        worker
    };
    let coord = if coord.is_empty() {
        "http://127.0.0.1:8091"
    } else {
        coord
    };
    let log = format!("{win_root}\\target\\live\\llama_edge.log");
    format!(
        "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe -WindowStyle Hidden -ExecutionPolicy Bypass -Command \"Start-Process -FilePath '{win_exe}' -ArgumentList '--coordinator','{coord}','--worker-id','{worker}','--log-file','{log}' -WorkingDirectory '{win_root}' -WindowStyle Hidden\""
    )
}

/// Persist `llama_edge` across reboot (current user, no cmd window).
/// Find a built `llama_edge` exe (debug first, then release).
fn edge_exe(repo_root: &std::path::Path) -> Option<std::path::PathBuf> {
    for dir in [
        "target/debug/llama_edge.exe",
        "target/release/llama_edge.exe",
    ] {
        let p = repo_root.join(dir);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn edge_install(repo_root: &std::path::Path, worker: &str, coord: &str) -> Result<String, String> {
    let exe = edge_exe(repo_root)
        .ok_or_else(|| "no built llama_edge (run: cargo build --bin llama_edge)".to_string())?;
    let dlls = ensure_serve_dlls(&exe)?;
    if !dlls.is_empty() {
        println!("edge-install: staged runtime DLLs: {}", dlls.join(", "));
    }
    let tr = edge_task_tr(repo_root, &exe, worker, coord);
    if try_schtasks_edge(&tr) {
        return Ok(format!(
            "llama-edge-install: schtasks llama-edge (ONLOGON, hidden)\nTR={tr}"
        ));
    }
    if try_hkcu_run_edge(&tr) {
        return Ok(format!(
            "llama-edge-install: HKCU Run llama-edge (hidden)\nTR={tr}"
        ));
    }
    Err("could not persist (need schtasks or reg.exe)".into())
}

fn edge_uninstall() -> Result<String, String> {
    let mut notes = Vec::new();
    let st = Command::new(r"C:\Windows\System32\schtasks.exe")
        .args(["/Delete", "/TN", "llama-edge", "/F"])
        .status()
        .map_err(|e| format!("schtasks: {e}"))?;
    notes.push(format!("schtasks delete: {}", st.success()));
    let reg = Command::new(r"C:\Windows\System32\reg.exe")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "llama-edge",
            "/f",
        ])
        .status()
        .map_err(|e| format!("reg: {e}"))?;
    notes.push(format!("hkcu run delete: {}", reg.success()));
    Ok(format!("llama-edge-uninstall: {}", notes.join(", ")))
}

fn try_schtasks_edge(tr: &str) -> bool {
    Command::new(r"C:\Windows\System32\schtasks.exe")
        .args([
            "/Create",
            "/TN",
            "llama-edge",
            "/SC",
            "ONLOGON",
            "/RL",
            "LIMITED",
            "/F",
            "/TR",
            tr,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn try_hkcu_run_edge(tr: &str) -> bool {
    Command::new(r"C:\Windows\System32\reg.exe")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "llama-edge",
            "/t",
            "REG_SZ",
            "/d",
            tr,
            "/f",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn try_schtasks(tr: &str) -> bool {
    Command::new(r"C:\Windows\System32\schtasks.exe")
        .args([
            "/Create",
            "/TN",
            "llama-serve",
            "/SC",
            "ONLOGON",
            "/RL",
            "LIMITED",
            "/F",
            "/TR",
            tr,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn try_hkcu_run(tr: &str) -> bool {
    Command::new(r"C:\Windows\System32\reg.exe")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "llama-serve",
            "/t",
            "REG_SZ",
            "/d",
            tr,
            "/f",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
