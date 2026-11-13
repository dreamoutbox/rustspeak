use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A 1:1 direct message conversation viewed from the perspective of the requesting user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct DmConversation {
    /// Unique conversation identifier (UUIDv7).
    pub id: Uuid,
    /// The other participant's user ID.
    pub peer_id: Uuid,
    /// The other participant's username.
    pub peer_username: String,
    /// Conversation creation timestamp.
    pub created_at: DateTime<Utc>,
}
