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
    #[error("rate limited")]
    RateLimited,
    #[error("locked: {0}")]
    Locked(String),
    #[error("external dependency blocked: {0}")]
    BlockedExternal(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}
impl AppError {
    pub fn too_many_requests() -> Self {
        Self::RateLimited
    }
    pub fn locked(msg: impl Into<String>) -> Self {
        Self::Locked(msg.into())
    }
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::Validation(_) => "VALIDATION",
            AppError::Conflict(_) => "CONFLICT",
            AppError::RateLimited => "RATE_LIMITED",
            AppError::Locked(_) => "LOCKED",
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
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::Locked(_) => StatusCode::LOCKED,
            AppError::BlockedExternal(_) => StatusCode::NOT_IMPLEMENTED,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Client-facing message. Self-audit hardening 2026-11-17: `Internal`
    /// (which wraps arbitrary `anyhow` chains incl. `sqlx`/`StoreError::Db`
    /// text) MUST render as a fixed string so DB internals can never reach
    /// the wire even if a handler forwards `e.to_string()`. All other
    /// variants carry only caller-supplied, validation-style messages.
    pub fn public_message(&self) -> String {
        match self {
            AppError::Internal(_) => "internal error".to_string(),
            _ => self.to_string(),
        }
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({"error": {"code": self.code(), "message": self.public_message()}}));
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
            AppError::too_many_requests().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(AppError::locked("x").status(), StatusCode::LOCKED);
        assert_eq!(AppError::locked("x").code(), "LOCKED");
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
    // --- self-audit hardening tests (2026-11-17) ---
    #[test]
    fn internal_never_leaks_db_detail() {
        // Simulate a handler forwarding a sqlx/StoreError chain verbatim.
        let leaked = AppError::Internal(anyhow::anyhow!(
            "SELECT * FROM users WHERE id=1: Column 'pw_hash' not found (sqlx mysql)"
        ));
        assert_eq!(leaked.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(leaked.code(), "INTERNAL");
        assert_eq!(leaked.public_message(), "internal error");
        assert!(!leaked.public_message().contains("SELECT"));
        assert!(!leaked.public_message().contains("sqlx"));
    }
    #[test]
    fn validation_messages_stay_client_safe() {
        // allowlist-style messages remain visible to callers.
        let v = AppError::Validation("password invalid".into());
        assert_eq!(v.public_message(), "validation failed: password invalid");
        assert_eq!(v.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    #[test]
    fn blocked_external_maps_to_501() {
        let b = AppError::BlockedExternal("ocr engine unavailable".into());
        assert_eq!(b.status(), StatusCode::NOT_IMPLEMENTED);
        assert_eq!(b.code(), "EXTERNAL_BLOCKED");
    }
}
