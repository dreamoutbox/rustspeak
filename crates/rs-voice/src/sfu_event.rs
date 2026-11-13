use uuid::Uuid;

/// Outbound WebRTC signaling events emitted by `SfuEngine` toward a specific user.
#[derive(Debug, Clone)]
pub enum SfuEvent {
    /// Renegotiation SDP offer from the SFU to `user_id`.
    Offer { user_id: Uuid, sdp: String },
    /// SDP answer from the SFU to `user_id`.
    Answer { user_id: Uuid, sdp: String },
    /// Trickle ICE candidate JSON from the SFU to `user_id`.
    IceCandidate {
        user_id: Uuid,
        candidate: serde_json::Value,
    },
}
