use thiserror::Error;

/// Shared error type for common utilities, configuration, and database setup.
#[derive(Debug, Error)]
pub enum CommonError {
    /// Configuration value is missing or invalid.
    #[error("invalid configuration: {0}")]
    Config(String),

    /// Database connection or query failure.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Database migration failure.
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}
