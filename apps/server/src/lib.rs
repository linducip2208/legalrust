use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use legalrust_application::{ApiResponse, AppError};
use legalrust_audit::AuditEvent;
use legalrust_auth::{Claims, Session};
use legalrust_authorization::can;
use legalrust_database::{CoreStore, DocRow, MatterRow, TaskRow, UserStore};
use legalrust_database::{DeedRow, InvoiceRow, OcrJobRow, OpsStore, TemplateRow};
use legalrust_domain::{Permission, Role};
use legalrust_storage::{LocalStore, ObjectStore};
use legalrust_workflow::{transition, DeedState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::{
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use validator::Validate;

const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_UPLOAD: u64 = 50 * 1024 * 1024;

// Auth brute-force guard: max failed logins per email inside a fixed window.
const LOGIN_MAX_ATTEMPTS: u32 = 5;
const LOGIN_LOCK_WINDOW: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<std::sync::Mutex<HashMap<String, (u32, Instant)>>>,
    max: u32,
}
impl RateLimiter {
    pub fn new(max: u32) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(HashMap::new())),
            max,
        }
    }
    pub fn check(&self, key: &str) -> bool {
        let mut m = self.inner.lock().unwrap();
        let now = Instant::now();
        let e = m.entry(key.into()).or_insert((0, now));
        if now.duration_since(e.1) > Duration::from_secs(60) {
            *e = (0, now);
        }
        if e.0 >= self.max {
            return false;
        }
        e.0 += 1;
        true
    }
}

#[derive(Clone)]
pub struct Stores {
    pub users: Arc<dyn UserStore>,
    pub core: Arc<dyn CoreStore>,
    pub ops: Arc<dyn OpsStore>,
    pub objects: Arc<dyn ObjectStore>,
    pub jwt_secret: Vec<u8>,
    pub integration_secret: Vec<u8>,
    pub integrations: Vec<String>,
    pub limiter: RateLimiter,
    pub events: tokio::sync::broadcast::Sender<String>,
    /// Failed-login counter keyed by normalized email: (attempts, window_start).
    pub login_failures: Arc<std::sync::Mutex<HashMap<String, (u32, Instant)>>>,
}
impl Stores {
    pub fn new(
        users: Arc<dyn UserStore>,
        core: Arc<dyn CoreStore>,
        ops: Arc<dyn OpsStore>,
        objects: Arc<dyn ObjectStore>,
        jwt_secret: Vec<u8>,
    ) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(64);
        Self {
            users,
            core,
            ops,
            objects,
            jwt_secret,
            integration_secret: Vec::new(),
            integrations: vec!["dukcapil".into(), "bpn".into(), "psre".into()],
            limiter: RateLimiter::new(20),
            events,
            login_failures: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}
fn login_locked(stores: &Stores, email: &str) -> bool {
    match stores.login_failures.lock().unwrap().get(email) {
        Some((n, t)) => *n >= LOGIN_MAX_ATTEMPTS && t.elapsed() < LOGIN_LOCK_WINDOW,
        None => false,
    }
}
fn login_note_failure(stores: &Stores, email: &str) -> u32 {
    let mut m = stores.login_failures.lock().unwrap();
    let now = Instant::now();
    let e = m.entry(email.to_string()).or_insert((0, now));
    if now.duration_since(e.1) >= LOGIN_LOCK_WINDOW {
        *e = (0, now);
    }
    e.0 += 1;
    e.0
}
#[derive(Clone)]
pub struct FullState {
    pub version: &'static str,
    pub pool: Option<sqlx::MySqlPool>,
    pub stores: Stores,
}

pub struct AuthUser {
    pub claims: Claims,
}
async fn auth_user(headers: &HeaderMap, state: &FullState) -> Result<AuthUser, AppError> {
    let v = headers
        .get("authorization")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;
    let tok = v.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
    match legalrust_auth::validate_token(&state.stores.jwt_secret, tok) {
        Ok(claims) => {
            if !state
                .stores
                .users
                .session_live(&claims.sid)
                .await
                .unwrap_or(false)
            {
                return Err(AppError::Unauthorized);
            }
            Ok(AuthUser { claims })
        }
        Err(_) => {
            let h = legalrust_document::sha256_hex(tok.as_bytes());
            let (uid, org) = state
                .stores
                .users
                .token_owner(&h)
                .await
                .map_err(|_| AppError::Unauthorized)?
                .ok_or(AppError::Unauthorized)?;
            let roles = state
                .stores
                .users
                .user_roles(&uid)
                .await
                .map_err(|_| AppError::Unauthorized)?;
            Ok(AuthUser {
                claims: Claims {
                    sub: uid,
                    org,
                    roles,
                    sid: "api-token".into(),
                    exp: usize::MAX,
                },
            })
        }
    }
}
fn authorize(user: &AuthUser, perm: Permission) -> Result<(), AppError> {
    if user
        .claims
        .roles
        .iter()
        .filter_map(|r| Role::parse(r))
        .any(|r| can(r, perm))
    {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
async fn audit(pool: &Option<sqlx::MySqlPool>, stores: &Stores, e: AuditEvent) {
    if let Some(p) = pool {
        let _ = legalrust_audit::record(p, &e).await;
    } else {
        let _ = stores
            .ops
            .audit_add(
                &e.tenant_id,
                e.actor_id.as_deref(),
                &e.action,
                &e.entity,
                e.entity_id,
            )
            .await;
    }
}

#[derive(Debug, Deserialize, Validate)]
struct RegisterReq {
    #[validate(length(min = 2))]
    org_name: String,
    #[validate(email)]
    email: String,
    #[validate(length(min = 2))]
    name: String,
    #[validate(length(min = 8))]
    password: String,
}
#[derive(Debug, Deserialize, Validate)]
struct LoginReq {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}
async fn register(
    State(s): State<FullState>,
    Json(b): Json<RegisterReq>,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("auth") {
        return Err(AppError::RateLimited);
    }
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let r: Session = legalrust_auth::register(
        s.stores.users.as_ref(),
        &s.stores.jwt_secret,
        &b.org_name,
        &b.email,
        &b.name,
        &b.password,
    )
    .await?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: r.org_id.clone(),
            actor_id: Some(r.user_id.clone()),
            action: "org.registered".into(),
            entity: "organization".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    Ok(Json(
        json!({"user_id": r.user_id, "org_id": r.org_id, "token": r.token}),
    ))
}
async fn login(
    State(s): State<FullState>,
    Json(b): Json<LoginReq>,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("auth") {
        return Err(AppError::RateLimited);
    }
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let email = b.email.trim().to_lowercase();
    if login_locked(&s.stores, &email) {
        return Err(AppError::RateLimited);
    }
    match legalrust_auth::login(
        s.stores.users.as_ref(),
        &s.stores.jwt_secret,
        &b.email,
        &b.password,
    )
    .await
    {
        Ok(r) => {
            s.stores.login_failures.lock().unwrap().remove(&email);
            Ok(Json(
                json!({"user_id": r.user_id, "org_id": r.org_id, "token": r.token}),
            ))
        }
        Err(e) => {
            if login_note_failure(&s.stores, &email) >= LOGIN_MAX_ATTEMPTS {
                return Err(AppError::RateLimited);
            }
            Err(e)
        }
    }
}
async fn logout(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    s.stores
        .users
        .revoke_session(&u.claims.sid)
        .await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("revoke")))?;
    Ok(Json(json!({"revoked": true})))
}
async fn me(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    Ok(Json(
        json!({"user_id": u.claims.sub, "org_id": u.claims.org, "roles": u.claims.roles}),
    ))
}

#[derive(Debug, Deserialize, Validate)]
struct ClientReq {
    #[validate(length(min = 2))]
    display_name: String,
    kind: String,
}
async fn create_client(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ClientReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if !["individual", "corporate", "government"].contains(&b.kind.as_str()) {
        return Err(AppError::Validation("kind".into()));
    }
    let c = s
        .stores
        .core
        .create_client(&u.claims.org, &b.kind, &b.display_name)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!({"id": c.id, "display_name": c.display_name})))
}
async fn list_clients(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .core
        .list_clients(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v
        .iter()
        .map(|c| json!({"id": c.id, "display_name": c.display_name, "kind": c.kind}))
        .collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct MatterReq {
    #[validate(length(min = 2))]
    title: String,
    client_id: Option<i64>,
}
async fn create_matter(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<MatterReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let number = format!("M-{}", uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
    let m: MatterRow = s
        .stores
        .core
        .create_matter(&u.claims.org, b.client_id, &number, &b.title)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    notify_user(
        &s,
        &u.claims.org,
        &u.claims.sub,
        "matter",
        &format!("Matter created: {}", m.number),
    )
    .await;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "matter.created".into(),
            entity: "matter".into(),
            entity_id: m.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": m.id, "number": m.number})))
}
async fn list_matters(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .core
        .list_matters(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v
        .iter()
        .map(|m| json!({"id": m.id, "number": m.number, "title": m.title, "status": m.status}))
        .collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct TaskReq {
    #[validate(length(min = 2))]
    title: String,
}
async fn create_task(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(mid): Path<i64>,
    Json(b): Json<TaskReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let t: TaskRow = s
        .stores
        .core
        .create_task(&u.claims.org, mid, &b.title)
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::NotFound => AppError::NotFound("matter".into()),
            _ => AppError::Internal(anyhow::anyhow!(e.to_string())),
        })?;
    Ok(Json(json!({"id": t.id, "status": t.status})))
}
async fn list_tasks(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(mid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .core
        .list_tasks(&u.claims.org, mid)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v
        .iter()
        .map(|t| json!({"id": t.id, "title": t.title, "status": t.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct TaskPatch {
    status: String,
}
async fn patch_task(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(tid): Path<i64>,
    Json(b): Json<TaskPatch>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    let t = s
        .stores
        .core
        .set_task_status(&u.claims.org, tid, &b.status)
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::NotFound => AppError::NotFound("task".into()),
            legalrust_database::StoreError::Conflict(m) => AppError::Conflict(m),
            _ => AppError::Internal(anyhow::anyhow!(e.to_string())),
        })?;
    Ok(Json(json!({"id": t.id, "status": t.status})))
}

#[derive(Debug, Deserialize, Validate)]
struct DocReq {
    matter_id: Option<i64>,
    #[validate(length(min = 1))]
    name: String,
    mime: String,
    size: u64,
    sha: String,
}
async fn create_doc(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DocReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    legalrust_document::validate_upload(&b.mime, b.size, MAX_UPLOAD)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if b.sha.len() != 64 {
        return Err(AppError::Validation("sha256".into()));
    }
    let d: DocRow = s
        .stores
        .core
        .create_doc(&u.claims.org, b.matter_id, &b.name, &b.mime, b.size, &b.sha)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(
        json!({"id": d.id, "upload": format!("/api/v1/documents/{}/content", d.id)}),
    ))
}
async fn get_doc(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (d, vs) = s
        .stores
        .core
        .get_doc(&u.claims.org, id)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    Ok(Json(
        json!({"id": d.id, "name": d.name, "mime": d.mime, "size": d.size, "sha": d.sha, "versions": vs.iter().map(|v| json!({"v": v.version_no, "key": v.object_key})).collect::<Vec<_>>()}),
    ))
}
async fn list_docs(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let matter = q.get("matter_id").and_then(|v| v.parse().ok());
    let v = s
        .stores
        .core
        .list_docs(&u.claims.org, matter)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v
        .iter()
        .map(|d| json!({"id": d.id, "name": d.name}))
        .collect::<Vec<_>>())))
}
async fn put_content(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let (d, _) = s
        .stores
        .core
        .get_doc(&u.claims.org, id)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    if body.len() as u64 != d.size {
        return Err(AppError::Validation("size mismatch".into()));
    }
    let sha = legalrust_document::sha256_hex(&body);
    if sha != d.sha {
        return Err(AppError::Validation("sha256 mismatch".into()));
    }
    let key = format!("docs/{}/content", id);
    s.stores.objects.put(&key, &body).await?;
    let v = s
        .stores
        .core
        .add_version(&u.claims.org, id, &key, &sha)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!({"version": v.version_no})))
}
async fn get_content(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Download)?;
    let (d, vs) = s
        .stores
        .core
        .get_doc(&u.claims.org, id)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let v = vs
        .last()
        .ok_or_else(|| AppError::NotFound("content".into()))?;
    let bytes = s.stores.objects.get(&v.object_key).await?;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        d.mime
            .parse()
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    Ok((StatusCode::OK, h, bytes))
}

async fn healthz() -> Json<Value> {
    Json(
        json!({"data": {"status": "ok", "service": "legalrust-server"}, "meta": {"version": "v1"}}),
    )
}
async fn version(State(s): State<FullState>) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::new(
        json!({"version": s.version}),
        "bootstrap".into(),
    ))
}
async fn readyz(State(s): State<FullState>) -> Result<Json<Value>, AppError> {
    match &s.pool {
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => Ok(Json(json!({"ready": true, "db": true}))),
            Err(_) => Err(AppError::BlockedExternal("database unreachable".into())),
        },
        None => Err(AppError::BlockedExternal(
            "database not configured; ephemeral dev store active".into(),
        )),
    }
}
// ---------- Phase 7-12 handlers ----------
fn map_store(e: legalrust_database::StoreError) -> AppError {
    match e {
        legalrust_database::StoreError::NotFound => AppError::NotFound("record".into()),
        legalrust_database::StoreError::Conflict(m) => AppError::Conflict(m),
        legalrust_database::StoreError::Db(m) => AppError::Internal(anyhow::anyhow!(m)),
    }
}

#[derive(Debug, Deserialize, Validate)]
struct OcrJobReq {
    document_id: i64,
    doc_type: String,
}
const DOC_TYPES: &[&str] = &[
    "ktp",
    "kk",
    "npwp",
    "passport",
    "land_certificate",
    "deed",
    "contract",
    "general",
];
async fn ocr_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<OcrJobReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    if !DOC_TYPES.contains(&b.doc_type.as_str()) {
        return Err(AppError::Validation("doc_type".into()));
    }
    s.stores
        .core
        .get_doc(&u.claims.org, b.document_id)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let id = s
        .stores
        .ops
        .ocr_create(&u.claims.org, b.document_id, &b.doc_type)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": "queued"})))
}
async fn ocr_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ocr_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|j: &OcrJobRow| json!({"id": j.id, "status": j.status, "doc_type": j.doc_type}))
        .collect::<Vec<_>>())))
}
async fn ocr_get(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (j, fs) = s
        .stores
        .ops
        .ocr_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"id": j.id, "status": j.status, "fields": fs.iter().map(|f| json!({"name": f.name, "value": f.value, "confidence": f.confidence})).collect::<Vec<_>>()}),
    ))
}
#[derive(Debug, Deserialize)]
struct OcrReviewReq {
    decision: String,
    note: Option<String>,
}
async fn ocr_review(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<OcrReviewReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Verify)?;
    if !["approve", "reject"].contains(&b.decision.as_str()) {
        return Err(AppError::Validation("decision".into()));
    }
    s.stores
        .ops
        .ocr_review(
            &u.claims.org,
            id,
            &u.claims.sub,
            &b.decision,
            b.note.as_deref(),
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"reviewed": b.decision})))
}
async fn ocr_process(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let (job, _) = s
        .stores
        .ops
        .ocr_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    // Estimate input size for honest logging (job stays queued; nothing processed).
    let est_bytes: u64 = s
        .stores
        .core
        .get_doc(&u.claims.org, job.doc)
        .await
        .map(|(d, _)| d.size)
        .unwrap_or(0);
    tracing::warn!(
        job_id = id,
        doc_type = %job.doc_type,
        est_bytes,
        "ocr process blocked: no OCR engine configured, job left queued"
    );
    Err(AppError::BlockedExternal(format!(
        "ocr engine unavailable (job {id}, type {}, ~{est_bytes} bytes not processed); native libtesseract missing, cloud fallback unconfigured",
        job.doc_type
    )))
}
async fn ocr_supported(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    Ok(Json(json!(DOC_TYPES)))
}

#[derive(Debug, Deserialize, Validate)]
struct TplReq {
    #[validate(length(min = 2))]
    name: String,
    #[validate(length(min = 1))]
    body: String,
}
async fn tpl_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<TplReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let t: TemplateRow = s
        .stores
        .ops
        .tpl_create(&u.claims.org, &b.name, &b.body)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": t.id, "version": t.version})))
}
async fn tpl_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .tpl_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|t| json!({"id": t.id, "name": t.name, "version": t.version}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct TplPut {
    #[validate(length(min = 1))]
    body: String,
}
async fn tpl_update(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<TplPut>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let t = s
        .stores
        .ops
        .tpl_update(&u.claims.org, id, &b.body)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": t.id, "version": t.version})))
}
fn substitute(body: &str, vars: &HashMap<String, String>) -> String {
    let mut out = body.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}
#[derive(Debug, Deserialize, Validate)]
struct DeedFromTpl {
    template_id: i64,
    matter_id: i64,
    #[validate(length(min = 2))]
    number: String,
    deed_type: String,
    variables: HashMap<String, String>,
}
async fn deed_from_template(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DeedFromTpl>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let t = s
        .stores
        .ops
        .tpl_get(&u.claims.org, b.template_id)
        .await
        .map_err(map_store)?;
    s.stores
        .core
        .get_matter(&u.claims.org, b.matter_id)
        .await
        .map_err(|_| AppError::NotFound("matter".into()))?;
    let rendered = substitute(&t.body, &b.variables);
    let d: DeedRow = s
        .stores
        .ops
        .deed_create(
            &u.claims.org,
            b.matter_id,
            &b.number,
            &b.deed_type,
            &rendered,
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": d.id, "state": d.state})))
}
async fn deed_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .deed_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|d| json!({"id": d.id, "number": d.number, "state": d.state}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct DeedTransition {
    to: String,
}
async fn deed_transition(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DeedTransition>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Approve)?;
    let cur = s
        .stores
        .ops
        .deed_list(&u.claims.org)
        .await
        .map_err(map_store)?
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| AppError::NotFound("deed".into()))?;
    let from = DeedState::parse(&cur.state)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("bad state")))?;
    let to = DeedState::parse(&b.to).ok_or_else(|| AppError::Validation("to".into()))?;
    transition(from, to).map_err(|_| AppError::Conflict("illegal transition".into()))?;
    let d = s
        .stores
        .ops
        .deed_set_state(&u.claims.org, id, b.to.as_str())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": d.id, "state": d.state})))
}
async fn deed_get(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let d = s
        .stores
        .ops
        .deed_list(&u.claims.org)
        .await
        .map_err(map_store)?
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| AppError::NotFound("deed".into()))?;
    Ok(Json(
        json!({"id": d.id, "matter_id": d.matter_id, "number": d.number, "deed_type": d.deed_type, "state": d.state}),
    ))
}
async fn deed_esign(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Approve)?;
    let _ = id;
    Err(AppError::BlockedExternal(
        "PSrE provider not configured".into(),
    ))
}

#[derive(Debug, Deserialize, Serialize, Validate)]
struct ChatMsg {
    role: String,
    content: String,
}
#[derive(Debug, Deserialize, Validate)]
struct ChatReq {
    #[validate(length(min = 1, max = 200))]
    system: Option<String>,
    #[validate(length(min = 1))]
    messages: Vec<ChatMsg>,
}
async fn ai_chat(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ChatReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    if b.messages.is_empty() || b.messages.iter().any(|m| m.content.trim().is_empty()) {
        return Err(AppError::Validation("messages".into()));
    }
    let chars: usize = b.messages.iter().map(|m| m.content.len()).sum::<usize>()
        + b.system.as_deref().map_or(0, str::len);
    let est = (chars / 4).max(1) as i64;
    let conv = s
        .stores
        .ops
        .conv_create(&u.claims.org, "chat")
        .await
        .map_err(map_store)?;
    for m in &b.messages {
        s.stores
            .ops
            .msg_add(conv, &m.role, &m.content)
            .await
            .map_err(map_store)?;
    }
    s.stores
        .ops
        .usage_add(&u.claims.org, est, 0)
        .await
        .map_err(map_store)?;
    Err(AppError::BlockedExternal(format!(
        "no AI provider configured; estimated input tokens: {est} (logged, zero output)"
    )))
}
async fn ai_usage(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (i, o) = s
        .stores
        .ops
        .usage_sum(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"input_tokens": i, "output_tokens": o})))
}

#[derive(Debug, Deserialize)]
struct RagIndexReq {
    document_id: i64,
}
async fn rag_index(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<RagIndexReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let (d, vs) = s
        .stores
        .core
        .get_doc(&u.claims.org, b.document_id)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    if !d.mime.starts_with("text/") {
        return Err(AppError::Validation("binary parsing not supported".into()));
    }
    let v = vs
        .last()
        .ok_or_else(|| AppError::Validation("no content".into()))?;
    let bytes = s.stores.objects.get(&v.object_key).await?;
    let text = String::from_utf8(bytes).map_err(|_| AppError::Validation("not utf8".into()))?;
    let chunks = legalrust_rag::chunk_text(&text, 800, 100);
    s.stores
        .ops
        .rag_put(b.document_id, &chunks)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"chunks": chunks.len()})))
}
#[derive(Debug, Deserialize)]
struct RagSearchQ {
    q: String,
    limit: Option<i64>,
}
async fn rag_search(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<RagSearchQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    if q.q.trim().is_empty() {
        return Err(AppError::Validation("q".into()));
    }
    let hits = s
        .stores
        .ops
        .rag_search(&u.claims.org, &q.q, q.limit.unwrap_or(10).clamp(1, 50))
        .await
        .map_err(map_store)?;
    Ok(Json(json!(hits
        .iter()
        .map(|h| json!({"doc_id": h.doc_id, "idx": h.idx, "text": h.text}))
        .collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct InvReq {
    #[validate(range(min = 1))]
    total_minor: i64,
    currency: Option<String>,
}
async fn inv_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(mid): Path<i64>,
    Json(b): Json<InvReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .core
        .get_matter(&u.claims.org, mid)
        .await
        .map_err(|_| AppError::NotFound("matter".into()))?;
    let number = format!(
        "INV-{}",
        uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
    );
    let v: InvoiceRow = s
        .stores
        .ops
        .inv_create(
            &u.claims.org,
            mid,
            &number,
            b.total_minor,
            b.currency.as_deref().unwrap_or("IDR"),
        )
        .await
        .map_err(map_store)?;
    notify_user(
        &s,
        &u.claims.org,
        &u.claims.sub,
        "invoice",
        &format!("Invoice issued: {}", v.number),
    )
    .await;
    Ok(Json(json!({"id": v.id, "number": v.number})))
}
async fn inv_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(mid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .inv_list(&u.claims.org, mid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|i| json!({"id": i.id, "number": i.number, "total": i.total}))
        .collect::<Vec<_>>())))
}
async fn inv_get(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (v, ps) = s
        .stores
        .ops
        .inv_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    let paid: i64 = ps.iter().map(|p| p.amount).sum();
    Ok(Json(
        json!({"id": v.id, "total": v.total, "paid": paid, "balance": v.total - paid, "payments": ps.iter().map(|p| json!({"id": p.id, "amount": p.amount})).collect::<Vec<_>>()}),
    ))
}
#[derive(Debug, Deserialize, Validate)]
struct PayReq {
    #[validate(range(min = 1))]
    amount_minor: i64,
    method: String,
}
async fn pay_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<PayReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let p = s
        .stores
        .ops
        .pay_add(&u.claims.org, id, b.amount_minor, &b.method)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": p.id, "amount": p.amount})))
}
async fn pay_gateway(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let _ = id;
    Err(AppError::BlockedExternal(
        "billing gateway not configured".into(),
    ))
}
fn csv_field(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}
async fn matters_csv(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .core
        .list_matters(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let mut csv = String::from("id,number,title,status\n");
    for m in &v {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            m.id,
            csv_field(&m.number),
            csv_field(&m.title),
            csv_field(&m.status)
        ));
    }
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    Ok((StatusCode::OK, h, csv))
}

async fn webhook_in(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    if s.stores.integration_secret.is_empty() {
        return Err(AppError::BlockedExternal(
            "integration secret not configured".into(),
        ));
    }
    let sig = headers
        .get("x-signature")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;
    let ts = headers
        .get("x-ts")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;
    legalrust_integrations::verify_webhook(&s.stores.integration_secret, &body, ts, sig)
        .map_err(|_| AppError::Unauthorized)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: "ext".into(),
            actor_id: None,
            action: format!("webhook.{provider}.received"),
            entity: "webhook".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"accepted": true, "provider": provider})))
}
async fn integrations(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let configured = !s.stores.integration_secret.is_empty();
    Ok(Json(json!(s
        .stores
        .integrations
        .iter()
        .map(|n| json!({"name": n, "configured": configured}))
        .collect::<Vec<_>>())))
}
// ---------- end Phase 7-12 handlers ----------

// ---------- Phase 11/12 final handlers ----------
async fn notify_user(s: &FullState, org: &str, user: &str, kind: &str, title: &str) {
    let _ = s.stores.ops.notify(org, user, kind, title, None).await;
    let _ = s.stores.events.send(format!("notify:{kind}"));
}

#[derive(Debug, Deserialize, Validate)]
struct TokenReq {
    #[validate(length(min = 2))]
    name: String,
}
async fn token_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<TokenReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let raw = format!("lr_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let hash = legalrust_document::sha256_hex(raw.as_bytes());
    let id = s
        .stores
        .users
        .create_token(&u.claims.sub, &b.name, &hash)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "token": raw})))
}
async fn token_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let v = s
        .stores
        .users
        .list_tokens(&u.claims.sub)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|(id, n)| json!({"id": id, "name": n}))
        .collect::<Vec<_>>())))
}
async fn token_revoke(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    s.stores
        .users
        .revoke_token(&u.claims.sub, &id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"revoked": true})))
}

#[derive(Debug, Deserialize, Validate)]
struct PasswordReq {
    #[validate(length(min = 8))]
    current_password: String,
    #[validate(length(min = 8))]
    new_password: String,
}
async fn password_change(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PasswordReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    if u.claims.sid == "api-token" {
        return Err(AppError::Forbidden);
    }
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let row = s
        .stores
        .users
        .get_user(&u.claims.sub)
        .await
        .map_err(map_store)?
        .ok_or(AppError::Unauthorized)?;
    legalrust_auth::verify_password(&row.pw_hash, &b.current_password)
        .map_err(|_| AppError::Unauthorized)?;
    let hash = legalrust_auth::hash_password(&b.new_password)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("hash")))?;
    s.stores
        .users
        .set_password(&u.claims.sub, &hash)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"changed": true})))
}

#[derive(Debug, Deserialize)]
struct RoleReq {
    role: String,
}
async fn user_assign_role(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(uid): Path<String>,
    Json(b): Json<RoleReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Assign)?;
    let role = legalrust_domain::Role::parse(&b.role)
        .ok_or_else(|| AppError::Validation("role".into()))?;
    s.stores
        .users
        .get_user(&uid)
        .await
        .map_err(map_store)?
        .ok_or_else(|| AppError::NotFound("user".into()))?;
    s.stores
        .users
        .assign_role(&u.claims.org, &uid, role.as_str())
        .await
        .map_err(map_store)?;
    let roles = s.stores.users.user_roles(&uid).await.map_err(map_store)?;
    Ok(Json(json!({"user_id": uid, "roles": roles})))
}

async fn notify_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let v = s
        .stores
        .ops
        .notify_list(&u.claims.sub)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|n| json!({"id": n.id, "kind": n.kind, "title": n.title, "read": n.read}))
        .collect::<Vec<_>>())))
}
async fn notify_read(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    s.stores
        .ops
        .notify_read(&u.claims.sub, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"read": true})))
}

#[derive(Debug, Deserialize)]
struct ShareReq {
    hours: Option<i64>,
}
async fn share_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<ShareReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Share)?;
    s.stores
        .core
        .get_doc(&u.claims.org, id)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let hours = b.hours.unwrap_or(72).clamp(1, 720);
    let token = uuid::Uuid::new_v4().to_string();
    s.stores
        .ops
        .share_create(&u.claims.org, id, &token, hours)
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"token": token, "url": format!("/api/v1/shared/{token}")}),
    ))
}
async fn share_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .share_list(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|x| json!({"id": x.id, "token": x.token, "expires": x.expires, "revoked": x.revoked}))
        .collect::<Vec<_>>())))
}
async fn share_revoke(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Share)?;
    s.stores
        .ops
        .share_revoke(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"revoked": true})))
}
fn share_valid(expires: &str) -> bool {
    for fmt in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
    ] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(expires, fmt) {
            return dt > chrono::Utc::now().naive_utc();
        }
    }
    if let Ok(epoch) = expires.parse::<i64>() {
        return epoch > chrono::Utc::now().timestamp();
    }
    false
}
async fn shared_get(
    State(s): State<FullState>,
    Path(token): Path<String>,
) -> Result<Json<Value>, AppError> {
    let sh = s
        .stores
        .ops
        .share_get(&token)
        .await
        .map_err(map_store)?
        .ok_or_else(|| AppError::NotFound("share".into()))?;
    if sh.revoked || !share_valid(&sh.expires) {
        return Err(AppError::NotFound("share".into()));
    }
    let (d, vs) = s
        .stores
        .core
        .get_doc(&sh.org, sh.doc)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    Ok(Json(
        json!({"name": d.name, "mime": d.mime, "size": d.size, "versions": vs.len()}),
    ))
}
async fn shared_content(
    State(s): State<FullState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let sh = s
        .stores
        .ops
        .share_get(&token)
        .await
        .map_err(map_store)?
        .ok_or_else(|| AppError::NotFound("share".into()))?;
    if sh.revoked || !share_valid(&sh.expires) {
        return Err(AppError::NotFound("share".into()));
    }
    let (d, vs) = s
        .stores
        .core
        .get_doc(&sh.org, sh.doc)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let v = vs
        .last()
        .ok_or_else(|| AppError::NotFound("content".into()))?;
    let bytes = s.stores.objects.get(&v.object_key).await?;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        d.mime
            .parse()
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    Ok((StatusCode::OK, h, bytes))
}

async fn report_matters(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let v = s
        .stores
        .core
        .list_matters(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut rows = vec![vec![
        "id".into(),
        "number".into(),
        "title".into(),
        "status".into(),
    ]];
    for m in v {
        rows.push(vec![m.id.to_string(), m.number, m.title, m.status]);
    }
    Ok((
        [("content-type", "text/csv")],
        legalrust_reporting::to_csv(&rows),
    ))
}

#[derive(Debug, Deserialize)]
struct AuditQ {
    entity: Option<String>,
    limit: Option<i64>,
}
async fn audit_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<AuditQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let allowed = u
        .claims
        .roles
        .iter()
        .filter_map(|r| legalrust_domain::Role::parse(r))
        .any(|r| {
            matches!(
                r,
                legalrust_domain::Role::SuperAdmin
                    | legalrust_domain::Role::OrgOwner
                    | legalrust_domain::Role::OfficeAdmin
                    | legalrust_domain::Role::Auditor
            )
        });
    if !allowed {
        return Err(AppError::Forbidden);
    }
    let v = s
        .stores
        .ops
        .audit_list(
            &u.claims.org,
            q.entity.as_deref(),
            q.limit.unwrap_or(100).clamp(1, 500),
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|a| json!({"id": a.id, "actor": a.actor, "action": a.action, "entity": a.entity, "entity_id": a.entity_id, "created": a.created})).collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize)]
struct JobReq {
    kind: String,
    payload: Option<String>,
}
async fn job_enqueue(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<JobReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    if b.kind.trim().is_empty() {
        return Err(AppError::Validation("kind".into()));
    }
    let id = s
        .stores
        .ops
        .job_enqueue(&u.claims.org, &b.kind, b.payload.as_deref().unwrap_or("{}"))
        .await
        .map_err(map_store)?;
    let _ = s.stores.events.send(format!("job.enqueued:{id}"));
    Ok(Json(json!({"id": id, "status": "queued"})))
}
async fn job_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .job_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|j| json!({"id": j.id, "kind": j.kind, "status": j.status, "attempts": j.attempts}))
        .collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct TrustAcctReq {
    client_id: i64,
    currency: Option<String>,
}
async fn trust_acct_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<TrustAcctReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let a = s
        .stores
        .ops
        .trust_create_account(
            &u.claims.org,
            b.client_id,
            b.currency.as_deref().unwrap_or("IDR"),
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": a.id, "balance": a.balance})))
}
async fn trust_acct_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .trust_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|a| json!({"id": a.id, "client_id": a.client_id, "balance": a.balance}))
        .collect::<Vec<_>>())))
}
async fn trust_acct_get(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (a, es) = s
        .stores
        .ops
        .trust_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"id": a.id, "balance": a.balance, "entries": es.iter().map(|e| json!({"id": e.id, "kind": e.kind, "amount": e.amount})).collect::<Vec<_>>()}),
    ))
}
#[derive(Debug, Deserialize, Validate)]
struct TrustEntryReq {
    matter_id: Option<i64>,
    kind: String,
    #[validate(range(min = 1))]
    amount_minor: i64,
}
async fn trust_entry_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<TrustEntryReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if !["debit", "credit"].contains(&b.kind.as_str()) {
        return Err(AppError::Validation("kind".into()));
    }
    let e = s
        .stores
        .ops
        .trust_post(
            &u.claims.org,
            id,
            b.matter_id,
            &b.kind,
            b.amount_minor,
            "manual",
            id,
            &u.claims.sub,
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": e.id, "amount": e.amount})))
}
#[derive(Debug, Deserialize, Validate)]
struct TrustTransferReq {
    from: i64,
    to: i64,
    matter_id: Option<i64>,
    #[validate(range(min = 1))]
    amount_minor: i64,
}
async fn trust_transfer(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<TrustTransferReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let (a, x) = s
        .stores
        .ops
        .trust_transfer(
            &u.claims.org,
            b.from,
            b.to,
            b.amount_minor,
            b.matter_id,
            "transfer",
            b.from,
            &u.claims.sub,
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"out": a.id, "in": x.id})))
}

async fn sse_events(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<
    axum::response::Sse<
        impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
    >,
    AppError,
> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio_stream::{wrappers::BroadcastStream, StreamExt};
    let _u = auth_user(&headers, &s).await?;
    let rx = s.stores.events.subscribe();
    let live = BroadcastStream::new(rx)
        .filter_map(|r| r.ok())
        .map(|msg| Ok(Event::default().data(msg)));
    let seeded = tokio_stream::iter(vec![Ok(Event::default().data("hello"))]).chain(live);
    Ok(Sse::new(seeded).keep_alive(KeepAlive::default()))
}
// ---------- end Phase 11/12 final handlers ----------

async fn not_found() -> AppError {
    AppError::NotFound("route".into())
}

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
        .route(
            "/api/v1/matters/{mid}/tasks",
            post(create_task).get(list_tasks),
        )
        .route("/api/v1/tasks/{tid}", post(patch_task))
        .route("/api/v1/documents", post(create_doc).get(list_docs))
        .route("/api/v1/documents/{id}", get(get_doc))
        .route(
            "/api/v1/documents/{id}/content",
            put(put_content).get(get_content),
        )
        .route("/api/v1/ocr/jobs", post(ocr_create).get(ocr_list))
        .route("/api/v1/ocr/jobs/{id}", get(ocr_get))
        .route("/api/v1/ocr/jobs/{id}/review", post(ocr_review))
        .route("/api/v1/ocr/jobs/{id}/process", post(ocr_process))
        .route("/api/v1/ocr/supported", get(ocr_supported))
        .route("/api/v1/deed-templates", post(tpl_create).get(tpl_list))
        .route("/api/v1/deed-templates/{id}", put(tpl_update))
        .route("/api/v1/deeds/from-template", post(deed_from_template))
        .route("/api/v1/deeds", get(deed_list))
        .route("/api/v1/deeds/{id}", get(deed_get))
        .route("/api/v1/deeds/{id}/esign", post(deed_esign))
        .route("/api/v1/deeds/{id}/transition", post(deed_transition))
        .route("/api/v1/ai/chat", post(ai_chat))
        .route("/api/v1/ai/usage", get(ai_usage))
        .route("/api/v1/rag/index", post(rag_index))
        .route("/api/v1/rag/search", get(rag_search))
        .route(
            "/api/v1/matters/{mid}/invoices",
            post(inv_create).get(inv_list),
        )
        .route("/api/v1/invoices/{id}", get(inv_get))
        .route("/api/v1/invoices/{id}/payments", post(pay_add))
        .route("/api/v1/invoices/{id}/pay-gateway", post(pay_gateway))
        .route("/api/v1/reports/matters-csv", get(matters_csv))
        .route("/api/v1/webhooks/incoming/{provider}", post(webhook_in))
        .route("/api/v1/integrations", get(integrations))
        .route("/api/v1/auth/tokens", post(token_create).get(token_list))
        .route("/api/v1/auth/tokens/{id}", delete(token_revoke))
        .route("/api/v1/auth/password", post(password_change))
        .route("/api/v1/users/{uid}/roles", post(user_assign_role))
        .route("/api/v1/notifications", get(notify_list))
        .route("/api/v1/notifications/{id}/read", post(notify_read))
        .route(
            "/api/v1/documents/{id}/shares",
            post(share_create).get(share_list),
        )
        .route("/api/v1/shares/{id}", delete(share_revoke))
        .route("/api/v1/shared/{token}", get(shared_get))
        .route("/api/v1/shared/{token}/content", get(shared_content))
        .route("/api/v1/reports/matters.csv", get(report_matters))
        .route("/api/v1/audit", get(audit_list))
        .route("/api/v1/jobs", post(job_enqueue).get(job_list))
        .route(
            "/api/v1/trust/accounts",
            post(trust_acct_create).get(trust_acct_list),
        )
        .route("/api/v1/trust/accounts/{id}", get(trust_acct_get))
        .route("/api/v1/trust/accounts/{id}/entries", post(trust_entry_add))
        .route("/api/v1/trust/transfer", post(trust_transfer))
        .route("/api/v1/events", get(sse_events))
        .fallback(not_found)
        .layer(RequestBodyLimitLayer::new(
            (MAX_UPLOAD + 1024 * 1024) as usize,
        ))
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'self'"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        ))
        .with_state(state)
}

pub fn dev_state() -> FullState {
    let m = std::sync::Arc::new(legalrust_database::memory::MemoryStores::default());
    let o = std::sync::Arc::new(legalrust_database::ops_memory::MemoryOps::default());
    let mut stores = Stores::new(
        m.clone(),
        m,
        o,
        Arc::new(LocalStore::new(std::env::temp_dir().join("lr-dev-objects"))),
        b"test-secret".to_vec(),
    );
    stores.integration_secret = b"test-integration".to_vec();
    FullState {
        version: "0.1.0",
        pool: None,
        stores,
    }
}
