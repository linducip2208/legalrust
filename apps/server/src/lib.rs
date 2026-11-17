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
    #[validate(length(min = 12))]
    password: String,
}
#[derive(Debug, Deserialize, Validate)]
struct LoginReq {
    #[validate(email)]
    email: String,
    #[validate(length(min = 12))]
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
    #[validate(length(min = 12))]
    current_password: String,
    #[validate(length(min = 12))]
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

async fn static_img(Path(name): Path<String>) -> Result<impl IntoResponse, AppError> {
    const ALLOW: &[(&str, &str)] = &[
        ("hero-office.jpg", "image/jpeg"),
        ("hero-office.png", "image/png"),
        ("hero-office.webp", "image/webp"),
        ("cta-building.jpg", "image/jpeg"),
        ("cta-building.png", "image/png"),
        ("cta-building.webp", "image/webp"),
    ];
    let (file, ctype) = ALLOW
        .iter()
        .find(|(f, _)| *f == name)
        .ok_or_else(|| AppError::NotFound("asset".into()))?;
    let path = std::path::PathBuf::from("apps/server/static").join(file);
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| AppError::NotFound("asset".into()))?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, *ctype)],
        bytes,
    ))
}
async fn not_found() -> AppError {
    AppError::NotFound("route".into())
}

// ---------- public pages (landing + login + app shell) ----------
const PAGE_CSS: &str = ":root{--paper:#FAF7F1;--ink:#1C1917;--muted:#78716C;--gold:#B45309;--emerald:#0C4A3B;--line:rgba(28,25,23,.1);--card:#FFFFFF}\
*{box-sizing:border-box}html{scroll-behavior:smooth}\
body{font-family:'Plus Jakarta Sans',system-ui,sans-serif;margin:0;background:var(--paper);color:var(--ink);-webkit-font-smoothing:antialiased}\
.serif{font-family:'Fraunces',Georgia,serif}h1,h2{letter-spacing:-.02em}\
a{color:var(--emerald)}.wrap{max-width:1120px;margin:0 auto;padding:0 24px}\
.island{position:sticky;top:20px;z-index:20;display:flex;justify-content:space-between;align-items:center;max-width:1120px;margin:20px auto 0;padding:12px 12px 12px 24px;background:rgba(250,247,241,.85);backdrop-filter:blur(16px);border:1px solid var(--line);border-radius:999px}\
.island nav a{margin-left:6px;text-decoration:none;font-size:14px;font-weight:600;color:var(--ink);padding:8px 14px;border-radius:999px;transition:all .5s cubic-bezier(.32,.72,0,1)}\
.island nav a:hover{background:rgba(28,25,23,.06)}\
.brand{font-weight:800;letter-spacing:.14em;font-size:13px;text-decoration:none;color:var(--ink)}\
.pill{display:inline-flex;align-items:center;gap:12px;background:var(--ink);color:#fff;padding:8px 8px 8px 24px;border-radius:999px;text-decoration:none;font-weight:700;font-size:15px;transition:transform .5s cubic-bezier(.32,.72,0,1)}\
.pill:hover{transform:translateY(-2px)}.pill:active{transform:scale(.98)}\
.pill .arr{width:36px;height:36px;border-radius:50%;background:rgba(255,255,255,.14);display:flex;align-items:center;justify-content:center;transition:transform .5s cubic-bezier(.32,.72,0,1)}\
.pill:hover .arr{transform:translate(2px,-2px) scale(1.06)}\
.pill.ghost{background:transparent;color:var(--ink);border:1px solid var(--line)}\
.pill.ghost .arr{background:rgba(28,25,23,.07)}\
.eyebrow{display:inline-block;font-size:11px;font-weight:700;letter-spacing:.22em;text-transform:uppercase;color:var(--gold);border:1px solid var(--line);border-radius:999px;padding:6px 14px;background:#fff}\
.hero{padding:96px 0 72px}.hero h1{font-size:clamp(44px,7vw,88px);line-height:1.02;margin:24px 0}\
.hero p.lead{font-size:19px;color:var(--muted);max-width:620px;line-height:1.7}\
.hero-cta{display:flex;gap:14px;margin-top:32px;flex-wrap:wrap}\
.bento{display:grid;grid-template-columns:repeat(12,1fr);gap:20px;padding:24px 0 96px}\
.shell{background:rgba(28,25,23,.045);border:1px solid var(--line);border-radius:28px;padding:10px;transition:transform .7s cubic-bezier(.32,.72,0,1),opacity .7s}\
.shell:hover{transform:translateY(-4px)}\
.core{background:var(--card);border-radius:20px;padding:28px;box-shadow:inset 0 1px 1px rgba(255,255,255,.9),0 24px 48px -24px rgba(28,25,23,.18);height:100%}\
.core .n{font-size:12px;font-weight:800;letter-spacing:.18em;color:var(--gold)}\
.core h3{font-size:22px;margin:10px 0 8px}.core p{color:var(--muted);font-size:15px;line-height:1.65;margin:0}\
.s4{grid-column:span 4}.s6{grid-column:span 6}.s8{grid-column:span 8}.s12{grid-column:span 12}\
.strip{display:flex;gap:40px;flex-wrap:wrap;padding:28px 0;border-top:1px solid var(--line);border-bottom:1px solid var(--line);margin-bottom:8px}\
.strip b{font-size:30px;display:block}.strip span{color:var(--muted);font-size:13px}\
footer{padding:48px 0 64px;color:var(--muted);font-size:13px}\
.authmin{min-height:100dvh;display:flex;align-items:center;justify-content:center;padding:32px 16px}\
.authcard{width:100%;max-width:440px}\
.authcard .core h2{font-size:30px;margin:12px 0 4px}\
.tabs{display:flex;gap:8px;margin:18px 0}.tabs button{flex:1;padding:10px;border-radius:999px;border:1px solid var(--line);background:transparent;font-weight:700;cursor:pointer;transition:all .5s cubic-bezier(.32,.72,0,1)}\
.tabs button.on{background:var(--ink);color:#fff}\
label.fl{display:block;font-size:13px;font-weight:700;margin:12px 0 2px}\
input.fi{width:100%;padding:13px 16px;margin:4px 0 6px;border-radius:14px;border:1px solid var(--line);background:var(--paper);font-size:15px;font-family:inherit}\
input.fi:focus{outline:2px solid var(--emerald);outline-offset:1px}\
.wfull{width:100%;justify-content:center}\
.err{color:#B91C1C;font-size:14px;min-height:20px}\
.dashhead{display:flex;justify-content:space-between;align-items:center;padding:36px 0 8px;flex-wrap:wrap;gap:12px}\
table.tbl{width:100%;border-collapse:collapse;font-size:15px}\
table.tbl th{text-align:left;font-size:12px;letter-spacing:.12em;text-transform:uppercase;color:var(--muted);padding:10px 12px;border-bottom:1px solid var(--line)}\
table.tbl td{padding:12px;border-bottom:1px solid var(--line)}\
.badge{display:inline-block;font-size:12px;font-weight:700;padding:4px 12px;border-radius:999px;background:rgba(12,74,59,.1);color:var(--emerald)}\
.rv{opacity:0;transform:translateY(28px);filter:blur(6px);transition:opacity .9s cubic-bezier(.32,.72,0,1),transform .9s cubic-bezier(.32,.72,0,1),filter .9s}\
.rv.in{opacity:1;transform:none;filter:none}\
@media(max-width:768px){.s4,.s6,.s8{grid-column:span 12}.wrap{padding:0 16px}.hero{padding:64px 0 48px}.island{margin:12px;border-radius:24px}}";

const LANDING_HTML: &str = r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>LegalRust â€” Smart Legal Solution</title><link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet"><style>
*{box-sizing:border-box}body{font-family:'Plus Jakarta Sans',system-ui,sans-serif;margin:0;color:#0F1B3D;background:#fff}
a{text-decoration:none}.wrap{max-width:1240px;margin:0 auto;padding:0 28px}
.topbar{display:flex;align-items:center;gap:28px;padding:18px 0}
.logo{display:flex;align-items:center;gap:10px;font-weight:800;font-size:22px;color:#0B2BA8}
.logo small{display:block;font-size:11px;font-weight:600;color:#5A6B8C}
.nav{display:flex;gap:26px;margin-left:24px;font-size:15px;font-weight:600}
.nav a{color:#1E2A4A}.nav a.on{background:#2563EB;color:#fff;padding:8px 20px;border-radius:999px}
.topact{margin-left:auto;display:flex;gap:12px;align-items:center}
.btn{display:inline-flex;align-items:center;gap:10px;font-weight:700;font-size:15px;padding:13px 26px;border-radius:14px;transition:transform .45s cubic-bezier(.32,.72,0,1)}
.btn:active{transform:scale(.97)}.btn-pri{background:#1D5BD8;color:#fff;box-shadow:0 12px 28px -10px rgba(29,91,216,.55)}
.btn-pri:hover{transform:translateY(-2px)}.btn-line{border:1.5px solid #C9D6F2;color:#16224A;background:#fff}
.btn-white{background:#fff;color:#16224A}
.hero{background:linear-gradient(180deg,rgba(234,242,255,.92),rgba(247,250,255,.94)),url('/static/hero-office.png') center/cover;overflow:hidden}
.hero-in{display:grid;grid-template-columns:1fr 1.15fr;gap:36px;align-items:center;padding:56px 0 72px}
.badge{display:inline-flex;align-items:center;gap:8px;background:#fff;border:1px solid #D8E4FA;border-radius:999px;padding:8px 18px;font-size:13.5px;font-weight:600;color:#33415E;box-shadow:0 6px 18px -8px rgba(29,91,216,.35)}
.hero h1{font-size:clamp(38px,4.6vw,62px);line-height:1.06;margin:22px 0 18px;letter-spacing:-.02em;color:#101C3D}
.hero h1 .bl{color:#1D5BD8}.hero p.lead{color:#4A5878;font-size:17px;line-height:1.7;max-width:520px}
.cta-row{display:flex;gap:14px;margin:28px 0 22px;flex-wrap:wrap}
.checks{display:flex;gap:22px;flex-wrap:wrap;font-size:14px;font-weight:600;color:#2A3866}
.checks span{display:flex;align-items:center;gap:8px}.tick{width:22px;height:22px;border-radius:50%;background:#1D5BD8;color:#fff;display:inline-flex;align-items:center;justify-content:center;font-size:13px}
.mock{background:#fff;border-radius:20px;box-shadow:0 40px 80px -32px rgba(15,40,120,.35);border:1px solid #E3EBFA;overflow:hidden}
.mock-bar{display:flex;align-items:center;gap:8px;padding:12px 18px;border-bottom:1px solid #EDF1F8;font-size:12px;color:#8A97B5}
.dot{width:10px;height:10px;border-radius:50%}
.mock-body{display:grid;grid-template-columns:190px 1fr;min-height:420px}
.side{background:#F6F9FF;padding:18px 12px;font-size:13.5px;font-weight:600;color:#3D4C6E}
.side .mi{padding:10px 14px;border-radius:10px;margin-bottom:4px}.side .mi.on{background:#1D5BD8;color:#fff}
.dmain{padding:20px 22px}.dtop{display:flex;justify-content:space-between;align-items:center;margin-bottom:16px}
.pill{background:#EAF1FE;border-radius:8px;padding:7px 14px;font-size:12.5px;font-weight:700;color:#29407A}
.stat4{display:grid;grid-template-columns:repeat(4,1fr);gap:12px;margin-bottom:16px}
.stat{background:#fff;border:1px solid #E7EDF7;border-radius:14px;padding:14px;font-size:12px;color:#66739A}
.stat b{display:block;font-size:22px;color:#101C3D;margin-top:4px}.stat .up{color:#16A34A;font-weight:700}
.chart{background:#fff;border:1px solid #E7EDF7;border-radius:14px;padding:14px;margin-bottom:14px}
.feed{background:#fff;border:1px solid #E7EDF7;border-radius:14px;padding:14px;font-size:12.5px}
.feed div{padding:8px 0;border-bottom:1px solid #F0F3F9;color:#3D4C6E}.feed div:last-child{border:0}
.cap{font-size:11.5px;color:#8A97B5;text-align:center;padding:8px}
.trust{display:flex;align-items:center;gap:26px;background:#fff;border:1px solid #E7EDF7;border-radius:18px;padding:20px 30px;box-shadow:0 18px 40px -24px rgba(15,40,120,.25);font-size:13px;color:#5A6B8C;flex-wrap:wrap}
.trust b{color:#101C3D}.seg{font-weight:800;color:#33415E;letter-spacing:.04em}
.feat{padding:72px 0 20px;text-align:center}.feat h2{font-size:clamp(28px,3.4vw,44px);margin:14px 0 10px;color:#101C3D;letter-spacing:-.02em}
.feat p.sub{color:#5A6B8C;font-size:16px}.fgrid{display:grid;grid-template-columns:repeat(3,1fr);gap:18px;margin-top:36px;text-align:left}
.fcard{background:#F4F7FE;border-radius:18px;padding:28px;transition:transform .5s cubic-bezier(.32,.72,0,1)}
.fcard:hover{transform:translateY(-5px)}.fcard .ic{font-size:26px}.fcard h3{font-size:17px;margin:12px 0 8px;color:#101C3D}.fcard p{font-size:14px;color:#5A6B8C;line-height:1.65;margin:0}
.cta{background:linear-gradient(100deg,rgba(11,43,168,.94),rgba(29,91,216,.88)),url('/static/cta-building.png') center/cover;border-radius:26px;color:#fff;padding:56px;display:grid;grid-template-columns:1.3fr 1fr;gap:24px;align-items:center;margin:64px 0}
.cta h2{font-size:clamp(24px,3vw,38px);margin:0 0 10px}.cta p{opacity:.85}.cta ul{list-style:none;margin:0;padding:0;font-size:15px;font-weight:600}.cta li{margin:10px 0}
footer{padding:28px 0 48px;color:#8A97B5;font-size:13px;display:flex;justify-content:space-between;flex-wrap:wrap;gap:12px}
.rv{opacity:0;transform:translateY(26px);transition:opacity .8s cubic-bezier(.32,.72,0,1),transform .8s cubic-bezier(.32,.72,0,1)}
.rv.in{opacity:1;transform:none}
@media(max-width:960px){.hero-in{grid-template-columns:1fr}.nav{display:none}.stat4{grid-template-columns:repeat(2,1fr)}.fgrid{grid-template-columns:1fr}.cta{grid-template-columns:1fr;padding:36px}.mock-body{grid-template-columns:1fr}.side{display:none}}
</style></head><body>
<div class="wrap"><div class="topbar"><div class="logo">âš–ï¸<span>LegalRust<small>Smart Legal Solution</small></span></div><nav class="nav"><a class="on" href="/">Beranda</a><a href="#fitur">Fitur</a><a href="/app">Solusi</a><a href="#fitur">Harga</a><a href="#fitur">Tentang</a><a href="/login">Kontak</a></nav><div class="topact"><a class="btn btn-line" href="/login" style="padding:10px 22px">Masuk</a><a class="btn btn-pri" href="/login" style="padding:10px 22px">Coba Gratis â†’</a></div></div></div>
<section class="hero"><div class="wrap hero-in"><div>
<span class="badge rv">ðŸš€ Solusi LegalTech Terdepan di Indonesia</span>
<h1 class="rv">Kelola Perkara Hukum Lebih Mudah, <span class="bl">Profesional</span> <span class="bl">dan Terpercaya</span></h1>
<p class="lead rv">LegalRust adalah platform manajemen hukum modern untuk notaris, firma hukum dan profesional hukum dengan teknologi Rust yang super cepat dan aman.</p>
<div class="cta-row rv"><a class="btn btn-pri" href="/app">â–¶&nbsp; Lihat Demo â†’</a><a class="btn btn-line" href="/login">Mulai Gratis Sekarang â†’</a></div>
<div class="checks rv"><span><i class="tick">âœ“</i>Gratis Selamanya</span><span><i class="tick">âœ“</i>Aman &amp; Terpercaya</span><span><i class="tick">âœ“</i>Support 24/7</span></div>
</div>
<div class="mock rv"><div class="mock-bar"><span class="dot" style="background:#FF5F57"></span><span class="dot" style="background:#FEBC2E"></span><span class="dot" style="background:#28C840"></span><span style="margin-left:8px">LegalRust â€” pratinjau dasbor (ilustrasi)</span></div>
<div class="mock-body"><div class="side"><div class="mi on">ðŸ  Dashboard</div><div class="mi">ðŸ“ Perkara</div><div class="mi">ðŸ‘¥ Klien</div><div class="mi">ðŸ“„ Dokumen</div><div class="mi">ðŸ” OCR &amp; Verifikasi</div><div class="mi">âœ’ï¸ Akta &amp; Kontrak</div><div class="mi">ðŸ’° Keuangan</div><div class="mi">ðŸ“Š Laporan</div><div class="mi">âš™ï¸ Pengaturan</div></div>
<div class="dmain"><div class="dtop"><b>Dashboard</b><span class="pill">30 Hari Terakhir â–¾</span></div>
<div class="stat4"><div class="stat">Total Perkara<b>128</b><span class="up">â†‘ 12.5%</span></div><div class="stat">Dokumen Diproses<b>842</b><span class="up">â†‘ 18.7%</span></div><div class="stat">Klien Aktif<b>256</b><span class="up">â†‘ 8.2%</span></div><div class="stat">Pendapatan<b>Rp 248 jt</b><span class="up">â†‘ 14.6%</span></div></div>
<div class="chart"><b style="font-size:13px">Tren Perkara</b><svg viewBox="0 0 400 110" width="100%" height="90"><polygon points="0,90 40,82 80,84 120,70 160,72 200,58 240,60 280,44 320,46 360,28 400,24 400,110 0,110" fill="rgba(37,99,235,.12)"/><polyline points="0,90 40,82 80,84 120,70 160,72 200,58 240,60 280,44 320,46 360,28 400,24" fill="none" stroke="#2563EB" stroke-width="3"/></svg></div>
<div class="feed"><div>âœ… Dokumen Akta Notaris disetujui â€” 2 menit lalu</div><div>ðŸ“ Perkara #KR-2026-1047 diperbarui â€” 5 menit lalu</div><div>ðŸ‘¤ Klien baru: PT Maju Bersama â€” 10 menit lalu</div></div>
</div></div><div class="cap">Angka di atas ilustrasi tampilan â€” data live Anda tampil setelah masuk.</div></div>
</div></section>
<div class="wrap"><div class="trust rv"><div><b>Dipercaya oleh</b><br>Ribuan Profesional</div><span class="seg">NOTARIS</span><span class="seg">PPAT</span><span class="seg">FIRMA HUKUM</span><span class="seg">KORPORAT</span><span class="seg">ADVOKAT</span></div>
<section class="feat" id="fitur"><span class="badge rv">Fitur Unggulan</span><h2 class="rv">Semua yang Anda Butuhkan dalam Satu Platform</h2><p class="sub rv">Dari manajemen perkara hingga laporan analitik, semua terintegrasi dengan sempurna.</p>
<div class="fgrid">
<div class="fcard rv"><div class="ic">âš¡</div><h3>Manajemen Perkara Terintegrasi</h3><p>Kelola seluruh perkara â€” klien, tugas, jadwal â€” dalam satu alur yang tertib dan teraudit.</p></div>
<div class="fcard rv"><div class="ic">ðŸ“„</div><h3>OCR &amp; Verifikasi Dokumen</h3><p>Scan dan verifikasi KTP, KK, NPWP, dan sertifikat dengan skor kepercayaan + review manusia.</p></div>
<div class="fcard rv"><div class="ic">âœ’ï¸</div><h3>Pengelolaan Akta</h3><p>Buat akta dan kontrak dari template profesional dengan alur persetujuan ber-guard.</p></div>
<div class="fcard rv"><div class="ic">ðŸ’°</div><h3>Keuangan &amp; Trust Ledger</h3><p>Invoice anti-overpay dan rekening dana klien yang saldonya tak pernah negatif.</p></div>
<div class="fcard rv"><div class="ic">ðŸ“Š</div><h3>Laporan &amp; Analitik</h3><p>Pantau kinerja perkara dan keuangan lewat ekspor CSV siap olah.</p></div>
<div class="fcard rv"><div class="ic">ðŸ›¡ï¸</div><h3>Keamanan Tingkat Tinggi</h3><p>Argon2, JWT + lockout, RBAC per endpoint, dan jejak audit di setiap langkah.</p></div>
</div></section>
<section class="cta rv"><div><h2>Siap Meningkatkan Produktivitas Kantor Hukum Anda?</h2><p>Bergabunglah dengan ribuan profesional yang telah mempercayai LegalRust.</p><br><a class="btn btn-white" href="/login">Mulai Sekarang â†’</a></div><ul><li>âœ“ Gratis selamanya</li><li>âœ“ Setup mudah dalam 1 menit</li><li>âœ“ Support profesional</li></ul></section>
<footer><span>Â© 2026 LegalRust â€” Smart Legal Solution</span><span><a href="/healthz">Status</a> Â· <a href="/api/v1/version">API</a> Â· <a href="/login">Masuk</a></span></footer></div>
<script>const io=new IntersectionObserver(e=>e.forEach(x=>x.isIntersecting&&x.target.classList.add('in')),{threshold:.1});document.querySelectorAll('.rv').forEach(el=>io.observe(el));</script>
</body></html>"##;

const LOGIN_HTML: &str = r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Masuk â€” LEGALRUST</title><link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,500;9..144,700&family=Plus+Jakarta+Sans:wght@400;600;700;800&display=swap" rel="stylesheet"><style>CSS</style></head><body>
<div class="island"><a class="brand" href="/">LEGALRUST</a><nav><a href="/">Beranda</a></nav></div>
<div class="authmin"><div class="authcard shell"><div class="core">
<span class="eyebrow">Akses konsultan</span><h2 class="serif">Masuk</h2>
<p style="color:var(--muted);font-size:14px">Sesi JWT + proteksi brute-force 5x percobaan.</p>
<div class="tabs"><button id="t1" class="on" onclick="tab(1)">Masuk</button><button id="t2" onclick="tab(2)">Daftar organisasi</button></div>
<div id="f_reg" style="display:none"><label class="fl">Nama organisasi</label><input class="fi" id="org"><label class="fl">Nama Anda</label><input class="fi" id="nm"></div>
<label class="fl">Email</label><input class="fi" id="email" type="email" autocomplete="username">
<label class="fl">Kata sandi</label><input class="fi" id="pwd" type="password" autocomplete="current-password">
<p class="err" id="err"></p>
<a class="pill wfull" href="#" id="go" onclick="go();return false">Masuk aplikasi <span class="arr">â†’</span></a>
</div></div></div>
<script>
let mode=1;function tab(m){mode=m;document.getElementById('t1').className=m===1?'on':'';document.getElementById('t2').className=m===2?'on':'';document.getElementById('f_reg').style.display=m===2?'block':'none';document.getElementById('go').innerHTML=(m===1?'Masuk aplikasi ':'Daftar & masuk ')+'<span class="arr">â†’</span>';}
async function go(){const e=document.getElementById('err');e.textContent='';document.getElementById('go').style.opacity='.6';
const url=mode===1?'/api/v1/auth/login':'/api/v1/auth/register';
const body=mode===1?{email:email.value,password:pwd.value}:{org_name:org.value,email:email.value,name:nm.value,password:pwd.value};
try{const r=await fetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});
const j=await r.json();if(!r.ok){e.textContent=mode===1?'Gagal masuk ('+r.status+'). Periksa email & kata sandi.':'Gagal daftar ('+r.status+'). Email mungkin sudah dipakai.';document.getElementById('go').style.opacity='1';return;}
localStorage.setItem('lr_token',j.token);location.href='/app';}catch(x){e.textContent='Server tidak terjangkau.';document.getElementById('go').style.opacity='1';}}
</script></body></html>"##;

const APP_HTML: &str = r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Dasbor â€” LEGALRUST</title><link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,500;9..144,700&family=Plus+Jakarta+Sans:wght@400;600;700;800&display=swap" rel="stylesheet"><style>CSS</style></head><body>
<div class="island"><a class="brand" href="/">LEGALRUST</a><nav><a href="#" onclick="out();return false">Keluar</a></nav></div>
<div class="wrap"><div class="dashhead"><div><span class="eyebrow">Ruang kerja</span><h1 class="serif" id="hi" style="font-size:40px;margin:12px 0 4px">Dasbor</h1></div><a class="pill ghost" href="/login" onclick="localStorage.removeItem('lr_token')">Ganti akun <span class="arr">â†’</span></a></div>
<p class="err" id="err"></p>
<div class="tabs" id="tabs"><button class="on" data-t="ringkasan" onclick="show('ringkasan')">Ringkasan</button><button data-t="dokumen" onclick="show('dokumen')">Dokumen</button><button data-t="akta" onclick="show('akta')">Akta</button><button data-t="keuangan" onclick="show('keuangan')">Keuangan</button></div>
<section id="tab-ringkasan"><section class="bento" id="g" style="padding-top:8px"></section>
<div class="shell s12 rv"><div class="core"><div class="n">PERKARA AKTIF</div><div id="m"><p style="color:var(--muted)">Memuatâ€¦</p></div></div></div></section>
<section id="tab-dokumen" style="display:none"><div class="shell s12"><div class="core"><div class="n">DOKUMEN</div><h3 class="serif">Dokumen</h3><div id="dlist"><p style="color:var(--muted)">Memuatâ€¦</p></div><h3 class="serif" style="margin-top:16px">Unggah dokumen</h3><label class="fl">Matter ID (opsional)</label><input class="fi" id="up_matter" inputmode="numeric" placeholder="cth 1"><label class="fl">Nama berkas</label><input class="fi" id="up_name" value="catatan.txt"><label class="fl">MIME</label><input class="fi" id="up_mime" value="text/plain"><label class="fl">Isi teks</label><input class="fi" id="up_text" value="hello legalrust"><p class="err" id="dmsg"></p><a class="pill" href="#" onclick="upDoc();return false">Unggah <span class="arr">â†‘</span></a></div></div></section>
<section id="tab-akta" style="display:none"><div class="shell s12"><div class="core"><div class="n">AKTA</div><h3 class="serif">Akta</h3><div id="alist"><p style="color:var(--muted)">Memuatâ€¦</p></div><p class="err" id="amsg"></p></div></div></section>
<section id="tab-keuangan" style="display:none"><div class="shell s12"><div class="core"><div class="n">KEUANGAN</div><h3 class="serif">Keuangan</h3><div id="ilist"><p style="color:var(--muted)">Memuatâ€¦</p></div><h3 class="serif" style="margin-top:16px">Trust accounts</h3><div id="tlist"><p style="color:var(--muted)">Memuatâ€¦</p></div><h3 class="serif" style="margin-top:16px">Transfer trust</h3><label class="fl">Dari (akun id)</label><input class="fi" id="tf_from" inputmode="numeric"><label class="fl">Ke (akun id)</label><input class="fi" id="tf_to" inputmode="numeric"><label class="fl">Nominal (minor)</label><input class="fi" id="tf_amt" inputmode="numeric" value="100000"><p class="err" id="tmsg"></p><a class="pill" href="#" onclick="doTransfer();return false">Transfer <span class="arr">â†’</span></a></div></div></section>
</div>
<script>
const t=localStorage.getItem('lr_token');if(!t)location.href='/login';
const H={Authorization:'Bearer '+t};
function show(n){document.querySelectorAll('#tabs button').forEach(b=>b.className=b.dataset.t===n?'on':'');for(const k of ['ringkasan','dokumen','akta','keuangan']){document.getElementById('tab-'+k).style.display=k===n?'block':'none';}}
async function j(u){const r=await fetch(u,{headers:H});if(r.status===401){location.href='/login';return null;}return r.json();}
async function loadDocs(){try{const d=await j('/api/v1/documents');document.getElementById('dlist').innerHTML='<table class="tbl"><tr><th>ID</th><th>Nama</th></tr>'+d.map(x=>'<tr><td><b>'+x.id+'</b></td><td>'+x.name+'</td></tr>').join('')+'</table>'||'<p>Belum ada dokumen.</p>';}catch(e){document.getElementById('dmsg').textContent='Gagal memuat dokumen: '+e;}}
async function hexSha(b){const h=await crypto.subtle.digest('SHA-256',b);return [...new Uint8Array(h)].map(x=>x.toString(16).padStart(2,'0')).join('');}
async function upDoc(){const m=document.getElementById('dmsg');m.textContent='';try{const enc=new TextEncoder().encode(document.getElementById('up_text').value);const sha=await hexSha(enc);const mid=document.getElementById('up_matter').value.trim();const meta={matter_id:mid===''?null:Number(mid),name:document.getElementById('up_name').value,mime:document.getElementById('up_mime').value,size:enc.length,sha};const r=await fetch('/api/v1/documents',{method:'POST',headers:{...H,'Content-Type':'application/json'},body:JSON.stringify(meta)});const jj=await r.json();if(!r.ok){m.textContent='Metadata gagal ('+r.status+'): '+(jj.error?jj.error.message:'');return;}const p=await fetch('/api/v1/documents/'+jj.id+'/content',{method:'PUT',headers:H,body:enc});if(!p.ok){m.textContent='PUT bytes gagal ('+p.status+')';return;}m.textContent='';loadDocs();}catch(e){m.textContent='Gagal unggah: '+e;}}
async function loadAkta(){try{const a=await j('/api/v1/deeds');const nxt=['collection','verification','legal_review','approval','signing'];document.getElementById('alist').innerHTML='<table class="tbl"><tr><th>ID</th><th>Nomor</th><th>Status</th><th>Aksi</th></tr>'+a.map(x=>'<tr><td><b>'+x.id+'</b></td><td>'+x.number+'</td><td><span class="badge">'+x.state+'</span></td><td>'+nxt.map(s=>'<button onclick="trDeed('+x.id+",'"+s+'\')">'+s+'</button>').join(' ')+'</td></tr>').join('')+'</table>'||'<p>Belum ada akta.</p>';}catch(e){document.getElementById('amsg').textContent='Gagal memuat akta: '+e;}}
async function trDeed(id,to){const m=document.getElementById('amsg');m.textContent='';const r=await fetch('/api/v1/deeds/'+id+'/transition',{method:'POST',headers:{...H,'Content-Type':'application/json'},body:JSON.stringify({to})});if(r.status===409){const jj=await r.json();m.textContent='Transisi ditolak (409): '+(jj.error?jj.error.message:'illegal transition');return;}if(!r.ok){m.textContent='Gagal transisi ('+r.status+')';return;}loadAkta();}
async function loadKeu(matters){try{let h='';for(const mm of matters.slice(0,5)){const inv=await j('/api/v1/matters/'+mm.id+'/invoices');h+='<p><b>'+mm.number+'</b> Â· '+inv.length+' invoice'+(inv.length?': '+inv.map(v=>v.number+' ('+v.total+')').join(', '):'')+'</p>';}document.getElementById('ilist').innerHTML=h||'<p>Belum ada perkara.</p>';const tr=await j('/api/v1/trust/accounts');document.getElementById('tlist').innerHTML='<table class="tbl"><tr><th>ID</th><th>Klien</th><th>Saldo</th></tr>'+tr.map(x=>'<tr><td><b>'+x.id+'</b></td><td>'+x.client_id+'</td><td>'+x.balance+'</td></tr>').join('')+'</table>'||'<p>Belum ada akun.</p>';}catch(e){document.getElementById('tmsg').textContent='Gagal memuat keuangan: '+e;}}
async function doTransfer(){const m=document.getElementById('tmsg');m.textContent='';try{const r=await fetch('/api/v1/trust/transfer',{method:'POST',headers:{...H,'Content-Type':'application/json'},body:JSON.stringify({from:Number(tf_from.value),to:Number(tf_to.value),amount_minor:Number(tf_amt.value)})});const jj=await r.json();if(r.status===409){m.textContent='Transfer ditolak (409): '+(jj.error?jj.error.message:'saldo tidak cukup');return;}if(!r.ok){m.textContent='Gagal transfer ('+r.status+')';return;}m.textContent='';loadKeu(window._m||[]);}catch(e){m.textContent='Gagal transfer: '+e;}}
(async()=>{try{
const me=await j('/api/v1/orgs/me');document.getElementById('hi').textContent='Dasbor â€” '+me.org_id.slice(0,8)+'â€¦';
const c=await j('/api/v1/clients'),m=await j('/api/v1/matters'),d=await j('/api/v1/documents'),a=await j('/api/v1/deeds'),tr=await j('/api/v1/trust/accounts');
window._m=m;
const cards=[['Klien',c.length,'nama terdaftar'],['Perkara',m.length,'matter aktif'],['Dokumen',d.length,'berkas berversi'],['Akta',a.length,'dalam alur'],['Trust',tr.length,'rekening dana'],['Saldo trust','Rp '+tr.reduce((s,x)=>s+(x.balance||0),0).toLocaleString('id-ID'),'total minor']];
document.getElementById('g').innerHTML=cards.map((x,i)=>'<div class="shell '+(i<2?'s6':'s4')+' rv in"><div class="core"><div class="n">0'+(i+1)+'</div><div style="font-size:30px;font-weight:800">'+x[1]+'</div><div style="color:var(--muted);font-size:14px">'+x[0]+' Â· '+x[2]+'</div></div></div>').join('');
document.getElementById('m').innerHTML='<table class="tbl"><tr><th>Nomor</th><th>Judul</th><th>Status</th></tr>'+m.map(x=>'<tr><td><b>'+x.number+'</b></td><td>'+x.title+'</td><td><span class="badge">'+x.status+'</span></td></tr>').join('')+'</table>'||'<p style="color:var(--muted)">Belum ada perkara.</p>';
loadDocs();loadAkta();loadKeu(m);
}catch(e){document.getElementById('err').textContent='Gagal memuat: '+e;}})();
async function out(){try{await fetch('/api/v1/auth/logout',{method:'POST',headers:H});}catch(x){}localStorage.removeItem('lr_token');location.href='/login';}
</script></body></html>"##;

fn page(html: &str) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html.replacen("CSS", PAGE_CSS, 1),
    )
}
async fn landing() -> impl IntoResponse {
    page(LANDING_HTML)
}
async fn login_page() -> impl IntoResponse {
    page(LOGIN_HTML)
}
async fn app_page() -> impl IntoResponse {
    page(APP_HTML)
}
// ---------- end public pages ----------

pub fn build_router(state: FullState) -> Router {
    let request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/", get(landing))
        .route("/login", get(login_page))
        .route("/app", get(app_page))
        .route("/static/{name}", get(static_img))
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
        b"test-secret-0123456789-0123456789ab".to_vec(),
    );
    stores.integration_secret = b"test-integration".to_vec();
    FullState {
        version: "0.1.0",
        pool: None,
        stores,
    }
}
