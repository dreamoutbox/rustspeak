use axum::Json;
use axum::extract::{Path, State};
use rs_common::Role;
use rs_user::User;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth_extractor::RequireAdmin;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct UsersListResponse {
    pub users: Vec<User>,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub user: User,
}

#[derive(Deserialize)]
pub struct ChangeRoleRequest {
    pub role: Role,
}

#[derive(Deserialize)]
pub struct SetDisabledRequest {
    pub disabled: bool,
}

/// Lists all registered users (`GET /api/admin/users`).
pub async fn list_users(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
) -> Result<Json<UsersListResponse>, ApiError> {
    let users = state.user_service.list_users(claims.role).await?;
    Ok(Json(UsersListResponse { users }))
}

/// Changes a user's global role (`PATCH /api/admin/users/{id}/role`).
pub async fn change_role(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Path(target_id): Path<Uuid>,
    Json(payload): Json<ChangeRoleRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state
        .user_service
        .change_role(claims.sub, claims.role, target_id, payload.role)
        .await?;
    Ok(Json(UserResponse { user }))
}

/// Enables or disables a user account (`PATCH /api/admin/users/{id}/disabled`).
pub async fn set_disabled(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Path(target_id): Path<Uuid>,
    Json(payload): Json<SetDisabledRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state
        .user_service
        .set_disabled(claims.sub, claims.role, target_id, payload.disabled)
        .await?;
    Ok(Json(UserResponse { user }))
}
