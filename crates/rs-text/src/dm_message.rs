use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A persisted message inside a 1:1 direct message conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct DmMessage {
    /// Unique message identifier (UUIDv7, time-ordered).
    pub id: Uuid,
    /// Conversation containing this message.
    pub conversation_id: Uuid,
    /// Author's user ID.
    pub author_id: Uuid,
    /// Author's username at read time.
    pub author_username: String,
    /// Message text body.
    pub content: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
