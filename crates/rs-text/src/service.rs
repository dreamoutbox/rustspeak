use rs_common::Role;
use uuid::Uuid;

use crate::{
    DmConversation, DmMessage, Message, TextChannel, TextError, TextRepository,
    validate_channel_name, validate_message_content, validate_topic,
};

const DEFAULT_HISTORY_LIMIT: u32 = 50;
const MAX_HISTORY_LIMIT: u32 = 100;

/// Application service for text channels and messages.
#[derive(Clone)]
pub struct TextService {
    repo: TextRepository,
}

impl TextService {
    /// Creates a new `TextService` backed by `repo`.
    pub fn new(repo: TextRepository) -> Self {
        Self { repo }
    }

    /// Ensures a default `#general` channel exists on first boot.
    pub async fn seed_default_channel(&self) -> Result<(), TextError> {
        self.repo.seed_default_channel().await
    }

    /// Lists all text channels.
    pub async fn list_channels(&self) -> Result<Vec<TextChannel>, TextError> {
        self.repo.list_channels().await
    }

    /// Creates a new text channel (requires `Role::Admin` or higher).
    pub async fn create_channel(
        &self,
        actor_role: Role,
        name: &str,
        topic: &str,
    ) -> Result<TextChannel, TextError> {
        if actor_role < Role::Admin {
            return Err(TextError::Forbidden(
                "admin role required to create channels".to_string(),
            ));
        }
        let clean_name = validate_channel_name(name)?;
        let clean_topic = validate_topic(topic)?;
        self.repo
            .create_channel(Uuid::now_v7(), &clean_name, clean_topic)
            .await
    }

    /// Deletes a text channel (requires `Role::Admin` or higher).
    pub async fn delete_channel(
        &self,
        actor_role: Role,
        channel_id: Uuid,
    ) -> Result<(), TextError> {
        if actor_role < Role::Admin {
            return Err(TextError::Forbidden(
                "admin role required to delete channels".to_string(),
            ));
        }
        let deleted = self.repo.delete_channel(channel_id).await?;
        if !deleted {
            return Err(TextError::NotFound);
        }
        Ok(())
    }

    /// Posts a validated message to `channel_id` as `author_id`.
    pub async fn post_message(
        &self,
        channel_id: Uuid,
        author_id: Uuid,
        content: &str,
    ) -> Result<Message, TextError> {
        let clean_content = validate_message_content(content)?;
        if !self.repo.channel_exists(channel_id).await? {
            return Err(TextError::NotFound);
        }
        self.repo
            .insert_message(Uuid::now_v7(), channel_id, author_id, clean_content)
            .await
    }

    /// Fetches message history for `channel_id` ordered newest-first using cursor `before`.
    pub async fn get_history(
        &self,
        channel_id: Uuid,
        before: Option<Uuid>,
        limit: Option<u32>,
    ) -> Result<Vec<Message>, TextError> {
        if !self.repo.channel_exists(channel_id).await? {
            return Err(TextError::NotFound);
        }
        let clamped = limit
            .unwrap_or(DEFAULT_HISTORY_LIMIT)
            .clamp(1, MAX_HISTORY_LIMIT) as i64;
        self.repo.list_messages(channel_id, before, clamped).await
    }

    /// Opens or returns the unique 1:1 DM conversation between `actor_id` and `peer_id`.
    pub async fn open_dm(
        &self,
        actor_id: Uuid,
        peer_id: Uuid,
    ) -> Result<DmConversation, TextError> {
        if actor_id == peer_id {
            return Err(TextError::InvalidInput(
                "cannot open a DM conversation with yourself".to_string(),
            ));
        }
        if !self.repo.user_exists(peer_id).await? {
            return Err(TextError::NotFound);
        }
        self.repo.open_dm_conversation(actor_id, peer_id).await
    }

    /// Lists all DM conversations for `actor_id`.
    pub async fn list_dms(&self, actor_id: Uuid) -> Result<Vec<DmConversation>, TextError> {
        self.repo.list_dm_conversations(actor_id).await
    }

    /// Fetches DM history for `conversation_id` if `actor_id` is a participant.
    pub async fn get_dm_history(
        &self,
        actor_id: Uuid,
        conversation_id: Uuid,
        before: Option<Uuid>,
        limit: Option<u32>,
    ) -> Result<Vec<DmMessage>, TextError> {
        self.ensure_dm_participant(actor_id, conversation_id)
            .await?;
        let clamped = limit
            .unwrap_or(DEFAULT_HISTORY_LIMIT)
            .clamp(1, MAX_HISTORY_LIMIT) as i64;
        self.repo
            .list_dm_messages(conversation_id, before, clamped)
            .await
    }

    /// Sends a DM in `conversation_id` as `actor_id`, returning the message and `(user_a, user_b)`.
    pub async fn send_dm(
        &self,
        actor_id: Uuid,
        conversation_id: Uuid,
        content: &str,
    ) -> Result<(DmMessage, (Uuid, Uuid)), TextError> {
        let participants = self
            .ensure_dm_participant(actor_id, conversation_id)
            .await?;
        let clean_content = validate_message_content(content)?;
        let msg = self
            .repo
            .insert_dm_message(Uuid::now_v7(), conversation_id, actor_id, clean_content)
            .await?;
        Ok((msg, participants))
    }

    async fn ensure_dm_participant(
        &self,
        actor_id: Uuid,
        conversation_id: Uuid,
    ) -> Result<(Uuid, Uuid), TextError> {
        let (user_a, user_b) = self
            .repo
            .find_dm_participants(conversation_id)
            .await?
            .ok_or(TextError::NotFound)?;
        if actor_id != user_a && actor_id != user_b {
            return Err(TextError::Forbidden(
                "not a participant in this DM conversation".to_string(),
            ));
        }
        Ok((user_a, user_b))
    }
}
