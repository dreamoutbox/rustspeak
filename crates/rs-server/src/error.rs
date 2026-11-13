use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rs_text::TextError;
use rs_user::UserError;
use rs_voice::VoiceError;
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

impl From<VoiceError> for ApiError {
    fn from(err: VoiceError) -> Self {
        let status = match &err {
            VoiceError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            VoiceError::ChannelNameTaken => StatusCode::CONFLICT,
            VoiceError::NotFound => StatusCode::NOT_FOUND,
            VoiceError::Forbidden(_) => StatusCode::FORBIDDEN,
            VoiceError::WebRtc(_) | VoiceError::Database(_) | VoiceError::Common(_) => {
                tracing::error!(error = %err, "internal voice domain error");
                return Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
            }
        };
        Self::new(status, err.to_string())
    }
}

impl From<TextError> for ApiError {
    fn from(err: TextError) -> Self {
        let status = match &err {
            TextError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            TextError::ChannelNameTaken => StatusCode::CONFLICT,
            TextError::NotFound => StatusCode::NOT_FOUND,
            TextError::Forbidden(_) => StatusCode::FORBIDDEN,
            TextError::Database(_) | TextError::Common(_) => {
                tracing::error!(error = %err, "internal text domain error");
                return Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
            }
        };
        Self::new(status, err.to_string())
    }
}
