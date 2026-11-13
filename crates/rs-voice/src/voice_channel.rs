use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A server voice channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct VoiceChannel {
    /// Unique voice channel identifier (UUIDv7).
    pub id: Uuid,
    /// Display name (e.g. `Lobby`).
    pub name: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
