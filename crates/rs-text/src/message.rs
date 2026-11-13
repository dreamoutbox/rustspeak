use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A persisted text message in a channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    /// Unique message identifier (UUIDv7, time-ordered).
    pub id: Uuid,
    /// Channel containing this message.
    pub channel_id: Uuid,
    /// Author's user ID.
    pub author_id: Uuid,
    /// Author's username at read time.
    pub author_username: String,
    /// Message text body.
    pub content: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
