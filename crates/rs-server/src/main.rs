mod auth_extractor;
mod error;
mod router;
mod routes;
mod state;
mod tls;

#[cfg(test)]
mod server_test;

use axum_server::tls_rustls::RustlsConfig;
use rs_common::{Config, connect_pool, run_migrations};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::router::build_router;
use crate::state::AppState;
use crate::tls::{ServeMode, resolve_serve_mode};

/// Entry point for the rustspeak server binary.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Install ring as the default rustls CryptoProvider (both ring and aws-lc-rs exist in workspace deps).
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli_insecure = std::env::args().any(|arg| arg == "--insecure-dev");
    let config = Config::from_env()?;
    let serve_mode = resolve_serve_mode(&config, cli_insecure)?;

    let pool = connect_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let state = AppState::new(pool, &config.jwt_secret);
    state.text_service.seed_default_channel().await?;
    let app = build_router(state);

    match serve_mode {
        ServeMode::Tls {
            cert_path,
            key_path,
        } => {
            let tls_config = RustlsConfig::from_pem_file(&cert_path, &key_path).await?;
            tracing::info!(
                bind_addr = %config.bind_addr,
                cert = %cert_path.display(),
                "rustspeak server listening with TLS"
            );
            axum_server::bind_rustls(config.bind_addr, tls_config)
                .serve(app.into_make_service())
                .await?;
        }
        ServeMode::InsecureHttp => {
            tracing::warn!(
                bind_addr = %config.bind_addr,
                "running in insecure HTTP mode without TLS"
            );
            let listener = TcpListener::bind(config.bind_addr).await?;
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}
