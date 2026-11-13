use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// Health check endpoint (`GET /api/health`).
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

