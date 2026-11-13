use rs_common::{Config, connect_pool, run_migrations};
use tracing_subscriber::EnvFilter;

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

    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await?;
    tracing::info!(
        bind_addr = %config.bind_addr,
        db_check = row.0,
        "rustspeak server database migrated and ready"
    );

    Ok(())
}
