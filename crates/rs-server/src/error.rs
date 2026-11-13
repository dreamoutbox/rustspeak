use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rs_user::UserError;
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

/// HTTP error wrapper converting domain errors into JSON responses.
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    /// Creates an `ApiError` with explicit HTTP `status` and `message`.
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl From<UserError> for ApiError {
    fn from(err: UserError) -> Self {
        let status = match &err {
            UserError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            UserError::UsernameTaken => StatusCode::CONFLICT,
            UserError::InvalidCredentials | UserError::InvalidToken | UserError::TokenReused => {
                StatusCode::UNAUTHORIZED
            }
            UserError::UserDisabled | UserError::Forbidden(_) => StatusCode::FORBIDDEN,
            UserError::NotFound => StatusCode::NOT_FOUND,
            UserError::PasswordHash(_) | UserError::JwtEncode(_) | UserError::Database(_) => {
                tracing::error!(error = %err, "internal user domain error");
                return Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
            }
            UserError::Common(_) => {
                tracing::error!(error = %err, "internal common error");
                return Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
            }
        };
        Self::new(status, err.to_string())
    }
}
