mod auth_extractor;
mod error;
mod router;
mod routes;
mod state;

#[cfg(test)]
mod server_test;

use rs_common::{Config, connect_pool, run_migrations};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::router::build_router;
use crate::state::AppState;

/// Entry point for the rustspeak server binary.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;
    let pool = connect_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let state = AppState::new(pool, &config.jwt_secret);
    let app = build_router(state);

    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!(bind_addr = %config.bind_addr, "rustspeak server listening");
    axum::serve(listener, app).await?;

    Ok(())
}
