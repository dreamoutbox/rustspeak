use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::error::ApiError;
use crate::state::AppState;
use crate::ws::event::{ClientCommand, ServerEvent};

const HEARTBEAT_INTERVAL_SECS: u64 = 30;
const BEARER_PREFIX: &str = "Bearer ";

#[derive(Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// Upgrades an authenticated WebSocket connection (`GET /ws`).
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WsAuthQuery>,
) -> Result<Response, ApiError> {
    let token = extract_ws_token(&headers, &query)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "missing WebSocket auth token"))?;

    // Validate token before upgrading; never log the token value.
    let claims = state.token_service.verify_access_token(token)?;
    let user_id = claims.sub;

    Ok(ws
        .on_upgrade(move |socket| handle_socket(socket, state, user_id))
        .into_response())
}

fn extract_ws_token<'a>(headers: &'a HeaderMap, query: &'a WsAuthQuery) -> Option<&'a str> {
    if let Some(token) = query.token.as_deref()
        && !token.is_empty()
    {
        return Some(token);
    }
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix(BEARER_PREFIX))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: uuid::Uuid) {
    let (conn_id, mut rx, became_online, online_user_ids) = state.ws_hub.register(user_id).await;

    let (mut ws_sender, mut ws_receiver) = socket.split();

    let ready = ServerEvent::Ready {
        user_id,
        online_user_ids,
    };
    if let Ok(json) = serde_json::to_string(&ready)
        && ws_sender.send(Message::Text(json.into())).await.is_err()
    {
        state.ws_hub.unregister(user_id, conn_id).await;
        return;
    }

    if became_online {
        state
            .ws_hub
            .broadcast(&ServerEvent::PresenceUpdated {
                user_id,
                online: true,
            })
            .await;
    }

    let (ping_tx, mut ping_rx) = mpsc::unbounded_channel::<Message>();

    let write_task = tokio::spawn(async move {
        let mut heartbeat = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        heartbeat.tick().await;
        loop {
            tokio::select! {
                Some(text) = rx.recv() => {
                    if ws_sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Some(ctrl) = ping_rx.recv() => {
                    if ws_sender.send(ctrl).await.is_err() {
                        break;
                    }
                }
                _ = heartbeat.tick() => {
                    if ws_sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(ClientCommand::Ping) = serde_json::from_str::<ClientCommand>(&text)
                    && let Ok(pong) = serde_json::to_string(&ServerEvent::Pong)
                {
                    let _ = ping_tx.send(Message::Text(pong.into()));
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => {}
        }
    }

    write_task.abort();
    let became_offline = state.ws_hub.unregister(user_id, conn_id).await;
    if became_offline {
        state
            .ws_hub
            .broadcast(&ServerEvent::PresenceUpdated {
                user_id,
                online: false,
            })
            .await;
    }
}
