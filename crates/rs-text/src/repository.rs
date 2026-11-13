use sqlx::PgPool;
use uuid::Uuid;

use crate::{Message, TextChannel, TextError};

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
}

fn map_unique_violation(err: sqlx::Error) -> TextError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some(PG_UNIQUE_VIOLATION)
    {
        return TextError::ChannelNameTaken;
    }
    TextError::Database(err)
}
