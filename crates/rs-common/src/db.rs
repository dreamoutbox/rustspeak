use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::CommonError;

const DEFAULT_MAX_CONNECTIONS: u32 = 16;
const CONNECT_TIMEOUT_SECS: u64 = 5;

/// Connects a PostgreSQL connection pool for `database_url`.
///
/// # Example
/// ```no_run
/// # async fn run() -> Result<(), rs_common::CommonError> {
/// let pool = rs_common::connect_pool("postgres://localhost/rustspeak").await?;
/// # Ok(())
/// # }
/// ```
pub async fn connect_pool(database_url: &str) -> Result<PgPool, CommonError> {
    let pool = PgPoolOptions::new()
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .acquire_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Runs embedded SQLx migrations against `pool`.
pub async fn run_migrations(pool: &PgPool) -> Result<(), CommonError> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}

