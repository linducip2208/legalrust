use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("external dependency blocked: {0}")]
    BlockedExternal(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}
impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::Validation(_) => "VALIDATION",
            AppError::Conflict(_) => "CONFLICT",
            AppError::BlockedExternal(_) => "EXTERNAL_BLOCKED",
            AppError::Internal(_) => "INTERNAL",
        }
    }
    pub fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::BlockedExternal(_) => StatusCode::NOT_IMPLEMENTED,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({"error": {"code": self.code(), "message": self.to_string()}}));
        (self.status(), body).into_response()
    }
}
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    pub meta: Meta,
}
#[derive(Debug, Serialize)]
pub struct Meta {
    pub version: &'static str,
    pub request_id: String,
}
impl<T: Serialize> ApiResponse<T> {
    pub fn new(data: T, request_id: String) -> Self {
        Self {
            data,
            meta: Meta {
                version: "v1",
                request_id,
            },
        }
    }
}
#[derive(Debug, Deserialize, validator::Validate)]
pub struct Pagination {
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    #[validate(range(min = 1, max = 200))]
    pub per_page: Option<u32>,
}
impl Pagination {
    pub fn normalized(self) -> (u32, u32) {
        (
            self.page.unwrap_or(1).max(1),
            self.per_page.unwrap_or(20).min(200),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_codes_map_to_status() {
        assert_eq!(AppError::Unauthorized.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            AppError::BlockedExternal("x".into()).status(),
            StatusCode::NOT_IMPLEMENTED
        );
    }
    #[test]
    fn pagination_clamps() {
        let (p, n) = Pagination {
            page: Some(0),
            per_page: Some(999),
        }
        .normalized();
        assert_eq!((p, n), (1, 200));
    }
}
