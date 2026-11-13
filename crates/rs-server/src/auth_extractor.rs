use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use rs_common::{Claims, Role};

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

/// Verifies that `claims.role` meets or exceeds `min_role`.
pub fn require_role(claims: &Claims, min_role: Role) -> Result<(), ApiError> {
    if claims.role < min_role {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "insufficient permissions",
        ));
    }
    Ok(())
}

/// Axum extractor requiring at least `Role::Admin`.
pub struct RequireAdmin(pub Claims);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
        require_role(&claims, Role::Admin)?;
        Ok(Self(claims))
    }
}
