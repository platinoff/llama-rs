//! Local-address resolver for LAN-first flows (band 233 kit parity — mirrors
//! `gsv::net::local_addr`). Peers that do not share this box's loopback
//! (VMs, phones, edge PCs) address the services by the machine's local LAN
//! address instead of `127.0.0.1`.
//!
//! Order: `GSV_LOCAL_ADDR` env → `127.0.0.1` under the cargo-test harness
//! (deterministic contracts) → default-route source IP (UDP connect trick,
//! no packets sent) → loopback fallback.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

/// Whether the running exe is a rustc test-harness artifact (`deps/`).
pub fn is_cargo_test_harness() -> bool {
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/").contains("/deps/"))
        .unwrap_or(false)
}

/// Env form of the override: trimmed non-empty wins; blank is unset.
/// Kept pure so unit tests never race other tests via `set_var`.
fn from_env_value(v: Option<String>) -> Option<String> {
    let t = v?.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// This machine's local (LAN) address as a bare host string.
pub fn local_addr() -> String {
    if let Some(v) = from_env_value(std::env::var("GSV_LOCAL_ADDR").ok()) {
        return v;
    }
    if is_cargo_test_harness() {
        return "127.0.0.1".to_string();
    }
    default_route_addr().unwrap_or_else(|| "127.0.0.1".to_string())
}

fn default_route_addr() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:53").ok()?;
    match sock.local_addr().ok()? {
        SocketAddr::V4(v4) if v4.ip().is_loopback() => None,
        SocketAddr::V4(v4) => Some(v4.ip().to_string()),
        SocketAddr::V6(v6) if v6.ip().is_loopback() => None,
        SocketAddr::V6(v6) => Some(v6.ip().to_string()),
    }
}

/// `http://{local_addr}:{port}` — service base URL for peers.
pub fn http_base(port: u16) -> String {
    format!("http://{}:{port}", local_addr())
}

/// GSV live ingest target: `GSV_LIVE_URL` env (full `http://host:port`) else
/// the local-address form of `:9999`.
pub fn gsv_live_url() -> String {
    from_env_value(std::env::var("GSV_LIVE_URL").ok()).unwrap_or_else(|| http_base(9999))
}

/// Authority (`host:port`) of an `http://host:port/...` URL.
pub fn url_authority(url: &str) -> String {
    let rest = url.split_once("://").map(|x| x.1).unwrap_or(url);
    rest.split('/').next().unwrap_or(rest).to_string()
}

/// TCP-connect a URL's authority with a timeout (resolver for both bins).
pub fn connect_authority(url: &str, timeout: Duration) -> Option<TcpStream> {
    let auth = url_authority(url);
    let addr = auth.to_socket_addrs().ok()?.next()?;
    TcpStream::connect_timeout(&addr, timeout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_form_trims_and_rejects_blank() {
        assert_eq!(from_env_value(None), None);
        assert_eq!(from_env_value(Some("  ".into())), None);
        assert_eq!(
            from_env_value(Some(" 10.20.30.40 ".into())),
            Some("10.20.30.40".into())
        );
    }

    #[test]
    fn harness_defaults_are_loopback_and_urls_are_wellformed() {
        if from_env_value(std::env::var("GSV_LOCAL_ADDR").ok()).is_some() {
            assert!(!local_addr().is_empty());
            return;
        }
        assert_eq!(local_addr(), "127.0.0.1");
        assert_eq!(http_base(8080), "http://127.0.0.1:8080");
        std::env::set_var("GSV_LIVE_URL", "http://10.0.0.5:9999/");
        assert_eq!(url_authority(&gsv_live_url()), "10.0.0.5:9999");
        std::env::remove_var("GSV_LIVE_URL");
        assert_eq!(url_authority("http://127.0.0.1:9999"), "127.0.0.1:9999");
        assert_eq!(url_authority("127.0.0.1:9999"), "127.0.0.1:9999");
    }
}
