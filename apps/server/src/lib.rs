use axum::{
    extract::State,
    http::HeaderName,
    routing::{get, post},
    Json, Router,
};
use legalrust_application::{ApiResponse, AppError};
use legalrust_infrastructure::AppState;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use validator::Validate;

const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Deserialize, Validate)]
struct LoginRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

async fn healthz() -> Json<Value> {
    Json(
        json!({"data": {"status": "ok", "service": "legalrust-server"}, "meta": {"version": "v1"}}),
    )
}

async fn version(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::new(
        json!({"version": state.version}),
        "bootstrap".into(),
    ))
}

async fn readyz(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    match &state.pool {
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => Ok(Json(json!({"ready": true}))),
            Err(_) => Err(AppError::BlockedExternal("database unreachable".into())),
        },
        None => Err(AppError::BlockedExternal("database not configured".into())),
    }
}

async fn login(Json(body): Json<LoginRequest>) -> Result<Json<Value>, AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Err(AppError::BlockedExternal(
        "user store offline in scaffold; Phase 4 wires DB-backed login".into(),
    ))
}

async fn not_found() -> AppError {
    AppError::NotFound("route".into())
}

pub fn build_router(state: AppState) -> Router {
    let request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/version", get(version))
        .route("/api/v1/auth/login", post(login))
        .fallback(not_found)
        .layer(SetRequestIdLayer::new(request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(request_id))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub fn dev_state() -> AppState {
    AppState {
        version: "0.1.0",
        pool: None,
    }
}
