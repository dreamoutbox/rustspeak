use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::VoiceParticipant;

#[derive(Default)]
struct RoomState {
    rooms: HashMap<Uuid, HashMap<Uuid, (bool, bool)>>,
    user_room: HashMap<Uuid, Uuid>,
}

/// Result of joining a voice room, including any previous room left due to the one-room rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOutcome {
    /// Previous channel left automatically, if the user was already in another room.
    pub left_channel: Option<(Uuid, Vec<VoiceParticipant>)>,
    /// Channel joined and its updated participant list.
    pub joined_channel: (Uuid, Vec<VoiceParticipant>),
}

/// In-memory voice room membership registry enforcing at most one room per user.
#[derive(Clone, Default)]
pub struct RoomRegistry {
    state: Arc<RwLock<RoomState>>,
}

impl RoomRegistry {
    /// Creates an empty `RoomRegistry`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `user_id` to `channel_id` with default unmuted/undeafened state.
    pub async fn join(&self, user_id: Uuid, channel_id: Uuid) -> JoinOutcome {
        self.join_with_state(user_id, channel_id, false, false)
            .await
    }

    /// Adds `user_id` to `channel_id` with explicit `muted` and `deafened` state.
    pub async fn join_with_state(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        muted: bool,
        deafened: bool,
    ) -> JoinOutcome {
        let mut guard = self.state.write().await;

        let mut left_channel = None;
        if let Some(prev_channel) = guard.user_room.get(&user_id).copied()
            && prev_channel != channel_id
            && let Some(members) = guard.rooms.get_mut(&prev_channel)
        {
            members.remove(&user_id);
            let remaining = sorted_participants(members);
            if members.is_empty() {
                guard.rooms.remove(&prev_channel);
            }
            left_channel = Some((prev_channel, remaining));
        }

        guard.user_room.insert(user_id, channel_id);
        let members = guard.rooms.entry(channel_id).or_default();
        members.insert(user_id, (muted, deafened));
        let current = sorted_participants(members);

        JoinOutcome {
            left_channel,
            joined_channel: (channel_id, current),
        }
    }

    /// Updates `user_id`'s `muted` and `deafened` state in their current room, if any.
    pub async fn set_state(
        &self,
        user_id: Uuid,
        muted: bool,
        deafened: bool,
    ) -> Option<(Uuid, Vec<VoiceParticipant>)> {
        let mut guard = self.state.write().await;
        let channel_id = guard.user_room.get(&user_id).copied()?;
        let members = guard.rooms.get_mut(&channel_id)?;
        members.insert(user_id, (muted, deafened));
        Some((channel_id, sorted_participants(members)))
    }

    /// Removes `user_id` from their current room, returning `(channel_id, remaining_participants)` if any.
    pub async fn leave(&self, user_id: Uuid) -> Option<(Uuid, Vec<VoiceParticipant>)> {
        let mut guard = self.state.write().await;
        let channel_id = guard.user_room.remove(&user_id)?;
        let remaining = if let Some(members) = guard.rooms.get_mut(&channel_id) {
            members.remove(&user_id);
            let list = sorted_participants(members);
            if members.is_empty() {
                guard.rooms.remove(&channel_id);
            }
            list
        } else {
            Vec::new()
        };
        Some((channel_id, remaining))
    }

    /// Returns the `channel_id` that `user_id` is currently in, if any.
    pub async fn current_room(&self, user_id: Uuid) -> Option<Uuid> {
        let guard = self.state.read().await;
        guard.user_room.get(&user_id).copied()
    }

    /// Returns a snapshot of all non-empty rooms and their participants.
    pub async fn snapshot(&self) -> HashMap<Uuid, Vec<VoiceParticipant>> {
        let guard = self.state.read().await;
        guard
            .rooms
            .iter()
            .map(|(cid, members)| (*cid, sorted_participants(members)))
            .collect()
    }
}

fn sorted_participants(members: &HashMap<Uuid, (bool, bool)>) -> Vec<VoiceParticipant> {
    let mut list: Vec<VoiceParticipant> = members
        .iter()
        .map(|(&user_id, &(muted, deafened))| VoiceParticipant {
            user_id,
            muted,
            deafened,
        })
        .collect();
    list.sort_by_key(|p| p.user_id);
    list
}
