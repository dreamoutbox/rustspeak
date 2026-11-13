use thiserror::Error;

/// Shared error type for common utilities and configuration.
#[derive(Debug, Error)]
pub enum CommonError {
    /// Configuration value is missing or invalid.
    #[error("invalid configuration: {0}")]
    Config(String),
}
