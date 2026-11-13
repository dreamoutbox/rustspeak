use std::path::{Path, PathBuf};

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::HeaderValue;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, patch, post};
use tower_http::services::{ServeDir, ServeFile};

use crate::rate_limit::{rate_limit_auth, rate_limit_messages};
use crate::routes;
use crate::state::AppState;
use crate::ws;

const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;
const CSP_HEADER: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' wss: ws:; img-src 'self' data:; media-src 'self' blob:";

async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static(CSP_HEADER),
    );
    response
}

/// Builds the application Axum `Router` with all API routes and default `web/dist` static serving.
#[cfg_attr(not(test), allow(dead_code))]
pub fn build_router(state: AppState) -> Router {
    build_router_with_static(state, &PathBuf::from("web/dist"))
}

/// Builds the application Axum `Router` with all API routes, security hardening, and SPA static serving.
pub fn build_router_with_static(state: AppState, static_dir: &Path) -> Router {
    let index_html = static_dir.join("index.html");
    let spa_service = ServeDir::new(static_dir).not_found_service(ServeFile::new(index_html));

    let auth_routes = Router::new()
        .route("/api/auth/register", post(routes::auth::register))
        .route("/api/auth/login", post(routes::auth::login))
        .route("/api/auth/refresh", post(routes::auth::refresh))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_auth,
        ));

    let message_routes = Router::new()
        .route(
            "/api/channels/{id}/messages",
            get(routes::channels::get_history).post(routes::channels::post_message),
        )
        .route(
            "/api/dms/{id}/messages",
            get(routes::dms::get_dm_history).post(routes::dms::send_dm),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_messages,
        ));

    Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/ws", get(ws::handler::ws_handler))
        .route("/api/auth/logout", post(routes::auth::logout))
        .route("/api/users/me", get(routes::auth::me))
        .route("/api/users", get(routes::auth::list_directory))
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
            "/api/dms",
            get(routes::dms::list_dms).post(routes::dms::open_dm),
        )
        .route("/api/voice/config", get(routes::voice::get_config))
        .route(
            "/api/voice/channels",
            get(routes::voice::list_channels).post(routes::voice::create_channel),
        )
        .route(
            "/api/voice/channels/{id}",
            axum::routing::delete(routes::voice::delete_channel),
        )
        .merge(auth_routes)
        .merge(message_routes)
        .fallback_service(spa_service)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}
