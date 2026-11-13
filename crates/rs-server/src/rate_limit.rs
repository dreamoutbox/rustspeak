use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tokio::sync::Mutex;

use crate::error::ApiError;
use crate::state::AppState;

struct WindowEntry {
    count: u32,
    window_start: Instant,
}

/// Fixed-window in-memory rate limiter keyed by client identifier.
#[derive(Clone)]
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    entries: Arc<Mutex<HashMap<String, WindowEntry>>>,
}

impl RateLimiter {
    /// Creates a new `RateLimiter` allowing `max_requests` per `window`.
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns `true` if `key` is within the allowed request budget, or `false` if rate-limited.
    pub async fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut guard = self.entries.lock().await;

        // Periodically prune expired windows to prevent unbounded memory growth.
        if guard.len() > 1024 {
            guard.retain(|_, entry| now.duration_since(entry.window_start) < self.window);
        }

        match guard.get_mut(key) {
            Some(entry) if now.duration_since(entry.window_start) < self.window => {
                if entry.count >= self.max_requests {
                    false
                } else {
                    entry.count += 1;
                    true
                }
            }
            _ => {
                guard.insert(
                    key.to_string(),
                    WindowEntry {
                        count: 1,
                        window_start: now,
                    },
                );
                true
            }
        }
    }
}

fn extract_client_key(req: &Request) -> String {
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }
    if let Some(forwarded) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return forwarded.to_string();
    }
    "local".to_string()
}

/// Axum middleware enforcing the authentication endpoint rate limit.
pub async fn rate_limit_auth(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let key = extract_client_key(&req);
    if !state.auth_limiter.check(&key).await {
        return ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too many authentication requests; try again later",
        )
        .into_response();
    }
    next.run(req).await
}

/// Axum middleware enforcing the message-posting rate limit (POST requests only).
pub async fn rate_limit_messages(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    if req.method() == axum::http::Method::POST {
        let key = extract_client_key(&req);
        if !state.message_limiter.check(&key).await {
            return ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "message rate limit exceeded; slow down",
            )
            .into_response();
        }
    }
    next.run(req).await
}
