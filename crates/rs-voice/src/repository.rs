use sqlx::PgPool;
use uuid::Uuid;

use crate::{VoiceChannel, VoiceError};

const PG_UNIQUE_VIOLATION: &str = "23505";

/// PostgreSQL persistence for voice channels.
#[derive(Clone)]
pub struct VoiceRepository {
    pool: PgPool,
}

impl VoiceRepository {
    /// Creates a new `VoiceRepository` backed by `pool`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Seeds a default `Lobby` voice channel if none exist yet.
    pub(crate) async fn seed_default_channel(&self) -> Result<(), VoiceError> {
        let has_channels: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM voice_channels)")
            .fetch_one(&self.pool)
            .await?;
        if has_channels {
            return Ok(());
        }

        sqlx::query(
            "INSERT INTO voice_channels (id, name)
             VALUES ($1, 'Lobby')
             ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Lists all voice channels ordered by creation time.
    pub(crate) async fn list_channels(&self) -> Result<Vec<VoiceChannel>, VoiceError> {
        let rows = sqlx::query_as::<_, VoiceChannel>(
            "SELECT id, name, created_at
             FROM voice_channels
             ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Creates a new voice channel.
    pub(crate) async fn create_channel(
        &self,
        id: Uuid,
        name: &str,
    ) -> Result<VoiceChannel, VoiceError> {
        let row = sqlx::query_as::<_, VoiceChannel>(
            "INSERT INTO voice_channels (id, name)
             VALUES ($1, $2)
             RETURNING id, name, created_at",
        )
        .bind(id)
        .bind(name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_violation)?;
        Ok(row)
    }

    /// Deletes a voice channel by `channel_id`, returning `true` if deleted.
    pub(crate) async fn delete_channel(&self, channel_id: Uuid) -> Result<bool, VoiceError> {
        let res = sqlx::query("DELETE FROM voice_channels WHERE id = $1")
            .bind(channel_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Returns `true` if a voice channel with `channel_id` exists.
    pub(crate) async fn channel_exists(&self, channel_id: Uuid) -> Result<bool, VoiceError> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM voice_channels WHERE id = $1)")
                .bind(channel_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }
}

fn map_unique_violation(err: sqlx::Error) -> VoiceError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some(PG_UNIQUE_VIOLATION)
    {
        return VoiceError::ChannelNameTaken;
    }
    VoiceError::Database(err)
}
