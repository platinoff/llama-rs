//! xtask — pure Rust task runner for llama-rs (replaces shell/YAML where logical).
//! Usage: `cargo xtask <task>` where task = check | fmt | clippy | test | loc | sizing |
//! build-matrix | serve-install | edge-install | help
//! No Python/Java, no extra deps, delegates to `cargo`/`gsv-loc-audit` via `std::process`.

use std::env;
use std::process::{Command, ExitStatus};

/// One build target: cargo triple, OS, arch, artifact extension, notes.
struct Target {
    triple: &'static str,
    os: &'static str,
    arch: &'static str,
    ext: &'static str,
    note: &'static str,
}

/// The exe/bin/apk matrix (worker + provider role, 2026-10-16). llama-cpp-2 is
/// C++-linked, so cross builds need external toolchains (MSVC link.exe, cross
/// linkers, Android NDK) — `--build` attempts them and skips honestly.
const TARGETS: &[Target] = &[
    Target {
        triple: "x86_64-pc-windows-gnu",
        os: "windows",
        arch: "x86_64",
        ext: ".exe",
        note: "primary (rust-toolchain pin)",
    },
    Target {
        triple: "x86_64-pc-windows-msvc",
        os: "windows",
        arch: "x86_64",
        ext: ".exe",
        note: "needs MSVC link.exe",
    },
    Target {
        triple: "x86_64-unknown-linux-gnu",
        os: "linux",
        arch: "x86_64",
        ext: "",
        note: "Pi-class host sibling",
    },
    Target {
        triple: "aarch64-unknown-linux-gnu",
        os: "linux",
        arch: "aarch64",
        ext: "",
        note: "arm64 Linux",
    },
    Target {
        triple: "aarch64-linux-android",
        os: "android",
        arch: "arm64",
        ext: "",
        note: "apk/so, cargo-ndk/Termux, no Docker",
    },
];

/// Binaries shipped per target (provider + worker + core).
const BINS: &[&str] = &["llama_rs", "llama_serve", "llama_edge"];

/// SHA-256 (FIPS 180-4) of `data` as lowercase hex — pure `std`, no dependency.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut w = [0u32; 64];
    for chunk in msg.chunks_exact(64) {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h = [
            h[0].wrapping_add(a),
            h[1].wrapping_add(b),
            h[2].wrapping_add(c),
            h[3].wrapping_add(d),
            h[4].wrapping_add(e),
            h[5].wrapping_add(f),
            h[6].wrapping_add(g),
            h[7].wrapping_add(hh),
        ];
    }
    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{:08x}", word));
    }
    out
}

/// Print the OS×arch matrix and, for artifacts already present under
/// `target/<triple>/release/`, their SHA-256. With `do_build`, first attempt
/// `cargo build --release --target <triple>` for installed targets.
fn build_matrix(do_build: bool) {
    let installed = installed_targets();
    let host = host_triple();
    println!(
        "llama-rs build matrix ({} targets){}:",
        TARGETS.len(),
        if do_build { " — BUILD" } else { "" }
    );
    if do_build {
        println!(
            "  host {host}; installed targets: {}",
            if installed.is_empty() {
                "(rustup unavailable)".to_string()
            } else {
                installed.join(", ")
            }
        );
    }
    for t in TARGETS {
        println!("  {:26} {:7} {:8}  — {}", t.triple, t.os, t.arch, t.note);
        if do_build && !build_one(t, &host, &installed) {
            continue;
        }
        for bin in BINS {
            let artifact = format!("target/{}/release/{}{}", t.triple, bin, t.ext);
            match std::fs::read(&artifact) {
                Ok(bytes) => println!(
                    "    {:<12} sha256 {}  ({} bytes)",
                    bin,
                    sha256_hex(&bytes),
                    bytes.len()
                ),
                Err(_) => println!("    {:<12} (not built: {artifact})", bin),
            }
        }
    }
}

/// Build one matrix target. Returns `false` when skipped, so the caller omits
/// the artifact lines. Never aborts the matrix — a missing cross-linker is a
/// skip, not a fatal error.
fn build_one(t: &Target, host: &str, installed: &[String]) -> bool {
    let is_host = t.triple == host;
    if !is_host && !installed.iter().any(|x| x == t.triple) {
        println!(
            "    skipped: target not installed (rustup target add {})",
            t.triple
        );
        return false;
    }
    println!(
        "    building… (cargo build --release --target {})",
        t.triple
    );
    let ok = run_status(
        "cargo",
        &["build", "--release", "--target", t.triple, "--bins"],
    );
    if !ok {
        println!(
            "    skipped: link/build failed (needs {})",
            if t.triple.contains("android") {
                "Android NDK / cargo-ndk"
            } else if t.triple.contains("msvc") {
                "MSVC link.exe"
            } else {
                "a cross linker"
            }
        );
    }
    ok
}

/// `rustup target list --installed` (empty when rustup is unavailable).
fn installed_targets() -> Vec<String> {
    capture("rustup", &["target", "list", "--installed"])
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// `rustc -vV` → the `host: <triple>` line.
fn host_triple() -> String {
    capture("rustc", &["-vV"])
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("host: ").map(|h| h.trim().to_string()))
        })
        .unwrap_or_default()
}

fn capture(prog: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(prog).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

fn run_status(prog: &str, args: &[&str]) -> bool {
    Command::new(prog)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

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
  build-matrix [--build]  exe/bin/apk matrix; --build gated cross-builds + SHA-256
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
  cargo xtask edge-install edge-pc-01 http://<this-pc-lan-ip>:8091
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
        "build-matrix" => {
            let do_build = env::args().skip(2).any(|a| a == "--build");
            build_matrix(do_build);
            (true, 0)
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
    // Band 233: default coordinator sits on this machine's local address
    // (`GSV_LOCAL_ADDR` override, else default-route IPv4, else loopback) —
    // same rule as `llama_rs::net`, re-implemented here because xtask stays
    // dependency-free.
    let coord = if coord.is_empty() {
        let host = {
            let from_env = std::env::var("GSV_LOCAL_ADDR")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());
            from_env.unwrap_or_else(|| {
                std::net::UdpSocket::bind("0.0.0.0:0")
                    .ok()
                    .filter(|s| s.connect("8.8.8.8:53").is_ok())
                    .and_then(|s| s.local_addr().ok())
                    .map(|a| a.ip().to_string())
                    .unwrap_or_else(|| "127.0.0.1".to_string())
            })
        };
        format!("http://{host}:8091")
    } else {
        coord.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_nist_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn matrix_covers_exe_bin_apk() {
        let triples: Vec<&str> = TARGETS.iter().map(|t| t.triple).collect();
        assert!(triples.contains(&"x86_64-pc-windows-gnu"));
        assert!(triples.contains(&"x86_64-unknown-linux-gnu"));
        assert!(triples.contains(&"aarch64-unknown-linux-gnu"));
        assert!(triples.contains(&"aarch64-linux-android"));
        assert_eq!(TARGETS.iter().filter(|t| t.os == "android").count(), 1);
        // Worker + provider + core are all shipped.
        assert!(BINS.contains(&"llama_edge"));
        assert!(BINS.contains(&"llama_serve"));
    }

    #[test]
    fn capture_missing_program_is_none() {
        assert!(capture("definitely-not-a-real-program-llama-xyz", &[]).is_none());
    }
}
