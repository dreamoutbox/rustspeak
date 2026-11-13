use axum::Router;
use axum::routing::{get, post};

use crate::routes;
use crate::state::AppState;

/// Builds the application Axum `Router` with all API routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/api/auth/register", post(routes::auth::register))
        .route("/api/auth/login", post(routes::auth::login))
        .route("/api/auth/refresh", post(routes::auth::refresh))
        .route("/api/auth/logout", post(routes::auth::logout))
        .route("/api/users/me", get(routes::auth::me))
        .with_state(state)
}

