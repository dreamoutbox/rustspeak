use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::ws::event::ServerEvent;

type ConnMap = HashMap<Uuid, HashMap<Uuid, mpsc::UnboundedSender<String>>>;

/// In-memory registry of active WebSocket connections for broadcasting events.
#[derive(Clone, Default)]
pub struct WsHub {
    connections: Arc<RwLock<ConnMap>>,
}

impl WsHub {
    /// Creates an empty `WsHub`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new connection for `user_id`.
    ///
    /// Returns `(conn_id, receiver, became_online, online_user_ids)`.
    pub async fn register(
        &self,
        user_id: Uuid,
    ) -> (Uuid, mpsc::UnboundedReceiver<String>, bool, Vec<Uuid>) {
        let conn_id = Uuid::now_v7();
        let (tx, rx) = mpsc::unbounded_channel();

        let mut guard = self.connections.write().await;
        let user_conns = guard.entry(user_id).or_default();
        let became_online = user_conns.is_empty();
        user_conns.insert(conn_id, tx);
        let online_user_ids: Vec<Uuid> = guard.keys().copied().collect();

        (conn_id, rx, became_online, online_user_ids)
    }

    /// Removes `conn_id` for `user_id`, returning `true` if the user has no remaining connections.
    pub async fn unregister(&self, user_id: Uuid, conn_id: Uuid) -> bool {
        let mut guard = self.connections.write().await;
        let Some(user_conns) = guard.get_mut(&user_id) else {
            return false;
        };
        user_conns.remove(&conn_id);
        if user_conns.is_empty() {
            guard.remove(&user_id);
            return true;
        }
        false
    }

    /// Broadcasts `event` to all connected WebSocket clients.
    pub async fn broadcast(&self, event: &ServerEvent) {
        let Ok(payload) = serde_json::to_string(event) else {
            tracing::error!("failed to serialize ServerEvent");
            return;
        };

        let guard = self.connections.read().await;
        for user_conns in guard.values() {
            for tx in user_conns.values() {
                let _ = tx.send(payload.clone());
            }
        }
    }

    /// Sends `event` to all active connections of `user_id`.
    #[allow(dead_code)]
    pub async fn send_to_user(&self, user_id: Uuid, event: &ServerEvent) {
        let Ok(payload) = serde_json::to_string(event) else {
            tracing::error!("failed to serialize ServerEvent");
            return;
        };

        let guard = self.connections.read().await;
        if let Some(user_conns) = guard.get(&user_id) {
            for tx in user_conns.values() {
                let _ = tx.send(payload.clone());
            }
        }
    }
}
