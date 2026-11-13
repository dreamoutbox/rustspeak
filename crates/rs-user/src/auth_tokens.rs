use serde::{Deserialize, Serialize};

/// Access and refresh token pair issued upon registration, login, or refresh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthTokens {
    /// Short-lived JWT access token.
    pub access_token: String,
    /// Opaque rotating refresh token.
    pub refresh_token: String,
    /// Access token lifetime in seconds.
    pub expires_in_secs: u64,
}

