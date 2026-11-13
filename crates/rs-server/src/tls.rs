use std::path::PathBuf;

use anyhow::bail;
use rs_common::Config;

/// Transport mode for the HTTP/HTTPS server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServeMode {
    /// Serve native TLS via rustls using the given certificate and key PEM paths.
    Tls {
        cert_path: PathBuf,
        key_path: PathBuf,
    },
    /// Serve plain HTTP without TLS (local development only).
    InsecureHttp,
}

/// Resolves whether the server should bind TLS or insecure HTTP based on `config` and `cli_insecure`.
pub fn resolve_serve_mode(config: &Config, cli_insecure: bool) -> anyhow::Result<ServeMode> {
    if cli_insecure || config.insecure_dev {
        return Ok(ServeMode::InsecureHttp);
    }

    let (Some(cert_path), Some(key_path)) = (&config.tls_cert_path, &config.tls_key_path) else {
        bail!(
            "TLS_CERT_PATH and TLS_KEY_PATH must be set (or pass --insecure-dev / INSECURE_DEV=true for local HTTP)"
        );
    };

    if !cert_path.exists() {
        bail!("TLS certificate file not found: {}", cert_path.display());
    }
    if !key_path.exists() {
        bail!("TLS private key file not found: {}", key_path.display());
    }

    Ok(ServeMode::Tls {
        cert_path: cert_path.clone(),
        key_path: key_path.clone(),
    })
}
