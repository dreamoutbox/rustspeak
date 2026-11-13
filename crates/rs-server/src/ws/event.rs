use rs_text::{DmMessage, Message, TextChannel};
use rs_voice::VoiceParticipant;
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
    /// A new 1:1 direct message was sent.
    DmCreated { message: DmMessage },
    /// A text channel was created.
    ChannelCreated { channel: TextChannel },
    /// A text channel was deleted.
    ChannelDeleted { channel_id: Uuid },
    /// A user's online presence changed.
    PresenceUpdated { user_id: Uuid, online: bool },
    /// Updated participant list for a voice channel.
    #[serde(rename = "voice.participants")]
    VoiceParticipants {
        channel_id: Uuid,
        user_ids: Vec<Uuid>,
        participants: Vec<VoiceParticipant>,
    },
    /// WebRTC SDP offer from the SFU to the client.
    #[serde(rename = "voice.offer")]
    VoiceOffer { sdp: String },
    /// WebRTC SDP answer from the SFU to the client.
    #[serde(rename = "voice.answer")]
    VoiceAnswer { sdp: String },
    /// WebRTC ICE candidate from the SFU to the client.
    #[serde(rename = "voice.ice")]
    VoiceIce { candidate: serde_json::Value },
    /// Response to a client ping.
    Pong,
}

/// Inbound command sent from a WebSocket client to the server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    /// Application-level heartbeat ping.
    Ping,
    /// Join a voice channel.
    #[serde(rename = "voice.join")]
    VoiceJoin {
        channel_id: Uuid,
        #[serde(default)]
        muted: bool,
        #[serde(default)]
        deafened: bool,
    },
    /// Update current voice mute/deafen state.
    #[serde(rename = "voice.state")]
    VoiceState { muted: bool, deafened: bool },
    /// Leave the current voice channel.
    #[serde(rename = "voice.leave")]
    VoiceLeave,
    /// Client SDP offer for WebRTC negotiation.
    #[serde(rename = "voice.offer")]
    VoiceOffer { sdp: String },
    /// Client SDP answer for WebRTC renegotiation.
    #[serde(rename = "voice.answer")]
    VoiceAnswer { sdp: String },
    /// Client ICE candidate.
    #[serde(rename = "voice.ice")]
    VoiceIce { candidate: serde_json::Value },
}
