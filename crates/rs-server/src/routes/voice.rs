use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rs_voice::VoiceChannel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth_extractor::{AuthUser, RequireAdmin};
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct VoiceChannelWithParticipants {
    #[serde(flatten)]
    pub channel: VoiceChannel,
    pub participant_ids: Vec<Uuid>,
}

#[derive(Serialize)]
pub struct VoiceChannelsResponse {
    pub channels: Vec<VoiceChannelWithParticipants>,
}

#[derive(Serialize)]
pub struct VoiceChannelResponse {
    pub channel: VoiceChannel,
}

#[derive(Deserialize)]
pub struct CreateVoiceChannelRequest {
    pub name: String,
}

/// Lists all voice channels and their current participants (`GET /api/voice/channels`).
pub async fn list_channels(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<VoiceChannelsResponse>, ApiError> {
    let channels = state.voice_service.list_channels().await?;
    let mut snapshot = state.voice_service.room_snapshot().await;
    let items = channels
        .into_iter()
        .map(|channel| {
            let participant_ids = snapshot.remove(&channel.id).unwrap_or_default();
            VoiceChannelWithParticipants {
                channel,
                participant_ids,
            }
        })
        .collect();
    Ok(Json(VoiceChannelsResponse { channels: items }))
}

/// Creates a new voice channel (`POST /api/voice/channels`, Admin+).
pub async fn create_channel(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Json(payload): Json<CreateVoiceChannelRequest>,
) -> Result<(StatusCode, Json<VoiceChannelResponse>), ApiError> {
    let channel = state
        .voice_service
        .create_channel(claims.role, &payload.name)
        .await?;
    Ok((StatusCode::CREATED, Json(VoiceChannelResponse { channel })))
}

/// Deletes a voice channel (`DELETE /api/voice/channels/{id}`, Admin+).
pub async fn delete_channel(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Path(channel_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .voice_service
        .delete_channel(claims.role, channel_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
