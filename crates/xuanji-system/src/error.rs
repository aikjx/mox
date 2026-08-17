//! 统一错误类型
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("资源不存在: {0}")]
    NotFound(String),
    #[error("未认证: {0}")]
    Unauthorized(String),
    #[error("权限不足: {0}")]
    Forbidden(String),
    #[error("状态非法: {0}")]
    InvalidState(String),
    #[error("请求非法: {0}")]
    BadRequest(String),
    #[error("资源冲突: {0}")]
    Conflict(String),
    #[error("内部错误: {0}")]
    Internal(String),
}

pub type Result<T, E = AppError> = std::result::Result<T, E>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::InvalidState(_) | AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, axum::Json(json!({ "error": self.to_string() }))).into_response()
    }
}
