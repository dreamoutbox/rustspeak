use serde::{Deserialize, Serialize};

/// Global user role ordered by privilege (`Member < Admin < Owner`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum Role {
    /// Standard server member.
    Member,
    /// Server administrator (manages channels and members).
    Admin,
    /// Server owner (first registered user; manages admins).
    Owner,
}
