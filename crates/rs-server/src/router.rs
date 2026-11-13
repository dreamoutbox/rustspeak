use std::path::{Path, PathBuf};

use axum::Router;
use axum::routing::{get, patch, post};
use tower_http::services::{ServeDir, ServeFile};

use crate::routes;
use crate::state::AppState;
use crate::ws;

/// Builds the application Axum `Router` with all API routes and default `web/dist` static serving.
#[cfg_attr(not(test), allow(dead_code))]
pub fn build_router(state: AppState) -> Router {
    build_router_with_static(state, &PathBuf::from("web/dist"))
}

/// Builds the application Axum `Router` with all API routes and SPA static file serving from `static_dir`.
pub fn build_router_with_static(state: AppState, static_dir: &Path) -> Router {
    let index_html = static_dir.join("index.html");
    let spa_service = ServeDir::new(static_dir).not_found_service(ServeFile::new(index_html));

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
        .route(
            "/api/voice/channels",
            get(routes::voice::list_channels).post(routes::voice::create_channel),
        )
        .route(
            "/api/voice/channels/{id}",
            axum::routing::delete(routes::voice::delete_channel),
        )
        .fallback_service(spa_service)
        .with_state(state)
}
