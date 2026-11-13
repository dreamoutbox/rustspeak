use chrono::{DateTime, Utc};
use rs_common::Role;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{User, UserError};

const PG_UNIQUE_VIOLATION: &str = "23505";
const FIRST_USER_LOCK_KEY: i64 = 0x7275_7374_7370_6561;

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct UserAuthRow {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
}

impl From<UserAuthRow> for User {
    fn from(row: UserAuthRow) -> Self {
        Self {
            id: row.id,
            username: row.username,
            role: row.role,
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

    /// Inserts a new user row, assigning `Role::Owner` to the first user in a race-safe transaction.
    pub(crate) async fn insert_user(
        &self,
        id: Uuid,
        username: &str,
        password_hash: &str,
    ) -> Result<User, UserError> {
        let mut tx = self.pool.begin().await?;

        // Serialize first-user check across concurrent registrations.
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(FIRST_USER_LOCK_KEY)
            .execute(&mut *tx)
            .await?;

        let has_users: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users)")
            .fetch_one(&mut *tx)
            .await?;
        let role = if has_users { Role::Member } else { Role::Owner };

        let row = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, username, password_hash, role)
             VALUES ($1, $2, $3, $4)
             RETURNING id, username, role, disabled, created_at",
        )
        .bind(id)
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_unique_violation)?;

        tx.commit().await?;
        Ok(row)
    }

    /// Finds a user and their password hash by case-insensitive username.
    pub(crate) async fn find_auth_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserAuthRow>, UserError> {
        let row = sqlx::query_as::<_, UserAuthRow>(
            "SELECT id, username, password_hash, role, disabled, created_at
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
            "SELECT id, username, role, disabled, created_at
             FROM users
             WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Lists all users ordered by creation time.
    pub(crate) async fn list_users(&self) -> Result<Vec<User>, UserError> {
        let rows = sqlx::query_as::<_, User>(
            "SELECT id, username, role, disabled, created_at
             FROM users
             ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Updates a user's role, returning `None` if the user does not exist.
    pub(crate) async fn update_role(
        &self,
        user_id: Uuid,
        role: Role,
    ) -> Result<Option<User>, UserError> {
        let row = sqlx::query_as::<_, User>(
            "UPDATE users
             SET role = $2
             WHERE id = $1
             RETURNING id, username, role, disabled, created_at",
        )
        .bind(user_id)
        .bind(role)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Updates a user's `disabled` flag, returning `None` if the user does not exist.
    pub(crate) async fn set_disabled(
        &self,
        user_id: Uuid,
        disabled: bool,
    ) -> Result<Option<User>, UserError> {
        let row = sqlx::query_as::<_, User>(
            "UPDATE users
             SET disabled = $2
             WHERE id = $1
             RETURNING id, username, role, disabled, created_at",
        )
        .bind(user_id)
        .bind(disabled)
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

    /// Revokes all active refresh tokens for `user_id`.
    pub(crate) async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), UserError> {
        sqlx::query(
            "UPDATE refresh_tokens
             SET revoked_at = NOW()
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
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
