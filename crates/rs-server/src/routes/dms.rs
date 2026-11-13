use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use rs_text::{DmConversation, DmMessage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth_extractor::AuthUser;
use crate::error::ApiError;
use crate::routes::channels::{HistoryQuery, PostMessageRequest};
use crate::state::AppState;
use crate::ws::event::ServerEvent;

#[derive(Serialize)]
pub struct DmConversationsResponse {
    pub conversations: Vec<DmConversation>,
}

#[derive(Serialize)]
pub struct DmConversationResponse {
    pub conversation: DmConversation,
}

#[derive(Deserialize)]
pub struct OpenDmRequest {
    pub peer_id: Uuid,
}

#[derive(Serialize)]
pub struct DmMessagesResponse {
    pub messages: Vec<DmMessage>,
}

#[derive(Serialize)]
pub struct DmMessageResponse {
    pub message: DmMessage,
}

/// Lists all 1:1 DM conversations for the authenticated user (`GET /api/dms`).
pub async fn list_dms(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<DmConversationsResponse>, ApiError> {
    let conversations = state.text_service.list_dms(claims.sub).await?;
    Ok(Json(DmConversationsResponse { conversations }))
}

/// Opens or retrieves a 1:1 DM conversation with `peer_id` (`POST /api/dms`).
pub async fn open_dm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<OpenDmRequest>,
) -> Result<Json<DmConversationResponse>, ApiError> {
    let conversation = state
        .text_service
        .open_dm(claims.sub, payload.peer_id)
        .await?;
    Ok(Json(DmConversationResponse { conversation }))
}

/// Fetches DM history for `conversation_id` (`GET /api/dms/{id}/messages`).
pub async fn get_dm_history(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(conversation_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<DmMessagesResponse>, ApiError> {
    let messages = state
        .text_service
        .get_dm_history(claims.sub, conversation_id, query.before, query.limit)
        .await?;
    Ok(Json(DmMessagesResponse { messages }))
}

/// Sends a DM and delivers it over WebSocket only to the two participants (`POST /api/dms/{id}/messages`).
pub async fn send_dm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<PostMessageRequest>,
) -> Result<(StatusCode, Json<DmMessageResponse>), ApiError> {
    let (message, (user_a, user_b)) = state
        .text_service
        .send_dm(claims.sub, conversation_id, &payload.content)
        .await?;

    let event = ServerEvent::DmCreated {
        message: message.clone(),
    };
    state.ws_hub.send_to_user(user_a, &event).await;
    state.ws_hub.send_to_user(user_b, &event).await;

    Ok((StatusCode::CREATED, Json(DmMessageResponse { message })))
}
