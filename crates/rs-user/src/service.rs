use chrono::Utc;
use rs_common::Role;
use uuid::Uuid;

use crate::{
    AuthTokens, TokenService, User, UserError, UserRepository, hash_password, validate_password,
    validate_username, verify_password,
};

/// Application service for user registration, login, and token lifecycle.
#[derive(Clone)]
pub struct UserService {
    repo: UserRepository,
    tokens: TokenService,
}

impl UserService {
    /// Creates a new `UserService` from a `UserRepository` and `TokenService`.
    pub fn new(repo: UserRepository, tokens: TokenService) -> Self {
        Self { repo, tokens }
    }

    /// Registers a new user and issues initial access and refresh tokens.
    pub async fn register(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(User, AuthTokens), UserError> {
        let clean_username = validate_username(username)?;
        validate_password(password)?;

        let password_hash = hash_password(password)?;
        let user = self
            .repo
            .insert_user(Uuid::now_v7(), clean_username, &password_hash)
            .await?;

        let tokens = self.issue_token_pair(&user, Uuid::now_v7()).await?;
        Ok((user, tokens))
    }

    /// Authenticates an existing user and issues new access and refresh tokens.
    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(User, AuthTokens), UserError> {
        let auth_row = self
            .repo
            .find_auth_by_username(username.trim())
            .await?
            .ok_or(UserError::InvalidCredentials)?;

        let valid = verify_password(password, &auth_row.password_hash)?;
        if !valid {
            return Err(UserError::InvalidCredentials);
        }

        if auth_row.disabled {
            return Err(UserError::UserDisabled);
        }

        let user = User::from(auth_row);
        let tokens = self.issue_token_pair(&user, Uuid::now_v7()).await?;
        Ok((user, tokens))
    }

    /// Rotates a refresh token, revoking the entire family if reuse is detected.
    pub async fn refresh(&self, refresh_token: &str) -> Result<(User, AuthTokens), UserError> {
        let hash = TokenService::hash_refresh_token(refresh_token);
        let row = self
            .repo
            .find_refresh_token_by_hash(&hash)
            .await?
            .ok_or(UserError::InvalidToken)?;

        if row.revoked_at.is_some() {
            // Reuse of an already-rotated token indicates possible theft; revoke the family.
            self.repo.revoke_token_family(row.family_id).await?;
            return Err(UserError::TokenReused);
        }

        if row.expires_at <= Utc::now() {
            return Err(UserError::InvalidToken);
        }

        let user = self
            .repo
            .find_by_id(row.user_id)
            .await?
            .ok_or(UserError::NotFound)?;

        if user.disabled {
            return Err(UserError::UserDisabled);
        }

        self.repo.revoke_refresh_token(row.id).await?;
        let tokens = self.issue_token_pair(&user, row.family_id).await?;
        Ok((user, tokens))
    }

    /// Revokes the provided refresh token (no-op if unknown or already revoked).
    pub async fn logout(&self, refresh_token: &str) -> Result<(), UserError> {
        let hash = TokenService::hash_refresh_token(refresh_token);
        if let Some(row) = self.repo.find_refresh_token_by_hash(&hash).await? {
            self.repo.revoke_refresh_token(row.id).await?;
        }
        Ok(())
    }

    /// Fetches a user profile by `user_id`.
    pub async fn get_user(&self, user_id: Uuid) -> Result<User, UserError> {
        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or(UserError::NotFound)?;
        if user.disabled {
            return Err(UserError::UserDisabled);
        }
        Ok(user)
    }

    /// Lists all users (requires `Role::Admin` or `Role::Owner`).
    pub async fn list_users(&self, actor_role: Role) -> Result<Vec<User>, UserError> {
        if actor_role < Role::Admin {
            return Err(UserError::Forbidden(
                "admin role required to list users".to_string(),
            ));
        }
        self.repo.list_users().await
    }

    /// Changes `target_id`'s role to `new_role` (only `Owner` can assign `Admin`/`Member`; `Owner` is not transferable).
    pub async fn change_role(
        &self,
        actor_id: Uuid,
        actor_role: Role,
        target_id: Uuid,
        new_role: Role,
    ) -> Result<User, UserError> {
        if actor_role != Role::Owner {
            return Err(UserError::Forbidden(
                "only the owner can change user roles".to_string(),
            ));
        }
        if new_role == Role::Owner {
            return Err(UserError::Forbidden(
                "owner role cannot be assigned".to_string(),
            ));
        }
        if actor_id == target_id {
            return Err(UserError::Forbidden(
                "cannot change your own role".to_string(),
            ));
        }

        self.repo
            .update_role(target_id, new_role)
            .await?
            .ok_or(UserError::NotFound)
    }

    /// Enables or disables `target_id` (requires `Admin+`, target must have lower role than actor).
    pub async fn set_disabled(
        &self,
        actor_id: Uuid,
        actor_role: Role,
        target_id: Uuid,
        disabled: bool,
    ) -> Result<User, UserError> {
        if actor_role < Role::Admin {
            return Err(UserError::Forbidden(
                "admin role required to disable users".to_string(),
            ));
        }
        if actor_id == target_id {
            return Err(UserError::Forbidden(
                "cannot disable your own account".to_string(),
            ));
        }

        let target = self
            .repo
            .find_by_id(target_id)
            .await?
            .ok_or(UserError::NotFound)?;
        if target.role >= actor_role {
            return Err(UserError::Forbidden(
                "cannot modify a user with equal or higher role".to_string(),
            ));
        }

        let updated = self
            .repo
            .set_disabled(target_id, disabled)
            .await?
            .ok_or(UserError::NotFound)?;
        if disabled {
            self.repo.revoke_all_user_tokens(target_id).await?;
        }
        Ok(updated)
    }

    async fn issue_token_pair(
        &self,
        user: &User,
        family_id: Uuid,
    ) -> Result<AuthTokens, UserError> {
        let access_token = self.tokens.issue_access_token(user)?;
        let (refresh_token, refresh_hash) = TokenService::generate_refresh_token();
        let expires_at = self.tokens.refresh_expires_at();

        self.repo
            .insert_refresh_token(
                Uuid::now_v7(),
                user.id,
                &refresh_hash,
                family_id,
                expires_at,
            )
            .await?;

        Ok(AuthTokens {
            access_token,
            refresh_token,
            expires_in_secs: self.tokens.access_ttl_secs(),
        })
    }
}
