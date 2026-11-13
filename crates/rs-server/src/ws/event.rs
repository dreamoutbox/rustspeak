use rs_text::{Message, TextChannel};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Outbound realtime event sent from the server to WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// Initial handshake payload with the authenticated user and online user IDs.
    Ready {
        user_id: Uuid,
        online_user_ids: Vec<Uuid>,
    },
    /// A new message was posted to a text channel.
    MessageCreated { message: Message },
    /// A text channel was created.
    ChannelCreated { channel: TextChannel },
    /// A text channel was deleted.
    ChannelDeleted { channel_id: Uuid },
    /// A user's online presence changed.
    PresenceUpdated { user_id: Uuid, online: bool },
    /// Response to a client ping.
    Pong,
}

/// Inbound command sent from a WebSocket client to the server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    /// Application-level heartbeat ping.
    Ping,
}
