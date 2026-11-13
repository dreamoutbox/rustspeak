use chrono::{DateTime, Utc};
use rs_common::Role;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Public user profile returned by authentication and user endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    /// Unique user identifier (UUIDv7).
    pub id: Uuid,
    /// Display and login username.
    pub username: String,
    /// Global role of the user.
    pub role: Role,
    /// Whether the account is disabled.
    pub disabled: bool,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
}
