use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{User, UserError};

const PG_UNIQUE_VIOLATION: &str = "23505";

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct UserAuthRow {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
}

impl From<UserAuthRow> for User {
    fn from(row: UserAuthRow) -> Self {
        Self {
            id: row.id,
            username: row.username,
            disabled: row.disabled,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct RefreshTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// PostgreSQL persistence for users and refresh tokens.
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Creates a new `UserRepository` backed by `pool`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Inserts a new user row, returning `UserError::UsernameTaken` on case-insensitive collision.
    pub(crate) async fn insert_user(
        &self,
        id: Uuid,
        username: &str,
        password_hash: &str,
    ) -> Result<User, UserError> {
        let row = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, username, password_hash)
             VALUES ($1, $2, $3)
             RETURNING id, username, disabled, created_at",
        )
        .bind(id)
        .bind(username)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_violation)?;
        Ok(row)
    }

    /// Finds a user and their password hash by case-insensitive username.
    pub(crate) async fn find_auth_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserAuthRow>, UserError> {
        let row = sqlx::query_as::<_, UserAuthRow>(
            "SELECT id, username, password_hash, disabled, created_at
             FROM users
             WHERE LOWER(username) = LOWER($1)",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Finds a public user profile by `user_id`.
    pub(crate) async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, UserError> {
        let row = sqlx::query_as::<_, User>(
            "SELECT id, username, disabled, created_at
             FROM users
             WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Stores a newly issued hashed refresh token.
    pub(crate) async fn insert_refresh_token(
        &self,
        id: Uuid,
        user_id: Uuid,
        token_hash: &[u8],
        family_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> Result<(), UserError> {
        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, family_id, expires_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(family_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Looks up a refresh token record by its SHA-256 `token_hash`.
    pub(crate) async fn find_refresh_token_by_hash(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<RefreshTokenRow>, UserError> {
        let row = sqlx::query_as::<_, RefreshTokenRow>(
            "SELECT id, user_id, family_id, expires_at, revoked_at
             FROM refresh_tokens
             WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Marks a single refresh token as revoked if not already revoked.
    pub(crate) async fn revoke_refresh_token(&self, id: Uuid) -> Result<(), UserError> {
        sqlx::query(
            "UPDATE refresh_tokens
             SET revoked_at = NOW()
             WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Revokes all active tokens belonging to `family_id` (used on token reuse detection).
    pub(crate) async fn revoke_token_family(&self, family_id: Uuid) -> Result<(), UserError> {
        sqlx::query(
            "UPDATE refresh_tokens
             SET revoked_at = NOW()
             WHERE family_id = $1 AND revoked_at IS NULL",
        )
        .bind(family_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn map_unique_violation(err: sqlx::Error) -> UserError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some(PG_UNIQUE_VIOLATION)
    {
        return UserError::UsernameTaken;
    }
    UserError::Database(err)
}

