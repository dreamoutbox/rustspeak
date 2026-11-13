use thiserror::Error;

/// Errors returned by the text messaging domain.
#[derive(Debug, Error)]
pub enum TextError {
    /// Invalid channel name, topic, or message content.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// A channel with the given name already exists.
    #[error("channel name already exists")]
    ChannelNameTaken,

    /// Requested channel or conversation was not found.
    #[error("channel not found")]
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
