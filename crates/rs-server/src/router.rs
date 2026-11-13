use axum::Router;
use axum::routing::{get, patch, post};

use crate::routes;
use crate::state::AppState;
use crate::ws;

/// Builds the application Axum `Router` with all API routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/ws", get(ws::handler::ws_handler))
        .route("/api/auth/register", post(routes::auth::register))
        .route("/api/auth/login", post(routes::auth::login))
        .route("/api/auth/refresh", post(routes::auth::refresh))
        .route("/api/auth/logout", post(routes::auth::logout))
        .route("/api/users/me", get(routes::auth::me))
        .route("/api/admin/users", get(routes::admin::list_users))
        .route(
            "/api/admin/users/{id}/role",
            patch(routes::admin::change_role),
        )
        .route(
            "/api/admin/users/{id}/disabled",
            patch(routes::admin::set_disabled),
        )
        .route(
            "/api/channels",
            get(routes::channels::list_channels).post(routes::channels::create_channel),
        )
        .route(
            "/api/channels/{id}",
            axum::routing::delete(routes::channels::delete_channel),
        )
        .route(
            "/api/channels/{id}/messages",
            get(routes::channels::get_history).post(routes::channels::post_message),
        )
        .route(
            "/api/dms",
            get(routes::dms::list_dms).post(routes::dms::open_dm),
        )
        .route(
            "/api/dms/{id}/messages",
            get(routes::dms::get_dm_history).post(routes::dms::send_dm),
        )
        .with_state(state)
}
