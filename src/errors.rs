use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("template rendering failed")]
    Template(#[from] askama::Error),

    #[error("Auth0 API error: {0}")]
    Auth0(String),

    #[error("not found")]
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Template(e) => {
                tracing::error!(error = ?e, "template rendering failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
            }
            AppError::Auth0(msg) => {
                tracing::error!(error = %msg, "Auth0 API error");
                (StatusCode::BAD_GATEWAY, "Upstream error").into_response()
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
        }
    }
}
