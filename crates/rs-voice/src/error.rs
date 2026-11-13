use thiserror::Error;

/// Errors returned by the voice and SFU domain.
#[derive(Debug, Error)]
pub enum VoiceError {
    /// Invalid voice channel name or signaling payload.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Voice channel name already exists.
    #[error("voice channel name already exists")]
    ChannelNameTaken,

    /// Voice channel was not found.
    #[error("voice channel not found")]
    NotFound,

    /// Caller lacks permission for the requested action.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Database query failure.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Underlying common error.
    #[error(transparent)]
    Common(#[from] rs_common::CommonError),
}
