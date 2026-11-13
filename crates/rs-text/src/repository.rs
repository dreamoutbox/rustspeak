use sqlx::PgPool;
use uuid::Uuid;

use crate::{DmConversation, DmMessage, Message, TextChannel, TextError};

const PG_UNIQUE_VIOLATION: &str = "23505";

/// PostgreSQL persistence for text channels and messages.
#[derive(Clone)]
pub struct TextRepository {
    pool: PgPool,
}

impl TextRepository {
    /// Creates a new `TextRepository` backed by `pool`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Seeds a default `#general` channel if no text channels exist yet.
    pub(crate) async fn seed_default_channel(&self) -> Result<(), TextError> {
        let has_channels: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM text_channels)")
            .fetch_one(&self.pool)
            .await?;
        if has_channels {
            return Ok(());
        }

        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO text_channels (id, name, topic)
             VALUES ($1, 'general', 'General discussion')
             ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Lists all text channels ordered by creation time.
    pub(crate) async fn list_channels(&self) -> Result<Vec<TextChannel>, TextError> {
        let rows = sqlx::query_as::<_, TextChannel>(
            "SELECT id, name, topic, created_at
             FROM text_channels
             ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Creates a new text channel, returning `ChannelNameTaken` on duplicate name.
    pub(crate) async fn create_channel(
        &self,
        id: Uuid,
        name: &str,
        topic: &str,
    ) -> Result<TextChannel, TextError> {
        let row = sqlx::query_as::<_, TextChannel>(
            "INSERT INTO text_channels (id, name, topic)
             VALUES ($1, $2, $3)
             RETURNING id, name, topic, created_at",
        )
        .bind(id)
        .bind(name)
        .bind(topic)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_violation)?;
        Ok(row)
    }

    /// Deletes a text channel by `channel_id`, returning `true` if a row was deleted.
    pub(crate) async fn delete_channel(&self, channel_id: Uuid) -> Result<bool, TextError> {
        let res = sqlx::query("DELETE FROM text_channels WHERE id = $1")
            .bind(channel_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Returns `true` if a text channel with `channel_id` exists.
    pub(crate) async fn channel_exists(&self, channel_id: Uuid) -> Result<bool, TextError> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM text_channels WHERE id = $1)")
                .bind(channel_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }

    /// Inserts a message and returns it joined with the author's username.
    pub(crate) async fn insert_message(
        &self,
        id: Uuid,
        channel_id: Uuid,
        author_id: Uuid,
        content: &str,
    ) -> Result<Message, TextError> {
        let row = sqlx::query_as::<_, Message>(
            "WITH inserted AS (
                INSERT INTO messages (id, channel_id, author_id, content)
                VALUES ($1, $2, $3, $4)
                RETURNING id, channel_id, author_id, content, created_at
             )
             SELECT i.id, i.channel_id, i.author_id, u.username AS author_username, i.content, i.created_at
             FROM inserted i
             JOIN users u ON u.id = i.author_id",
        )
        .bind(id)
        .bind(channel_id)
        .bind(author_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Fetches messages in `channel_id` ordered newest-first, optionally before cursor `before`.
    pub(crate) async fn list_messages(
        &self,
        channel_id: Uuid,
        before: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<Message>, TextError> {
        let rows = sqlx::query_as::<_, Message>(
            "SELECT m.id, m.channel_id, m.author_id, u.username AS author_username, m.content, m.created_at
             FROM messages m
             JOIN users u ON u.id = m.author_id
             WHERE m.channel_id = $1
               AND ($2::uuid IS NULL OR m.id < $2)
             ORDER BY m.id DESC
             LIMIT $3",
        )
        .bind(channel_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Returns `true` if a user with `user_id` exists.
    pub(crate) async fn user_exists(&self, user_id: Uuid) -> Result<bool, TextError> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(exists)
    }

    /// Opens or retrieves the unique 1:1 DM conversation between `actor_id` and `peer_id`.
    pub(crate) async fn open_dm_conversation(
        &self,
        actor_id: Uuid,
        peer_id: Uuid,
    ) -> Result<DmConversation, TextError> {
        let (user_a, user_b) = if actor_id < peer_id {
            (actor_id, peer_id)
        } else {
            (peer_id, actor_id)
        };

        sqlx::query(
            "INSERT INTO dm_conversations (id, user_a, user_b)
             VALUES ($1, $2, $3)
             ON CONFLICT (user_a, user_b) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(user_a)
        .bind(user_b)
        .execute(&self.pool)
        .await?;

        let conv = sqlx::query_as::<_, DmConversation>(
            "SELECT c.id, u.id AS peer_id, u.username AS peer_username, c.created_at
             FROM dm_conversations c
             JOIN users u ON u.id = $3
             WHERE c.user_a = $1 AND c.user_b = $2",
        )
        .bind(user_a)
        .bind(user_b)
        .bind(peer_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(conv)
    }

    /// Lists all DM conversations for `actor_id`.
    pub(crate) async fn list_dm_conversations(
        &self,
        actor_id: Uuid,
    ) -> Result<Vec<DmConversation>, TextError> {
        let rows = sqlx::query_as::<_, DmConversation>(
            "SELECT c.id,
                    CASE WHEN c.user_a = $1 THEN c.user_b ELSE c.user_a END AS peer_id,
                    u.username AS peer_username,
                    c.created_at
             FROM dm_conversations c
             JOIN users u ON u.id = CASE WHEN c.user_a = $1 THEN c.user_b ELSE c.user_a END
             WHERE c.user_a = $1 OR c.user_b = $1
             ORDER BY c.created_at DESC",
        )
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Returns `(user_a, user_b)` for `conversation_id`, or `None` if not found.
    pub(crate) async fn find_dm_participants(
        &self,
        conversation_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid)>, TextError> {
        let row = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT user_a, user_b FROM dm_conversations WHERE id = $1",
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Inserts a DM message and returns it joined with the author's username.
    pub(crate) async fn insert_dm_message(
        &self,
        id: Uuid,
        conversation_id: Uuid,
        author_id: Uuid,
        content: &str,
    ) -> Result<DmMessage, TextError> {
        let row = sqlx::query_as::<_, DmMessage>(
            "WITH inserted AS (
                INSERT INTO dm_messages (id, conversation_id, author_id, content)
                VALUES ($1, $2, $3, $4)
                RETURNING id, conversation_id, author_id, content, created_at
             )
             SELECT i.id, i.conversation_id, i.author_id, u.username AS author_username, i.content, i.created_at
             FROM inserted i
             JOIN users u ON u.id = i.author_id",
        )
        .bind(id)
        .bind(conversation_id)
        .bind(author_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Fetches DM messages in `conversation_id` ordered newest-first, optionally before `before`.
    pub(crate) async fn list_dm_messages(
        &self,
        conversation_id: Uuid,
        before: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<DmMessage>, TextError> {
        let rows = sqlx::query_as::<_, DmMessage>(
            "SELECT m.id, m.conversation_id, m.author_id, u.username AS author_username, m.content, m.created_at
             FROM dm_messages m
             JOIN users u ON u.id = m.author_id
             WHERE m.conversation_id = $1
               AND ($2::uuid IS NULL OR m.id < $2)
             ORDER BY m.id DESC
             LIMIT $3",
        )
        .bind(conversation_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

fn map_unique_violation(err: sqlx::Error) -> TextError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some(PG_UNIQUE_VIOLATION)
    {
        return TextError::ChannelNameTaken;
    }
    TextError::Database(err)
}
