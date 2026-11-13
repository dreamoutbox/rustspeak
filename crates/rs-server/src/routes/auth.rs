use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use rs_user::{AuthTokens, User};
use serde::{Deserialize, Serialize};

use crate::auth_extractor::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CredentialsRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: User,
    pub tokens: AuthTokens,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub user: User,
}

/// Registers a new user (`POST /api/auth/register`).
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CredentialsRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    let (user, tokens) = state
        .user_service
        .register(&payload.username, &payload.password)
        .await?;
    Ok((StatusCode::CREATED, Json(AuthResponse { user, tokens })))
}

/// Authenticates an existing user (`POST /api/auth/login`).
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<CredentialsRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let (user, tokens) = state
        .user_service
        .login(&payload.username, &payload.password)
        .await?;
    Ok(Json(AuthResponse { user, tokens }))
}

/// Rotates a refresh token (`POST /api/auth/refresh`).
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let (user, tokens) = state.user_service.refresh(&payload.refresh_token).await?;
    Ok(Json(AuthResponse { user, tokens }))
}

/// Revokes a refresh token (`POST /api/auth/logout`).
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<StatusCode, ApiError> {
    state.user_service.logout(&payload.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Returns the authenticated user's profile (`GET /api/users/me`).
pub async fn me(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<MeResponse>, ApiError> {
    let user = state.user_service.get_user(claims.sub).await?;
    Ok(Json(MeResponse { user }))
}

