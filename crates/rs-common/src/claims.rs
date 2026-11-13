use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT access token claims shared across workspace crates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Authenticated user's unique ID.
    pub sub: Uuid,
    /// Authenticated user's username.
    pub username: String,
    /// Issued-at timestamp (Unix seconds).
    pub iat: usize,
    /// Expiration timestamp (Unix seconds).
    pub exp: usize,
}

