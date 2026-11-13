use std::collections::HashMap;

use rs_common::Role;
use uuid::Uuid;

use crate::{
    JoinOutcome, RoomRegistry, VoiceChannel, VoiceError, VoiceParticipant, VoiceRepository,
};

const MIN_NAME_LEN: usize = 1;
const MAX_NAME_LEN: usize = 32;

/// Application service for voice channels and room membership.
#[derive(Clone)]
pub struct VoiceService {
    repo: VoiceRepository,
    rooms: RoomRegistry,
}

impl VoiceService {
    /// Creates a new `VoiceService`.
    pub fn new(repo: VoiceRepository, rooms: RoomRegistry) -> Self {
        Self { repo, rooms }
    }

    /// Seeds a default `Lobby` voice channel on first boot.
    pub async fn seed_default_channel(&self) -> Result<(), VoiceError> {
        self.repo.seed_default_channel().await
    }

    /// Lists all voice channels.
    pub async fn list_channels(&self) -> Result<Vec<VoiceChannel>, VoiceError> {
        self.repo.list_channels().await
    }

    /// Creates a new voice channel (requires `Role::Admin` or higher).
    pub async fn create_channel(
        &self,
        actor_role: Role,
        name: &str,
    ) -> Result<VoiceChannel, VoiceError> {
        if actor_role < Role::Admin {
            return Err(VoiceError::Forbidden(
                "admin role required to create voice channels".to_string(),
            ));
        }
        let clean_name = validate_voice_channel_name(name)?;
        self.repo.create_channel(Uuid::now_v7(), clean_name).await
    }

    /// Deletes a voice channel (requires `Role::Admin` or higher).
    pub async fn delete_channel(
        &self,
        actor_role: Role,
        channel_id: Uuid,
    ) -> Result<(), VoiceError> {
        if actor_role < Role::Admin {
            return Err(VoiceError::Forbidden(
                "admin role required to delete voice channels".to_string(),
            ));
        }
        let deleted = self.repo.delete_channel(channel_id).await?;
        if !deleted {
            return Err(VoiceError::NotFound);
        }
        Ok(())
    }

    /// Joins `user_id` to `channel_id`, enforcing the one-room-per-user rule.
    pub async fn join_room(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<JoinOutcome, VoiceError> {
        self.join_room_with_state(user_id, channel_id, false, false)
            .await
    }

    /// Joins `user_id` to `channel_id` with initial `muted` and `deafened` state.
    pub async fn join_room_with_state(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        muted: bool,
        deafened: bool,
    ) -> Result<JoinOutcome, VoiceError> {
        if !self.repo.channel_exists(channel_id).await? {
            return Err(VoiceError::NotFound);
        }
        Ok(self
            .rooms
            .join_with_state(user_id, channel_id, muted, deafened)
            .await)
    }

    /// Updates `user_id`'s `muted` and `deafened` state in their current voice room, if any.
    pub async fn set_voice_state(
        &self,
        user_id: Uuid,
        muted: bool,
        deafened: bool,
    ) -> Option<(Uuid, Vec<VoiceParticipant>)> {
        self.rooms.set_state(user_id, muted, deafened).await
    }

    /// Removes `user_id` from their current voice room, if any.
    pub async fn leave_room(&self, user_id: Uuid) -> Option<(Uuid, Vec<VoiceParticipant>)> {
        self.rooms.leave(user_id).await
    }

    /// Returns the `channel_id` that `user_id` is currently in, if any.
    pub async fn current_room(&self, user_id: Uuid) -> Option<Uuid> {
        self.rooms.current_room(user_id).await
    }

    /// Returns a snapshot of participants per voice channel.
    pub async fn room_snapshot(&self) -> HashMap<Uuid, Vec<VoiceParticipant>> {
        self.rooms.snapshot().await
    }
}

fn validate_voice_channel_name(name: &str) -> Result<&str, VoiceError> {
    let trimmed = name.trim();
    if trimmed.len() < MIN_NAME_LEN || trimmed.len() > MAX_NAME_LEN {
        return Err(VoiceError::InvalidInput(format!(
            "voice channel name must be {MIN_NAME_LEN}..={MAX_NAME_LEN} characters"
        )));
    }
    Ok(trimmed)
}
