use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
struct RoomState {
    rooms: HashMap<Uuid, HashSet<Uuid>>,
    user_room: HashMap<Uuid, Uuid>,
}

/// Result of joining a voice room, including any previous room left due to the one-room rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOutcome {
    /// Previous channel left automatically, if the user was already in another room.
    pub left_channel: Option<(Uuid, Vec<Uuid>)>,
    /// Channel joined and its updated participant list.
    pub joined_channel: (Uuid, Vec<Uuid>),
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

    /// Adds `user_id` to `channel_id`, automatically removing them from any prior room.
    pub async fn join(&self, user_id: Uuid, channel_id: Uuid) -> JoinOutcome {
        let mut guard = self.state.write().await;

        let mut left_channel = None;
        if let Some(prev_channel) = guard.user_room.get(&user_id).copied()
            && prev_channel != channel_id
            && let Some(members) = guard.rooms.get_mut(&prev_channel)
        {
            members.remove(&user_id);
            let remaining = sorted_members(members);
            if members.is_empty() {
                guard.rooms.remove(&prev_channel);
            }
            left_channel = Some((prev_channel, remaining));
        }

        guard.user_room.insert(user_id, channel_id);
        let members = guard.rooms.entry(channel_id).or_default();
        members.insert(user_id);
        let current = sorted_members(members);

        JoinOutcome {
            left_channel,
            joined_channel: (channel_id, current),
        }
    }

    /// Removes `user_id` from their current room, returning `(channel_id, remaining_users)` if any.
    pub async fn leave(&self, user_id: Uuid) -> Option<(Uuid, Vec<Uuid>)> {
        let mut guard = self.state.write().await;
        let channel_id = guard.user_room.remove(&user_id)?;
        let remaining = if let Some(members) = guard.rooms.get_mut(&channel_id) {
            members.remove(&user_id);
            let list = sorted_members(members);
            if members.is_empty() {
                guard.rooms.remove(&channel_id);
            }
            list
        } else {
            Vec::new()
        };
        Some((channel_id, remaining))
    }

    /// Returns a snapshot of all non-empty rooms and their participant IDs.
    pub async fn snapshot(&self) -> HashMap<Uuid, Vec<Uuid>> {
        let guard = self.state.read().await;
        guard
            .rooms
            .iter()
            .map(|(cid, members)| (*cid, sorted_members(members)))
            .collect()
    }
}

fn sorted_members(members: &HashSet<Uuid>) -> Vec<Uuid> {
    let mut list: Vec<Uuid> = members.iter().copied().collect();
    list.sort();
    list
}
