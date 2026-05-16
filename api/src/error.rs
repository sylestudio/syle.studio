use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Flat API error mapped to a status code; bodies are intentionally terse so
/// the tunneled admin surface leaks nothing.
#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    NotFound,
    BadRequest,
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest => StatusCode::BAD_REQUEST,
            ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        code.into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => ApiError::NotFound,
            _ => ApiError::Internal,
        }
    }
}
