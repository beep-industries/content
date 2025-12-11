use axum::{http::StatusCode, response::IntoResponse};
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("HttpServerError: {0}")]
    HttpServer(String),
    #[error("S3Error: {0}")]
    S3EndpointError(String),
    #[error("SigningKeyError: {0}")]
    SigningKeyError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("OpenTelemetryError: {0}")]
    OpenTelemetry(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ValidationError {
    pub message: String,
    pub field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema)]
pub enum ApiError {
    InternalServerError(String),
    #[allow(dead_code)]
    UnProcessableEntity(String),
    #[allow(dead_code)]
    NotFound(String),
    #[allow(dead_code)]
    Unauthorized(String),
    #[allow(dead_code)]
    Forbidden(String),
    #[allow(dead_code)]
    BadRequest(String),
    #[allow(dead_code)]
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::InternalServerError(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
            }
            ApiError::UnProcessableEntity(errors) => {
                (StatusCode::UNPROCESSABLE_ENTITY, errors).into_response()
            }
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message).into_response(),
            ApiError::Forbidden(message) => (StatusCode::FORBIDDEN, message).into_response(),
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            ApiError::ServiceUnavailable(message) => {
                (StatusCode::SERVICE_UNAVAILABLE, message).into_response()
            }
        }
    }
}
