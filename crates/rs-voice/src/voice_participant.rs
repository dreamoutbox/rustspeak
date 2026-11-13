use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A participant in a voice channel with their current microphone and deafen state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceParticipant {
    /// Participant's user ID.
    pub user_id: Uuid,
    /// Whether the participant's microphone is muted (or unavailable).
    pub muted: bool,
    /// Whether the participant is deafened.
    pub deafened: bool,
}
