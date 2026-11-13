use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use rs_text::{Message, TextChannel};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth_extractor::{AuthUser, RequireAdmin};
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ChannelsResponse {
    pub channels: Vec<TextChannel>,
}

#[derive(Serialize)]
pub struct ChannelResponse {
    pub channel: TextChannel,
}

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(default)]
    pub topic: String,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub before: Option<Uuid>,
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct MessagesResponse {
    pub messages: Vec<Message>,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: Message,
}

#[derive(Deserialize)]
pub struct PostMessageRequest {
    pub content: String,
}

/// Lists all text channels (`GET /api/channels`).
pub async fn list_channels(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<ChannelsResponse>, ApiError> {
    let channels = state.text_service.list_channels().await?;
    Ok(Json(ChannelsResponse { channels }))
}

/// Creates a new text channel (`POST /api/channels`, Admin+).
pub async fn create_channel(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Json(payload): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), ApiError> {
    let channel = state
        .text_service
        .create_channel(claims.role, &payload.name, &payload.topic)
        .await?;
    Ok((StatusCode::CREATED, Json(ChannelResponse { channel })))
}

/// Deletes a text channel (`DELETE /api/channels/{id}`, Admin+).
pub async fn delete_channel(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Path(channel_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .text_service
        .delete_channel(claims.role, channel_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Lists message history in a channel (`GET /api/channels/{id}/messages`).
pub async fn get_history(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<MessagesResponse>, ApiError> {
    let messages = state
        .text_service
        .get_history(channel_id, query.before, query.limit)
        .await?;
    Ok(Json(MessagesResponse { messages }))
}

/// Posts a new message in a channel (`POST /api/channels/{id}/messages`).
pub async fn post_message(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<PostMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    let message = state
        .text_service
        .post_message(channel_id, claims.sub, &payload.content)
        .await?;
    Ok((StatusCode::CREATED, Json(MessageResponse { message })))
}
