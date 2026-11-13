use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Role;

/// JWT access token claims shared across workspace crates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Authenticated user's unique ID.
    pub sub: Uuid,
    /// Authenticated user's username.
    pub username: String,
    /// Authenticated user's global role.
    pub role: Role,
    /// Issued-at timestamp (Unix seconds).
    pub iat: usize,
    /// Expiration timestamp (Unix seconds).
    pub exp: usize,
}
