#!/usr/bin/env python3
"""Impl part 2: server routes (auth+RBAC+core+DMS), migration 0004, tests, CI, docs."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

w("migrations/0004_client_optional.sql", """-- 0004: matters may start without a linked client (intake-first flow)
ALTER TABLE matters MODIFY client_id BIGINT UNSIGNED NULL;
""")

w("apps/server/Cargo.toml", """[package]
name = "legalrust-server"
version = "0.1.0"
edition = "2021"

[lib]
name = "legalrust_server"
path = "src/lib.rs"

[[bin]]
name = "legalrust-server"
path = "src/main.rs"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.7", features = ["trace", "cors", "request-id", "limit"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }
validator = { version = "0.20", features = ["derive"] }
sqlx = { version = "0.9", features = ["runtime-tokio", "mysql"] }
legalrust-application = { path = "../../crates/application" }
legalrust-infrastructure = { path = "../../crates/infrastructure" }
legalrust-database = { path = "../../crates/database" }
legalrust-auth = { path = "../../crates/auth" }
legalrust-domain = { path = "../../crates/domain" }
legalrust-authorization = { path = "../../crates/authorization" }
legalrust-document = { path = "../../crates/document" }
legalrust-storage = { path = "../../crates/storage" }
legalrust-audit = { path = "../../crates/audit" }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
""")

w("apps/server/src/lib.rs", r'''
use axum::{Json, Router, extract::{Path, Query, State}, http::{HeaderMap, HeaderName, HeaderValue, StatusCode}, response::IntoResponse, routing::{get, post, put}};
use legalrust_application::{ApiResponse, AppError};
use legalrust_audit::AuditEvent;
use legalrust_auth::{Claims, Session};
use legalrust_authorization::can;
use legalrust_database::{CoreStore, DocRow, MatterRow, TaskRow, UserStore};
use legalrust_domain::{Permission, Role};
use legalrust_infrastructure::AppState;
use legalrust_storage::{LocalStore, ObjectStore};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer}, trace::TraceLayer};
use validator::Validate;

const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_UPLOAD: u64 = 50 * 1024 * 1024;

#[derive(Clone)]
pub struct Stores {
    pub users: Arc<dyn UserStore>, pub core: Arc<dyn CoreStore>, pub objects: Arc<dyn ObjectStore>,
    pub jwt_secret: Vec<u8>,
}
#[derive(Clone)]
pub struct FullState { pub version: &'static str, pub pool: Option<sqlx::MySqlPool>, pub stores: Stores }

pub struct AuthUser { pub claims: Claims }
async fn auth_user(headers: &HeaderMap, state: &FullState) -> Result<AuthUser, AppError> {
    let v = headers.get("authorization").ok_or(AppError::Unauthorized)?.to_str().map_err(|_| AppError::Unauthorized)?;
    let tok = v.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    let claims = legalrust_auth::validate_token(&state.stores.jwt_secret, tok).map_err(|_| AppError::Unauthorized)?;
    if !state.stores.users.session_live(&claims.sid).await.unwrap_or(false) { return Err(AppError::Unauthorized); }
    Ok(AuthUser { claims })
}
fn authorize(user: &AuthUser, perm: Permission) -> Result<(), AppError> {
    if user.claims.roles.iter().filter_map(|r| Role::from_str(r)).any(|r| can(r, perm)) { Ok(()) } else { Err(AppError::Forbidden) }
}
async fn audit(pool: &Option<sqlx::MySqlPool>, e: AuditEvent) {
    if let Some(p) = pool { let _ = legalrust_audit::record(p, &e).await; }
}

#[derive(Debug, Deserialize, Validate)]
struct RegisterReq { #[validate(length(min = 2))] org_name: String, #[validate(email)] email: String, #[validate(length(min = 2))] name: String, #[validate(length(min = 8))] password: String }
#[derive(Debug, Deserialize, Validate)]
struct LoginReq { #[validate(email)] email: String, #[validate(length(min = 8))] password: String }
async fn register(State(s): State<FullState>, Json(b): Json<RegisterReq>) -> Result<Json<Value>, AppError> {
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let r: Session = legalrust_auth::register(s.stores.users.as_ref(), &s.stores.jwt_secret, &b.org_name, &b.email, &b.name, &b.password).await?;
    audit(&s.pool, AuditEvent { tenant_id: r.org_id.clone(), actor_id: Some(r.user_id.clone()), action: "org.registered".into(), entity: "organization".into(), entity_id: 0, ip: None }).await;
    Ok(Json(json!({"user_id": r.user_id, "org_id": r.org_id, "token": r.token})))
}
async fn login(State(s): State<FullState>, Json(b): Json<LoginReq>) -> Result<Json<Value>, AppError> {
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let r: Session = legalrust_auth::login(s.stores.users.as_ref(), &s.stores.jwt_secret, &b.email, &b.password).await?;
    Ok(Json(json!({"user_id": r.user_id, "org_id": r.org_id, "token": r.token})))
}
async fn logout(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    s.stores.users.revoke_session(&u.claims.sid).await.map_err(|_| AppError::Internal(anyhow::anyhow!("revoke")))?;
    Ok(Json(json!({"revoked": true})))
}
async fn me(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    Ok(Json(json!({"user_id": u.claims.sub, "org_id": u.claims.org, "roles": u.claims.roles})))
}

#[derive(Debug, Deserialize, Validate)]
struct ClientReq { #[validate(length(min = 2))] display_name: String, kind: String }
async fn create_client(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<ClientReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    if !["individual", "corporate", "government"].contains(&b.kind.as_str()) { return Err(AppError::Validation("kind".into())); }
    let c = s.stores.core.create_client(&u.claims.org, &b.kind, &b.display_name).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!({"id": c.id, "display_name": c.display_name})))
}
async fn list_clients(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::View)?;
    let v = s.stores.core.list_clients(&u.claims.org).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v.iter().map(|c| json!({"id": c.id, "display_name": c.display_name, "kind": c.kind})).collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct MatterReq { #[validate(length(min = 2))] title: String, client_id: Option<i64> }
async fn create_matter(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<MatterReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let number = format!("M-{}", &uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
    let m: MatterRow = s.stores.core.create_matter(&u.claims.org, b.client_id, &number, &b.title).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    audit(&s.pool, AuditEvent { tenant_id: u.claims.org.clone(), actor_id: Some(u.claims.sub.clone()), action: "matter.created".into(), entity: "matter".into(), entity_id: m.id, ip: None }).await;
    Ok(Json(json!({"id": m.id, "number": m.number})))
}
async fn list_matters(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::View)?;
    let v = s.stores.core.list_matters(&u.claims.org).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v.iter().map(|m| json!({"id": m.id, "number": m.number, "title": m.title, "status": m.status})).collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct TaskReq { #[validate(length(min = 2))] title: String }
async fn create_task(State(s): State<FullState>, headers: HeaderMap, Path(mid): Path<i64>, Json(b): Json<TaskReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let t: TaskRow = s.stores.core.create_task(&u.claims.org, mid, &b.title).await.map_err(|e| match e { legalrust_database::StoreError::NotFound => AppError::NotFound("matter".into()), _ => AppError::Internal(anyhow::anyhow!(e.to_string())) })?;
    Ok(Json(json!({"id": t.id, "status": t.status})))
}
async fn list_tasks(State(s): State<FullState>, headers: HeaderMap, Path(mid): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::View)?;
    let v = s.stores.core.list_tasks(&u.claims.org, mid).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v.iter().map(|t| json!({"id": t.id, "title": t.title, "status": t.status})).collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct TaskPatch { status: String }
async fn patch_task(State(s): State<FullState>, headers: HeaderMap, Path(tid): Path<i64>, Json(b): Json<TaskPatch>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::Update)?;
    let t = s.stores.core.set_task_status(&u.claims.org, tid, &b.status).await.map_err(|e| match e { legalrust_database::StoreError::NotFound => AppError::NotFound("task".into()), legalrust_database::StoreError::Conflict(m) => AppError::Conflict(m), _ => AppError::Internal(anyhow::anyhow!(e.to_string())) })?;
    Ok(Json(json!({"id": t.id, "status": t.status})))
}

#[derive(Debug, Deserialize, Validate)]
struct DocReq { matter_id: Option<i64>, #[validate(length(min = 1))] name: String, mime: String, size: u64, sha: String }
async fn create_doc(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<DocReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    legalrust_document::validate_upload(&b.mime, b.size, MAX_UPLOAD).map_err(|e| AppError::Validation(e.to_string()))?;
    if b.sha.len() != 64 { return Err(AppError::Validation("sha256".into())); }
    let d: DocRow = s.stores.core.create_doc(&u.claims.org, b.matter_id, &b.name, &b.mime, b.size, &b.sha).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!({"id": d.id, "upload": format!("/api/v1/documents/{}/content", d.id)})))
}
async fn get_doc(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::View)?;
    let (d, vs) = s.stores.core.get_doc(&u.claims.org, id).await.map_err(|_| AppError::NotFound("document".into()))?;
    Ok(Json(json!({"id": d.id, "name": d.name, "mime": d.mime, "size": d.size, "sha": d.sha, "versions": vs.iter().map(|v| json!({"v": v.version_no, "key": v.object_key})).collect::<Vec<_>>()})))
}
async fn list_docs(State(s): State<FullState>, headers: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::View)?;
    let matter = q.get("matter_id").and_then(|v| v.parse().ok());
    let v = s.stores.core.list_docs(&u.claims.org, matter).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v.iter().map(|d| json!({"id": d.id, "name": d.name})).collect::<Vec<_>>())))
}
async fn put_content(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>, body: axum::body::Bytes) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::Create)?;
    let (d, _) = s.stores.core.get_doc(&u.claims.org, id).await.map_err(|_| AppError::NotFound("document".into()))?;
    if body.len() as u64 != d.size { return Err(AppError::Validation("size mismatch".into())); }
    let sha = legalrust_document::sha256_hex(&body);
    if sha != d.sha { return Err(AppError::Validation("sha256 mismatch".into())); }
    let key = format!("docs/{}/content", id);
    s.stores.objects.put(&key, &body).await?;
    let v = s.stores.core.add_version(&u.claims.org, id, &key, &sha).await.map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!({"version": v.version_no})))
}
async fn get_content(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?; authorize(&u, Permission::Download)?;
    let (d, vs) = s.stores.core.get_doc(&u.claims.org, id).await.map_err(|_| AppError::NotFound("document".into()))?;
    let v = vs.last().ok_or_else(|| AppError::NotFound("content".into()))?;
    let bytes = s.stores.objects.get(&v.object_key).await?;
    let mut h = HeaderMap::new();
    h.insert("content-type", d.mime.parse().unwrap_or(HeaderValue::from_static("application/octet-stream")));
    Ok((StatusCode::OK, h, bytes))
}

async fn healthz() -> Json<Value> { Json(json!({"data": {"status": "ok", "service": "legalrust-server"}, "meta": {"version": "v1"}})) }
async fn version(State(s): State<FullState>) -> Json<ApiResponse<Value>> { Json(ApiResponse::new(json!({"version": s.version}), "bootstrap".into())) }
async fn readyz(State(s): State<FullState>) -> Result<Json<Value>, AppError> {
    match &s.pool {
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => Ok(Json(json!({"ready": true, "db": true}))),
            Err(_) => Err(AppError::BlockedExternal("database unreachable".into())),
        },
        None => Err(AppError::BlockedExternal("database not configured; ephemeral dev store active".into())),
    }
}
async fn not_found() -> AppError { AppError::NotFound("route".into()) }

pub fn build_router(state: FullState) -> Router {
    let request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/version", get(version))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/orgs/me", get(me))
        .route("/api/v1/clients", post(create_client).get(list_clients))
        .route("/api/v1/matters", post(create_matter).get(list_matters))
        .route("/api/v1/matters/:mid/tasks", post(create_task).get(list_tasks))
        .route("/api/v1/tasks/:tid", post(patch_task))
        .route("/api/v1/documents", post(create_doc).get(list_docs))
        .route("/api/v1/documents/:id", get(get_doc))
        .route("/api/v1/documents/:id/content", put(put_content).get(get_content))
        .fallback(not_found)
        .layer(RequestBodyLimitLayer::new((MAX_UPLOAD + 1024 * 1024) as usize))
        .layer(SetRequestIdLayer::new(request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(request_id))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub fn dev_state() -> FullState {
    let m = std::sync::Arc::new(legalrust_database::memory::MemoryStores::default());
    FullState {
        version: "0.1.0", pool: None,
        stores: Stores { users: m.clone(), core: m, objects: Arc::new(LocalStore::new(std::env::temp_dir().join("lr-dev-objects"))) , jwt_secret: b"test-secret".to_vec() },
    }
}
''')

w("apps/server/src/main.rs", r'''
use legalrust_database as db;
use legalrust_infrastructure::{Config, init_tracing};
use legalrust_server::{FullState, Stores};
use legalrust_storage::LocalStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let config = Config::from_env();
    let secret = std::env::var("JWT_SECRET").unwrap_or_default().into_bytes();
    if secret.is_empty() { tracing::warn!("JWT_SECRET is not set; auth endpoints return 501 until configured"); }
    let pool = match tokio::time::timeout(Duration::from_secs(3), db::connect(&config.database_url)).await {
        Ok(Ok(p)) => {
            tracing::info!("database connected; running migrations");
            if let Err(e) = db::migrate(&p).await { tracing::error!("migration failed: {e}"); }
            Some(p)
        }
        _ => { tracing::warn!("database unreachable; ephemeral dev store active (non-production)"); None }
    };
    let (users, core): (Arc<dyn legalrust_database::UserStore>, Arc<dyn legalrust_database::CoreStore>) = match &pool {
        Some(p) => { let m = Arc::new(legalrust_database::mysql::MysqlStores::new(p.clone())); (m.clone(), m) }
        None => { let m = Arc::new(legalrust_database::memory::MemoryStores::default()); (m.clone(), m) }
    };
    let state = FullState {
        version: "0.1.0", pool,
        stores: Stores { users, core, objects: Arc::new(LocalStore::new(std::path::PathBuf::from("./var/objects"))), jwt_secret: secret },
    };
    let app = legalrust_server::build_router(state);
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!("listening on {}", config.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await?;
    Ok(())
}
''')
print("IMPL2A OK")
