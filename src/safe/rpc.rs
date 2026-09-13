//! GGML RPC workers (distributed inference) — coordinator side.
//!
//! Protocol (`llama.cpp/tools/rpc/README.md`): each worker runs
//! `ggml-rpc-server` (phones via Termux, Raspberry Pi, spare PCs — **LAN
//! only**, the RPC transport is plaintext and upstream marks it
//! proof-of-concept). The coordinator registers every worker with
//! `ggml_backend_rpc_add_server("host:port")` **after** backend init and
//! **before** model load; model weights and the KV cache then spread over
//! local + remote devices proportionally to free memory.
//!
//! Build gate: the single FFI symbol below links only when the vendored
//! `llama-cpp-sys-2` was built with `GGML_RPC=ON`. That build script forwards
//! any `GGML_*` env var to CMake, so no registry patch is needed — but the
//! call itself lives behind cargo feature `rpc`, keeping default builds
//! (and their link) untouched:
//!
//! ```text
//! GGML_RPC=ON cargo build --bin llama_serve --features rpc
//! cargo run --bin llama_serve --features rpc -- model.gguf --rpc 192.168.1.10:50052
//! ```
//!
//! Control-plane shape (register → heartbeat with capabilities → poll) mirrors
//! poolAI's `poolai-worker.rs` (`register-remote`, `heartbeat-remote`,
//! `pool/join`); tensor offload itself must go through ggml RPC — an HTTP
//! task split cannot parallelize autoregressive decode.

/// Validate + normalize one `--rpc` endpoint (`host:port`, port 1–65535).
pub fn parse_endpoint(s: &str) -> Result<String, String> {
    let s = s.trim();
    let (host, port) = s
        .rsplit_once(':')
        .ok_or_else(|| format!("bad --rpc endpoint (want host:port): {s}"))?;
    let host = host.trim();
    if host.is_empty() {
        return Err(format!("bad --rpc endpoint (empty host): {s}"));
    }
    let port: u16 = port
        .trim()
        .parse()
        .map_err(|_| format!("bad --rpc endpoint (bad port): {s}"))?;
    if port == 0 {
        return Err(format!("bad --rpc endpoint (port 0): {s}"));
    }
    Ok(format!("{host}:{port}"))
}

#[cfg(feature = "rpc")]
mod imp {
    use std::ffi::CString;

    // ggml-rpc.h (linked when GGML_RPC=ON; declared here so the registry
    // wrapper.h needs no edit): registers one worker, returns its reg.
    unsafe extern "C" {
        fn ggml_backend_rpc_add_server(endpoint: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    }

    pub(super) fn register(endpoints: &[String]) -> Result<usize, String> {
        let mut added = 0;
        for ep in endpoints {
            let c = CString::new(ep.as_str()).map_err(|e| format!("bad endpoint {ep}: {e}"))?;
            // NOTE: must run after Backend::init, before Model load.
            let reg = unsafe { ggml_backend_rpc_add_server(c.as_ptr()) };
            if reg.is_null() {
                return Err(format!("rpc worker refused: {ep}"));
            }
            added += 1;
        }
        Ok(added)
    }
}

/// Register validated RPC workers. Returns how many were added.
/// Without cargo feature `rpc`, always errors with the enable recipe.
pub fn register_servers(endpoints: &[String]) -> Result<usize, String> {
    #[cfg(feature = "rpc")]
    {
        imp::register(endpoints)
    }
    #[cfg(not(feature = "rpc"))]
    {
        let _ = endpoints;
        Err("rpc disabled: rebuild with GGML_RPC=ON cargo build --features rpc".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_accepts_host_port() {
        assert_eq!(
            parse_endpoint("192.168.1.10:50052").unwrap(),
            "192.168.1.10:50052"
        );
        assert_eq!(
            parse_endpoint("  phone-lan:50052 ").unwrap(),
            "phone-lan:50052"
        );
    }

    #[test]
    fn endpoint_rejects_garbage() {
        assert!(parse_endpoint("no-port").is_err());
        assert!(parse_endpoint(":50052").is_err());
        assert!(parse_endpoint("host:abc").is_err());
        assert!(parse_endpoint("host:0").is_err());
        assert!(parse_endpoint("host:99999").is_err());
    }

    #[test]
    fn register_without_feature_explains_itself() {
        #[cfg(not(feature = "rpc"))]
        assert!(register_servers(&["h:1".to_string()]).is_err());
    }
}
