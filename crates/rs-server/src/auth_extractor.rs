use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use rs_common::Claims;

use crate::error::ApiError;
use crate::state::AppState;

const BEARER_PREFIX: &str = "Bearer ";

/// Axum extractor that validates a Bearer JWT access token and yields `Claims`.
pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                ApiError::new(StatusCode::UNAUTHORIZED, "missing authorization header")
            })?;

        let token = header.strip_prefix(BEARER_PREFIX).ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "invalid authorization scheme")
        })?;

        let claims = state.token_service.verify_access_token(token)?;
        Ok(Self(claims))
    }
}
