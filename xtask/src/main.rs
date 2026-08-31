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
  help    this help

Examples:
  cargo xtask check
  cargo xtask loc
  cargo xtask test
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
