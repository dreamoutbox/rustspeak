use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A server text channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct TextChannel {
    /// Unique channel identifier (UUIDv7).
    pub id: Uuid,
    /// Channel name (e.g. `general`).
    pub name: String,
    /// Optional channel topic.
    pub topic: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
