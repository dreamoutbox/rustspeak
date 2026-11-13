use thiserror::Error;

/// Errors returned by the user and authentication domain.
#[derive(Debug, Error)]
pub enum UserError {
    /// Validation failure on username, password, or other input.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Username is already registered.
    #[error("username is already taken")]
    UsernameTaken,

    /// Invalid username or password during login.
    #[error("invalid username or password")]
    InvalidCredentials,

    /// Access or refresh token is missing, malformed, or expired.
    #[error("invalid or expired token")]
    InvalidToken,

    /// A previously revoked refresh token was presented again.
    #[error("refresh token reuse detected")]
    TokenReused,

    /// Account has been disabled by an administrator.
    #[error("user account is disabled")]
    UserDisabled,

    /// Caller lacks permission for the requested action.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Requested user does not exist.
    #[error("user not found")]
    NotFound,

    /// Password hashing or verification failure.
    #[error("password hashing failed: {0}")]
    PasswordHash(String),

    /// JWT encoding failure.
    #[error("token encoding failed: {0}")]
    JwtEncode(#[from] jsonwebtoken::errors::Error),

    /// Database query failure.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Underlying common error.
    #[error(transparent)]
    Common(#[from] rs_common::CommonError),
}
