use std::net::SocketAddr;
use std::path::PathBuf;

use crate::CommonError;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8443";
const DEFAULT_ICE_BIND_IP: &str = "127.0.0.1";
const DEFAULT_ICE_UDP_PORT_MIN: u16 = 40000;
const DEFAULT_ICE_UDP_PORT_MAX: u16 = 40100;
const DEFAULT_STATIC_DIR: &str = "web/dist";
const MIN_JWT_SECRET_LEN: usize = 32;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// PostgreSQL connection string (`DATABASE_URL`).
    pub database_url: String,
    /// Socket address to bind the HTTP/HTTPS server (`BIND_ADDR`).
    pub bind_addr: SocketAddr,
    /// HMAC secret for signing JWT access tokens (`JWT_SECRET`).
    pub jwt_secret: String,
    /// Path to the TLS certificate PEM file (`TLS_CERT_PATH`).
    pub tls_cert_path: Option<PathBuf>,
    /// Path to the TLS private key PEM file (`TLS_KEY_PATH`).
    pub tls_key_path: Option<PathBuf>,
    /// Explicit opt-in to run without TLS in local development (`INSECURE_DEV`).
    pub insecure_dev: bool,
    /// IP address to bind WebRTC ICE UDP sockets (`ICE_BIND_IP`).
    pub ice_bind_ip: String,
    /// Optional STUN server URL for WebRTC ICE (`STUN_URL`).
    pub stun_url: Option<String>,
    /// Inclusive lower bound of the UDP port range for WebRTC ICE (`ICE_UDP_PORT_MIN`).
    pub ice_udp_port_min: u16,
    /// Inclusive upper bound of the UDP port range for WebRTC ICE (`ICE_UDP_PORT_MAX`).
    pub ice_udp_port_max: u16,
    /// Directory containing built frontend static assets (`STATIC_DIR`).
    pub static_dir: PathBuf,
}

impl Config {
    /// Loads configuration from process environment variables.
    ///
    /// # Example
    /// ```no_run
    /// let config = rs_common::Config::from_env()?;
    /// # Ok::<(), rs_common::CommonError>(())
    /// ```
    pub fn from_env() -> Result<Self, CommonError> {
        Self::from_env_with(|key| std::env::var(key).ok())
    }

    /// Loads configuration using a custom lookup function.
    ///
    /// `get` receives an environment variable name and returns `Some(value)` if set.
    pub fn from_env_with<F>(get: F) -> Result<Self, CommonError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let database_url = required_non_empty(&get, "DATABASE_URL")?;
        let jwt_secret = required_non_empty(&get, "JWT_SECRET")?;
        if jwt_secret.len() < MIN_JWT_SECRET_LEN {
            return Err(CommonError::Config(format!(
                "JWT_SECRET must be at least {MIN_JWT_SECRET_LEN} bytes"
            )));
        }

        let bind_addr_raw = get("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string());
        let bind_addr: SocketAddr = bind_addr_raw
            .parse()
            .map_err(|err| CommonError::Config(format!("invalid BIND_ADDR: {err}")))?;

        let tls_cert_path = non_empty_path(&get, "TLS_CERT_PATH");
        let tls_key_path = non_empty_path(&get, "TLS_KEY_PATH");
        let insecure_dev = parse_bool_env(&get, "INSECURE_DEV", false)?;

        let ice_bind_ip = get("ICE_BIND_IP")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_ICE_BIND_IP.to_string());
        let stun_url = get("STUN_URL")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let ice_udp_port_min = parse_u16_env(&get, "ICE_UDP_PORT_MIN", DEFAULT_ICE_UDP_PORT_MIN)?;
        let ice_udp_port_max = parse_u16_env(&get, "ICE_UDP_PORT_MAX", DEFAULT_ICE_UDP_PORT_MAX)?;

        if ice_udp_port_min == 0 || ice_udp_port_min > ice_udp_port_max {
            return Err(CommonError::Config(format!(
                "invalid ICE UDP port range: {ice_udp_port_min}..={ice_udp_port_max}"
            )));
        }

        let static_dir =
            non_empty_path(&get, "STATIC_DIR").unwrap_or_else(|| PathBuf::from(DEFAULT_STATIC_DIR));

        Ok(Self {
            database_url,
            bind_addr,
            jwt_secret,
            tls_cert_path,
            tls_key_path,
            insecure_dev,
            ice_bind_ip,
            stun_url,
            ice_udp_port_min,
            ice_udp_port_max,
            static_dir,
        })
    }
}

fn required_non_empty<F>(get: &F, key: &str) -> Result<String, CommonError>
where
    F: Fn(&str) -> Option<String>,
{
    match get(key) {
        Some(val) if !val.trim().is_empty() => Ok(val),
        _ => Err(CommonError::Config(format!("{key} must be set"))),
    }
}

fn non_empty_path<F>(get: &F, key: &str) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<String>,
{
    get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn parse_u16_env<F>(get: &F, key: &str, default: u16) -> Result<u16, CommonError>
where
    F: Fn(&str) -> Option<String>,
{
    match get(key) {
        Some(raw) if !raw.trim().is_empty() => raw
            .trim()
            .parse::<u16>()
            .map_err(|err| CommonError::Config(format!("invalid {key}: {err}"))),
        _ => Ok(default),
    }
}

fn parse_bool_env<F>(get: &F, key: &str, default: bool) -> Result<bool, CommonError>
where
    F: Fn(&str) -> Option<String>,
{
    match get(key) {
        Some(raw) if !raw.trim().is_empty() => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Ok(true),
            "0" | "false" | "no" => Ok(false),
            _ => Err(CommonError::Config(format!("invalid {key}: {raw}"))),
        },
        _ => Ok(default),
    }
}
