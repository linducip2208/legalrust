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
use legalrust_database::{
    CoreStore, DocRow, MarketingArticleRow, MarketingFaqRow, MarketingLeadRow, MarketingPlanRow,
    MarketingStore, MatterRow, TaskRow, UserStore,
};
use legalrust_database::{DeedRow, InvoiceRow, OcrJobRow, OpsStore, TemplateRow};
use legalrust_domain::{MarketingLeadStatus, Permission, Role};
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
    pub mkt: Arc<dyn MarketingStore>,
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
        mkt: Arc<dyn MarketingStore>,
        objects: Arc<dyn ObjectStore>,
        jwt_secret: Vec<u8>,
    ) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(64);
        Self {
            users,
            core,
            ops,
            mkt,
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
    if name.contains("..") || name.contains('\\') || name.starts_with('/') {
        return Err(AppError::NotFound("asset".into()));
    }
    let ctype = match name.rsplit('.').next().unwrap_or("") {
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ttf" => "font/ttf",
        "woff2" => "font/woff2",
        _ => return Err(AppError::NotFound("asset".into())),
    };
    let bytes =
        match tokio::fs::read(std::path::PathBuf::from("apps/server/static").join(&name)).await {
            Ok(b) => b,
            Err(_) => tokio::fs::read(std::path::PathBuf::from("static").join(&name))
                .await
                .map_err(|_| AppError::NotFound("asset".into()))?,
        };
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, ctype)],
        bytes,
    ))
}
async fn not_found() -> AppError {
    AppError::NotFound("route".into())
}

// ---------- public pages (landing + login + app shell + marketing site) ----------
// All public HTML is produced through `page_shell` (single head/nav/footer).
// CSP-safe: no inline <style>, no style=""/onclick attributes, no inline behavior
// scripts — only Tabler + /static/site.css + /static/site.js references.
// Dropdowns use native <details> (no JS); the mobile hamburger is a <details>
// toggle (static assets are intentionally left untouched).

const SITE_BASE: &str = "https://legalrust.test";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn fmt_idr(minor: i64) -> String {
    let neg = minor < 0;
    let mut s = minor.abs().to_string();
    let mut groups: Vec<String> = Vec::new();
    while s.len() > 3 {
        let cut = s.len() - 3;
        groups.push(s[cut..].to_string());
        s.truncate(cut);
    }
    groups.push(s);
    groups.reverse();
    format!("{}Rp {}", if neg { "-" } else { "" }, groups.join("."))
}

fn badge(status: &str) -> String {
    let cls = match status {
        "TERSEDIA" => "bg-green-lt",
        "TERSEDIA TERBATAS" => "bg-yellow-lt",
        _ => "bg-red-lt",
    };
    format!(r#"<span class="badge {cls}">{status}</span>"#)
}

fn cta_row() -> String {
    r#"<div class="cta-row"><a class="a" href="/demo">Minta Demo</a><a class="b" href="/login">Mulai Gratis</a></div>"#
        .to_string()
}

fn endpoint_table(rows: &[(&str, &str)]) -> String {
    let mut out = String::from(
        r#"<div class="table-responsive"><table class="table table-sm"><thead><tr><th>Endpoint</th><th>Keterangan</th></tr></thead><tbody>"#,
    );
    for (ep, desc) in rows {
        out.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td></tr>",
            esc(ep),
            esc(desc)
        ));
    }
    out.push_str("</tbody></table></div>");
    out
}

/// Shared shell: full HTML head (Tabler + site.css + meta/OG/canonical +
/// minimal Organization JSON-LD) + topbar nav + footer.
fn page_shell(title: &str, desc: &str, path: &str, active: &str, body: &str) -> String {
    let on = |key: &str| {
        if key == active {
            " on"
        } else {
            ""
        }
    };
    let nav = format!(
        r##"<nav class="nav d-none d-md-flex" aria-label="Navigasi utama"><a href="/" class="{o0}">Beranda</a><details class="nav-dd"><summary>Fitur</summary><div class="nav-dd-menu"><a href="/fitur">Semua Fitur</a><a href="/fitur/manajemen-perkara">Manajemen Perkara</a><a href="/fitur/manajemen-dokumen">Manajemen Dokumen</a><a href="/fitur/ocr">OCR</a><a href="/fitur/ai-legal-assistant">AI Legal Assistant</a><a href="/fitur/generasi-akta">Generasi Akta</a><a href="/fitur/tanda-tangan">Tanda Tangan</a></div></details><details class="nav-dd"><summary>Solusi</summary><div class="nav-dd-menu"><a href="/solusi">Semua Solusi</a><a href="/solusi/notaris">Notaris</a><a href="/solusi/ppat">PPAT</a><a href="/solusi/firma-hukum">Firma Hukum</a><a href="/solusi/legal-corporate">Legal Corporate</a></div></details><a href="/harga" class="{o1}">Harga</a><a href="/tentang" class="{o2}">Tentang</a><a href="/artikel" class="{o3}">Artikel</a><a href="/kontak" class="{o4}">Kontak</a></nav><details class="mnav d-md-none"><summary aria-label="Menu navigasi">Menu</summary><nav aria-label="Navigasi seluler"><a href="/">Beranda</a><a href="/fitur">Fitur</a><a href="/solusi">Solusi</a><a href="/harga">Harga</a><a href="/tentang">Tentang</a><a href="/artikel">Artikel</a><a href="/kontak">Kontak</a><a href="/demo">Demo</a><a href="/faq">FAQ</a></nav></details>"##,
        o0 = on("Beranda"),
        o1 = on("Harga"),
        o2 = on("Tentang"),
        o3 = on("Artikel"),
        o4 = on("Kontak"),
    );
    format!(
        r#"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{t} — LegalRust</title><meta name="description" content="{d}"><link rel="canonical" href="{b}{p}"><meta property="og:title" content="{t} — LegalRust"><meta property="og:description" content="{d}"><meta property="og:type" content="website"><meta property="og:url" content="{b}{p}"><link rel="stylesheet" href="/static/vendor/tabler.min.css?v=2"><link rel="stylesheet" href="/static/vendor/tabler-fonts/tabler-icons.min.css?v=2"><link rel="stylesheet" href="/static/site.css?v=2"><script type="application/ld+json">{{"@context":"https://schema.org","@type":"Organization","name":"LegalRust","url":"{b}","description":"Platform manajemen hukum untuk notaris, PPAT, firma hukum, dan profesional legal."}}</script></head><body><div class="container-xl"><div class="topbar"><div class="logo">LegalRust<small>Smart Legal Solution</small></div>{n}<div class="topact"><a href="/login" class="btn-line sm">Masuk</a><a href="/login" class="btn-pri sm">Coba Gratis</a></div></div></div><main class="container-xl page-main">{body}</main><div class="container-xl"><footer class="site"><span>2026 LegalRust — Smart Legal Solution</span><span><a href="/kebijakan-privasi">Privasi</a> · <a href="/syarat-ketentuan">Syarat</a> · <a href="/faq">FAQ</a> · <a href="/sitemap.xml">Peta Situs</a> · <a href="/healthz">Status</a></span></footer></div><script src="/static/vendor/tabler.min.js?v=2"></script><script src="/static/site.js?v=2"></script></body></html>"#,
        t = esc(title),
        d = esc(desc),
        b = SITE_BASE,
        p = path,
        n = nav,
        body = body,
    )
}

fn html_ok(s: String) -> (StatusCode, [(&'static str, &'static str); 1], String) {
    (
        StatusCode::OK,
        [("content-type", "text/html; charset=utf-8")],
        s,
    )
}
fn html_status(
    st: StatusCode,
    s: String,
) -> (StatusCode, [(&'static str, &'static str); 1], String) {
    (st, [("content-type", "text/html; charset=utf-8")], s)
}

// ----- landing / login / app (share page_shell) -----
async fn landing() -> impl IntoResponse {
    let body = r#"<section class="hero"><div class="hero-in"><div><span class="badge-soft rv">Solusi LegalTech Terdepan di Indonesia</span><h1 class="rv">Kelola Perkara, Dokumen, dan Akta dalam Satu Platform Aman</h1><p class="lead rv">LegalRust membantu notaris, PPAT, dan firma hukum mengelola perkara, dokumen berversi dengan pin SHA-256, akta dari template dengan alur ber-guard, verifikasi identitas via OCR job plus review manusia, serta keuangan invoice anti-overpay dan trust ledger double-entry.</p><div class="cta-row"><a class="a" href="/app">Lihat Demo</a><a class="b" href="/login">Mulai Gratis Sekarang</a></div><div class="checks rv"><span><i class="tick">v</i>Gratis Selamanya</span><span><i class="tick">v</i>Aman dan Terpercaya</span><span><i class="tick">v</i>Jejak Audit Penuh</span></div></div><div class="mock rv"><div class="mock-top"><span class="dot"></span><span class="dot"></span><span class="dot"></span><span>LegalRust — pratinjau dasbor (ilustrasi)</span></div><div class="mock-body"><div class="side"><div class="logo2">LegalRust</div><div class="mi on">Dasbor</div><div class="mi">Perkara</div><div class="mi">Dokumen</div><div class="mi">Akta</div><div class="mi">Keuangan</div></div><div class="dmain"><div class="msearch"><span class="box">Cari perkara, klien, atau dokumen</span><span class="me"><span class="av">B</span><span><b>Budi Santoso</b><br><small>Notaris</small></span></span></div><div class="stat4"><div class="stat">Total Perkara<b>128</b><span class="up">+ 12,5%</span></div><div class="stat">Dokumen Diproses<b>842</b><span class="up">+ 18,7%</span></div><div class="stat">Klien Aktif<b>256</b><span class="up">+ 8,2%</span></div><div class="stat">Pendapatan<b>Rp 248 jt</b><span class="up">+ 14,6%</span></div></div></div></div></div></div></section><div class="trust rv"><div><b>Dipercaya oleh</b><br>Ribuan Profesional</div><span class="seg">NOTARIS</span><span class="seg">PPAT</span><span class="seg">FIRMA HUKUM</span><span class="seg">KORPORAT</span><span class="seg">ADVOKAT</span></div><section class="feat" id="fitur"><span class="badge-soft tag rv">FITUR UNGGULAN</span><h2 class="rv">Semua yang Anda Butuhkan dalam Satu Platform</h2><p class="sub rv">Dari manajemen perkara hingga tanda tangan digital, semua terintegrasi dengan sempurna.</p><div class="fgrid"><div class="fcard rv"><div class="ic blue">P</div><h3>Manajemen Perkara Terintegrasi</h3><p>Kelola seluruh perkara dengan mudah dan efisien. <a href="/fitur/manajemen-perkara">Pelajari</a></p></div><div class="fcard rv"><div class="ic purple">O</div><h3>OCR dan Verifikasi Dokumen</h3><p>Job OCR per tipe dokumen dengan review manusia. <a href="/fitur/ocr">Pelajari</a></p></div><div class="fcard rv"><div class="ic green">T</div><h3>Tanda Tangan Digital</h3><p>Alur akta teraudit; aktivasi PSrE bertahap. <a href="/fitur/tanda-tangan">Pelajari</a></p></div><div class="fcard rv"><div class="ic orange">A</div><h3>Pengelolaan Akta</h3><p>Buat akta dan kontrak dengan template profesional. <a href="/fitur/generasi-akta">Pelajari</a></p></div><div class="fcard rv"><div class="ic purple">L</div><h3>Laporan dan Analitik</h3><p>Ekspor CSV perkara dan jejak audit role-gated. <a href="/app">Pelajari</a></p></div><div class="fcard rv"><div class="ic green2">K</div><h3>Keamanan Tingkat Tinggi</h3><p>Argon2, JWT revocable, RBAC per endpoint. <a href="/fitur/manajemen-dokumen">Pelajari</a></p></div></div></section><section class="cta rv"><div><h2>Siap Meningkatkan Produktivitas Kantor Hukum Anda?</h2><p>Bergabunglah dengan ribuan profesional yang telah mempercayai LegalRust.</p><a class="go" href="/login">Mulai Sekarang</a></div><ul><li>Gratis selamanya</li><li>Setup mudah dalam 1 menit</li><li>Support profesional</li></ul></section>"#;
    html_ok(page_shell(
        "Kelola Perkara, Dokumen dan Akta",
        "Platform manajemen hukum terpadu untuk notaris, PPAT, firma hukum, dan profesional legal.",
        "/",
        "Beranda",
        body,
    ))
}

async fn login_page() -> impl IntoResponse {
    let body = r#"<div class="page page-center"><div class="container container-tight py-4"><div class="text-center mb-4"><a href="/">LegalRust</a><div class="text-muted">Smart Legal Solution</div></div><div class="card card-md"><div class="card-body"><h1 class="h2 text-center mb-1">Masuk</h1><p class="text-muted text-center">Sesi JWT plus proteksi brute-force 5x percobaan</p><div class="btn-group w-100 mb-3"><button class="btn btn-primary" id="t1">Masuk</button><button class="btn" id="t2">Daftar organisasi</button></div><div id="f_reg" class="hidden"><div class="mb-2"><label class="form-label" for="org">Nama organisasi</label><input class="form-control" id="org"></div><div class="mb-2"><label class="form-label" for="nm">Nama Anda</label><input class="form-control" id="nm"></div></div><div class="mb-2"><label class="form-label" for="email">Email</label><input class="form-control" id="email" type="email" autocomplete="username"></div><div class="mb-2"><label class="form-label" for="pwd">Kata sandi</label><input class="form-control" id="pwd" type="password" autocomplete="current-password"></div><div class="text-danger mb-2" id="err"></div><button class="btn btn-primary w-100" id="go">Masuk aplikasi</button><button class="btn btn-ghost-secondary w-100 mt-2" id="demo">Gunakan akun demo</button><details class="mt-3"><summary class="text-muted">Peran dan hak akses</summary><div class="table-responsive mt-2"><table class="table table-sm"><thead><tr><th>Peran</th><th>Hak akses ringkas</th></tr></thead><tbody><tr><td><span class="badge bg-blue-lt">org_owner</span></td><td>Semua hak akses</td></tr><tr><td><span class="badge bg-blue-lt">office_admin</span></td><td>Kelola harian plus Assign</td></tr><tr><td><span class="badge bg-blue-lt">notary / ppat</span></td><td>Akta: Sign, Verify, Approve</td></tr><tr><td><span class="badge bg-blue-lt">lawyer</span></td><td>Perkara dan dokumen</td></tr><tr><td><span class="badge bg-blue-lt">finance_officer</span></td><td>Keuangan: Approve</td></tr><tr><td><span class="badge bg-blue-lt">auditor</span></td><td>Lihat dan ekspor saja</td></tr></tbody></table></div></details></div></div><div class="text-center text-muted mt-3">Kembali ke <a href="/">Beranda</a></div></div></div>"#;
    html_ok(page_shell(
        "Masuk",
        "Masuk ke dasbor LegalRust dengan sesi JWT yang aman.",
        "/login",
        "",
        body,
    ))
}

async fn app_page() -> impl IntoResponse {
    let body = r#"<div class="page"><header class="navbar navbar-expand-md navbar-light bg-white border-bottom"><div class="container-xl"><a href="/" class="navbar-brand">LegalRust</a><ul class="navbar-nav mx-auto" id="tabs"><li class="nav-item"><a class="nav-link active" href="/app">Ringkasan</a></li><li class="nav-item"><a class="nav-link" href="/app">Dokumen</a></li><li class="nav-item"><a class="nav-link" href="/app">Akta</a></li><li class="nav-item"><a class="nav-link" href="/app">Keuangan</a></li></ul><button class="btn btn-outline-danger btn-sm logout">Keluar</button></div></header><div class="page-wrapper"><div class="container-xl py-4"><h1 class="page-title" id="hi">Dasbor</h1><div class="text-danger" id="err"></div><div id="p-ringkas"><div class="row g-2" id="g"></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Perkara Aktif</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody id="m"></tbody></table></div></div></div><div id="p-dok" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Dokumen</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th></tr></thead><tbody id="dl"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Unggah dokumen</h2><input class="form-control mb-2" id="dn" placeholder="nama berkas, mis. kontrak.pdf"><input class="form-control mb-2" type="file" id="df"><button class="btn btn-primary" id="doup">Unggah</button> <span class="text-danger" id="ue"></span></div></div></div><div id="p-akta" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Akta</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="al"></tbody></table></div></div><div class="text-muted mt-2" id="ae"></div></div><div id="p-keu" class="hidden"><div class="row g-2"><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Invoice per perkara</h2></div><div class="card-body"><input class="form-control mb-2" id="km" placeholder="ID perkara"><button class="btn btn-secondary mb-2" id="showinv">Tampilkan</button><div id="il"></div></div></div></div><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Trust transfer</h2></div><div class="card-body"><input class="form-control mb-2" id="tf" placeholder="dari rekening ID"><input class="form-control mb-2" id="tt" placeholder="ke rekening ID"><input class="form-control mb-2" id="ta" placeholder="nominal minor"><button class="btn btn-primary" id="dotrf">Transfer</button> <span class="text-danger" id="te"></span></div></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Rekening trust</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Klien</th><th>Saldo</th></tr></thead><tbody id="tl"></tbody></table></div></div></div><div class="mt-3"><a href="/login" class="btn btn-outline-secondary btn-sm logout">Ganti akun</a></div></div></div></div>"#;
    html_ok(page_shell(
        "Dasbor",
        "Dasbor aplikasi LegalRust: ringkasan perkara, dokumen, akta, dan keuangan.",
        "/app",
        "",
        body,
    ))
}

// ----- feature pages -----
fn feature_body(
    headline: &str,
    status: &str,
    intro: &str,
    rows: &[(&str, &str)],
    note: &str,
) -> String {
    format!(
        r#"<section class="feat"><span class="badge-soft tag">FITUR</span><h1>{h}</h1><p class="sub">{b} {intro}</p>{cta}</section><section class="card"><div class="card-body"><h2 class="card-title">Endpoint terkait</h2>{tbl}<h2 class="card-title mt-3">Catatan status</h2><p class="text-muted">{note}</p></div></section>"#,
        h = esc(headline),
        b = badge(status),
        intro = esc(intro),
        cta = cta_row(),
        tbl = endpoint_table(rows),
        note = esc(note),
    )
}

async fn fitur_index() -> impl IntoResponse {
    let body = format!(
        r#"<section class="feat"><span class="badge-soft tag">FITUR</span><h1>Fitur Lengkap LegalRust</h1><p class="sub">Enam kemampuan inti dalam satu platform teraudit.</p>{}</section><div class="fgrid"><div class="fcard"><h3>Manajemen Perkara Terintegrasi</h3><p>Klien, perkara, tugas bertransisi aman, notifikasi. <a href="/fitur/manajemen-perkara">Pelajari</a></p></div><div class="fcard"><h3>Manajemen Dokumen Berversi</h3><p>Upload SHA-pin, versi, share kedaluwarsa. <a href="/fitur/manajemen-dokumen">Pelajari</a></p></div><div class="fcard"><h3>OCR dan Verifikasi</h3><p>Job OCR plus review manusia. <a href="/fitur/ocr">Pelajari</a></p></div><div class="fcard"><h3>AI Legal Assistant</h3><p>RAG keyword live; chat provider-gated. <a href="/fitur/ai-legal-assistant">Pelajari</a></p></div><div class="fcard"><h3>Generasi Akta</h3><p>Template berversi plus state machine ber-guard. <a href="/fitur/generasi-akta">Pelajari</a></p></div><div class="fcard"><h3>Tanda Tangan Digital</h3><p>Alur teraudit; PSrE bertahap. <a href="/fitur/tanda-tangan">Pelajari</a></p></div></div>"#,
        cta_row()
    );
    html_ok(page_shell(
        "Fitur Lengkap",
        "Enam fitur inti LegalRust: perkara, dokumen, OCR, AI, akta, dan tanda tangan digital.",
        "/fitur",
        "",
        &body,
    ))
}

async fn fitur_perkara() -> impl IntoResponse {
    html_ok(page_shell(
        "Manajemen Perkara Terintegrasi",
        "Kelola klien, perkara, dan tugas bertransisi aman dengan notifikasi dan audit trail.",
        "/fitur/manajemen-perkara",
        "",
        &feature_body(
            "Manajemen Perkara Terintegrasi",
            "TERSEDIA",
            "Kelola klien, perkara, dan tugas dengan transisi status yang dijaga dan notifikasi in-app.",
            &[
                ("POST /api/v1/clients", "Buat klien individual, korporat, atau pemerintah"),
                ("POST /api/v1/matters", "Buat perkara dengan nomor otomatis"),
                ("POST /api/v1/matters/{mid}/tasks", "Tambah tugas ke perkara"),
                ("POST /api/v1/tasks/{tid}", "Ubah status tugas (transisi ilegal ditolak 409)"),
                ("GET /api/v1/notifications", "Notifikasi in-app per pengguna"),
            ],
            "Live dan tercakup RBAC per endpoint; setiap mutasi tercatat di audit trail.",
        ),
    ))
}

async fn fitur_dokumen() -> impl IntoResponse {
    html_ok(page_shell(
        "Manajemen Dokumen Berversi",
        "Dokumen berversi SHA-pin, tautan share 1-720 jam, dan unduh ber-RBAC.",
        "/fitur/manajemen-dokumen",
        "",
        &feature_body(
            "Manajemen Dokumen Berversi",
            "TERSEDIA",
            "Unggah dokumen yang dipin SHA-256, kelola versi, dan bagikan lewat tautan kedaluwarsa.",
            &[
                ("POST /api/v1/documents", "Registrasi metadata dokumen plus hash SHA-256"),
                ("PUT /api/v1/documents/{id}/content", "Unggah isi (ukuran dan hash harus cocok)"),
                ("GET /api/v1/documents/{id}/content", "Unduh versi terbaru (butuh permission Download)"),
                ("POST /api/v1/documents/{id}/shares", "Buat tautan share 1-720 jam"),
                ("GET /api/v1/reports/matters.csv", "Ekspor CSV perkara"),
            ],
            "Live; unduhan dan share dijaga RBAC dan masa berlaku.",
        ),
    ))
}

async fn fitur_ocr() -> impl IntoResponse {
    html_ok(page_shell(
        "OCR dan Verifikasi Dokumen",
        "Job OCR KTP, KK, NPWP, paspor, dan sertifikat dengan skor kepercayaan plus review manusia.",
        "/fitur/ocr",
        "",
        &feature_body(
            "OCR dan Verifikasi Dokumen",
            "BUTUH KONFIGURASI",
            "Antrean job OCR dan review approve/reject live; eksekusi OCR live butuh engine yang dikonfigurasi.",
            &[
                ("GET /api/v1/ocr/supported", "Daftar tipe dokumen yang didukung"),
                ("POST /api/v1/ocr/jobs", "Buat job OCR untuk sebuah dokumen"),
                ("POST /api/v1/ocr/jobs/{id}/review", "Keputusan approve/reject (butuh permission Verify)"),
                ("POST /api/v1/ocr/jobs/{id}/process", "Pemrosesan live — 501 tanpa engine OCR"),
            ],
            "Jujur: POST /api/v1/ocr/jobs/{id}/process mengembalikan 501 karena libtesseract native dan cloud fallback belum dikonfigurasi. Job tetap antre, tidak ada yang diproses diam-diam.",
        ),
    ))
}

async fn fitur_ai() -> impl IntoResponse {
    html_ok(page_shell(
        "Asisten Legal AI",
        "RAG keyword live plus pencatatan token; generasi chat provider-gated.",
        "/fitur/ai-legal-assistant",
        "",
        &feature_body(
            "Asisten Legal AI",
            "BUTUH KONFIGURASI",
            "Indeks dan pencarian RAG live untuk dokumen teks; chat generatif menunggu provider AI.",
            &[
                ("POST /api/v1/rag/index", "Indeks dokumen teks menjadi chunk pencarian"),
                ("GET /api/v1/rag/search", "Pencarian keyword full-text per organisasi"),
                ("POST /api/v1/ai/chat", "Chat generatif — 501 tanpa AI provider"),
                ("GET /api/v1/ai/usage", "Meteran token input/output per organisasi"),
            ],
            "Jujur: POST /api/v1/ai/chat mengembalikan 501 hingga provider AI dikonfigurasi; estimasi token tetap dicatat agar biaya transparan.",
        ),
    ))
}

async fn fitur_akta() -> impl IntoResponse {
    html_ok(page_shell(
        "Generasi Akta dari Template",
        "Template berversi, render variabel, dan state machine anti-transisi ilegal.",
        "/fitur/generasi-akta",
        "",
        &feature_body(
            "Generasi Akta dari Template",
            "TERSEDIA TERBATAS",
            "Template berversi dengan substitusi variabel dan alur status ber-guard; e-sign menunggu PSrE.",
            &[
                ("POST /api/v1/deed-templates", "Buat template akta berversi"),
                ("POST /api/v1/deeds/from-template", "Render akta dari template plus variabel"),
                ("POST /api/v1/deeds/{id}/transition", "Pindah status (ilegal ditolak 409)"),
                ("POST /api/v1/deeds/{id}/esign", "Tanda tangan elektronik — 501 tanpa PSrE"),
            ],
            "Terbatas: seluruh alur draf hingga approval live; langkah signing menunggu provider PSrE dikonfigurasi dan tidak pernah diklaim bekerja.",
        ),
    ))
}

async fn fitur_ttd() -> impl IntoResponse {
    html_ok(page_shell(
        "Tanda Tangan Digital",
        "Alur akta teraudit; tanda tangan elektronik PSrE terencana dan belum aktif.",
        "/fitur/tanda-tangan",
        "",
        &feature_body(
            "Tanda Tangan Digital",
            "BUTUH KONFIGURASI",
            "Persiapan dokumen dan persetujuan live; eksekusi tanda tangan menunggu provider PSrE.",
            &[
                ("POST /api/v1/deeds/{id}/transition", "Bawa akta ke status siap-signing"),
                ("POST /api/v1/deeds/{id}/esign", "Eksekusi e-sign — 501 tanpa provider PSrE"),
                ("POST /api/v1/webhooks/incoming/{provider}", "Webhook status provider (butuh secret)"),
            ],
            "Jujur: tanda tangan digital belum dapat dieksekusi; endpoint mengembalikan 501 hingga PSrE dikonfigurasi. Kami tidak mengklaimnya bekerja.",
        ),
    ))
}

// ----- solusi pages -----
fn solusi_body(headline: &str, intro: &str, points: &[&str]) -> String {
    let mut list = String::from("<ul>");
    for p in points {
        list.push_str(&format!("<li>{}</li>", esc(p)));
    }
    list.push_str("</ul>");
    format!(
        r#"<section class="feat"><span class="badge-soft tag">SOLUSI</span><h1>{h}</h1><p class="sub">{i}</p>{c}</section><section class="card"><div class="card-body"><h2 class="card-title">Cara kerja untuk Anda</h2>{l}<p class="text-muted">Tanpa testimoni fiktif dan tanpa angka pengguna: yang kami janjikan adalah alur teraudit di atas endpoint yang benar-benar live.</p></div></section>"#,
        h = esc(headline),
        i = esc(intro),
        c = cta_row(),
        l = list,
    )
}

async fn solusi_index() -> impl IntoResponse {
    let body = format!(
        r#"<section class="feat"><span class="badge-soft tag">SOLUSI</span><h1>Solusi untuk Setiap Praktik Hukum</h1><p class="sub">Alur yang disesuaikan untuk notaris, PPAT, firma hukum, dan legal corporate.</p>{}</section><div class="fgrid"><div class="fcard"><h3>Notaris</h3><p>Draf akta ke review ke approval dalam satu alur teraudit. <a href="/solusi/notaris">Pelajari</a></p></div><div class="fcard"><h3>PPAT</h3><p>Verifikasi dokumen tanah via OCR plus review. <a href="/solusi/ppat">Pelajari</a></p></div><div class="fcard"><h3>Firma Hukum</h3><p>Perkara, tugas, dan invoice per matter. <a href="/solusi/firma-hukum">Pelajari</a></p></div><div class="fcard"><h3>Legal Corporate</h3><p>Multi-org, peran, dan token API. <a href="/solusi/legal-corporate">Pelajari</a></p></div></div>"#,
        cta_row()
    );
    html_ok(page_shell(
        "Solusi Praktik Hukum",
        "Solusi LegalRust untuk notaris, PPAT, firma hukum, dan legal corporate.",
        "/solusi",
        "",
        &body,
    ))
}

async fn solusi_notaris() -> impl IntoResponse {
    html_ok(page_shell(
        "Software Notaris: Akta Teraudit",
        "Draf akta ke review ke approval ke signing dalam satu alur teraudit untuk kantor notaris.",
        "/solusi/notaris",
        "",
        &solusi_body(
            "Software Notaris: Akta Teraudit",
            "Draf akta ke review ke approval ke signing dalam satu alur teraudit untuk kantor notaris.",
            &[
                "Template akta berversi (POST /api/v1/deed-templates) dengan render variabel.",
                "Transisi status ber-guard: lompatan ilegal ditolak dengan 409.",
                "E-sign PSrE terencana — status BUTUH KONFIGURASI, tidak diklaim live.",
                "Audit trail per organisasi untuk setiap mutasi akta.",
            ],
        ),
    ))
}

async fn solusi_ppat() -> impl IntoResponse {
    html_ok(page_shell(
        "Software PPAT: Verifikasi Sertifikat",
        "Pipeline OCR plus review dokumen tanah sebelum penomoran akta.",
        "/solusi/ppat",
        "",
        &solusi_body(
            "Software PPAT: Verifikasi Sertifikat",
            "Pipeline OCR plus review dokumen tanah sebelum penomoran akta.",
            &[
                "Job OCR sertifikat dan identitas (POST /api/v1/ocr/jobs) plus daftar tipe (GET /api/v1/ocr/supported).",
                "Review manusia approve/reject dengan permission Verify.",
                "Arsip dokumen berversi SHA-pin untuk setiap berkas tanah.",
                "Eksekusi OCR live BUTUH KONFIGURASI engine.",
            ],
        ),
    ))
}

async fn solusi_firma() -> impl IntoResponse {
    html_ok(page_shell(
        "Manajemen Firma Hukum dan Advokat",
        "Perkara plus tugas plus invoice per matter plus ekspor CSV pelaporan.",
        "/solusi/firma-hukum",
        "",
        &solusi_body(
            "Manajemen Firma Hukum dan Advokat",
            "Perkara plus tugas plus invoice per matter plus ekspor CSV pelaporan.",
            &[
                "Perkara dan tugas bertransisi aman (POST /api/v1/matters, POST /api/v1/tasks/{tid}).",
                "Invoice per matter dengan guard anti-overpay.",
                "Ekspor CSV (GET /api/v1/reports/matters.csv) untuk pelaporan.",
                "Berbagi dokumen via tautan kedaluwarsa 1-720 jam.",
            ],
        ),
    ))
}

async fn solusi_corporate() -> impl IntoResponse {
    html_ok(page_shell(
        "Legal Corporate: Multi-Org dan Integrasi",
        "Multi-organisasi, peran Owner, Admin, dan Auditor, plus token API untuk integrasi internal.",
        "/solusi/legal-corporate",
        "",
        &solusi_body(
            "Legal Corporate: Multi-Org dan Integrasi",
            "Multi-organisasi, peran Owner, Admin, dan Auditor, plus token API untuk integrasi internal.",
            &[
                "Segregasi data per organisasi dengan RBAC per endpoint.",
                "Token API revocable (POST /api/v1/auth/tokens) untuk integrasi internal.",
                "Audit trail role-gated (GET /api/v1/audit) untuk Owner, Admin, dan Auditor.",
                "Trust ledger double-entry anti-negatif untuk dana klien.",
            ],
        ),
    ))
}

// ----- harga / tentang / kontak / demo -----
async fn harga(State(s): State<FullState>) -> impl IntoResponse {
    let plans = s
        .stores
        .mkt
        .mkt_pricing_list()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.active)
        .collect::<Vec<_>>();
    let inner = if plans.is_empty() {
        "<p>Paket kami: <strong>Free Trial</strong>, <strong>Professional</strong>, <strong>Business</strong>, dan <strong>Enterprise</strong> dengan struktur bulanan plus tahunan. Harga final ditentukan owner — hubungi tim penjualan untuk penawaran.</p>".to_string()
    } else {
        let mut t = String::from(
            r#"<div class="table-responsive"><table class="table"><thead><tr><th>Paket</th><th>Bulanan</th><th>Tahunan</th><th>Fitur</th></tr></thead><tbody>"#,
        );
        for p in &plans {
            let feats: String = serde_json::from_str::<Vec<String>>(&p.features_json)
                .map(|v| v.join(", "))
                .unwrap_or_else(|_| p.features_json.clone());
            t.push_str(&format!(
                "<tr><td><strong>{}</strong><br><small class=\"text-muted\">{}</small></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc(&p.name),
                esc(&p.code),
                esc(&fmt_idr(p.monthly_minor)),
                esc(&fmt_idr(p.annual_minor)),
                esc(&feats),
            ));
        }
        t.push_str("</tbody></table></div>");
        t
    };
    let body = format!(
        r#"<section class="feat"><span class="badge-soft tag">HARGA</span><h1>Harga LegalRust — Trial sampai Enterprise</h1><p class="sub">Empat tier: Free Trial, Professional, Business, Enterprise. Struktur bulanan plus tahunan.</p>{}</section><section class="card"><div class="card-body"><h2 class="card-title">Pilih paket Anda</h2>{}</div></section>"#,
        cta_row(),
        inner
    );
    html_ok(page_shell(
        "Harga LegalRust",
        "Empat tier: Free Trial, Professional, Business, Enterprise. Harga final oleh owner.",
        "/harga",
        "Harga",
        &body,
    ))
}

async fn tentang() -> impl IntoResponse {
    let body = format!(
        r#"<section class="feat"><span class="badge-soft tag">TENTANG</span><h1>Tentang LegalRust</h1><p class="sub">Platform manajemen hukum modern yang dibangun dengan Rust: cepat, aman, dan teraudit.</p>{}</section><section class="card"><div class="card-body"><h2 class="card-title">Status jujur</h2><p>LegalRust berada pada tahap uji internal — bukan production-ready. Setiap halaman fitur mencantumkan badge status yang jujur: TERSEDIA, TERSEDIA TERBATAS, atau BUTUH KONFIGURASI. Integrasi eksternal (OCR live, AI provider, PSrE, payment gateway) ditandai terbuka dan mengembalikan 501 hingga dikonfigurasi.</p><h2 class="card-title">Prinsip keamanan</h2><p>Password Argon2, sesi JWT revocable, lockout brute-force 5x per 5 menit, RBAC per endpoint, header CSP dan HSTS, serta audit trail yang hanya dapat dibaca peran terotorisasi.</p></div></section>"#,
        cta_row()
    );
    html_ok(page_shell(
        "Tentang LegalRust",
        "Tentang LegalRust: platform manajemen hukum Rust yang cepat, aman, dan teraudit.",
        "/tentang",
        "Tentang",
        &body,
    ))
}

async fn kontak() -> impl IntoResponse {
    let body = r#"<section class="feat"><span class="badge-soft tag">KONTAK</span><h1>Hubungi Kami</h1><p class="sub">Ceritakan kebutuhan kantor Anda; tim kami akan merespons melalui email.</p></section><section class="card"><div class="card-body"><h2 class="card-title">Formulir kontak</h2><form id="kontak-form" method="post" action="/api/v1/public/contact"><div class="mb-2"><label class="form-label" for="k-nama">Nama</label><input class="form-control" id="k-nama" name="name" required minlength="2"></div><div class="mb-2"><label class="form-label" for="k-email">Email</label><input class="form-control" id="k-email" name="email" type="email" required></div><div class="mb-2"><label class="form-label" for="k-subjek">Subjek</label><input class="form-control" id="k-subjek" name="subject" required minlength="2"></div><div class="mb-2"><label class="form-label" for="k-pesan">Pesan</label><textarea class="form-control" id="k-pesan" name="message" rows="4" required minlength="5"></textarea></div><button class="btn btn-primary" type="submit">Kirim Pesan</button></form></div></section>"#;
    html_ok(page_shell(
        "Hubungi Kami",
        "Hubungi tim LegalRust untuk penawaran, demo, dan pertanyaan produk.",
        "/kontak",
        "Kontak",
        body,
    ))
}

async fn demo() -> impl IntoResponse {
    let body = r#"<section class="feat"><span class="badge-soft tag">DEMO</span><h1>Minta Demo LegalRust</h1><p class="sub">Isi formulir singkat; kami akan menjadwalkan sesi demo untuk tim Anda.</p></section><section class="card"><div class="card-body"><h2 class="card-title">Formulir demo</h2><form id="demo-form" method="post" action="/api/v1/public/demo-requests"><div class="mb-2"><label class="form-label" for="d-nama">Nama</label><input class="form-control" id="d-nama" name="name" required minlength="2"></div><div class="mb-2"><label class="form-label" for="d-email">Email</label><input class="form-control" id="d-email" name="email" type="email" required></div><div class="mb-2"><label class="form-label" for="d-perusahaan">Perusahaan</label><input class="form-control" id="d-perusahaan" name="company"></div><div class="mb-2"><label class="form-label" for="d-waktu">Waktu yang diinginkan</label><input class="form-control" id="d-waktu" name="preferred_at" placeholder="mis. 2026-12-01 10:00"></div><button class="btn btn-primary" type="submit">Minta Demo</button></form></div></section>"#;
    html_ok(page_shell(
        "Minta Demo",
        "Jadwalkan sesi demo LegalRust untuk kantor hukum Anda.",
        "/demo",
        "",
        body,
    ))
}

// ----- artikel / faq -----
async fn artikel_list(State(s): State<FullState>) -> impl IntoResponse {
    let arts = s
        .stores
        .mkt
        .mkt_article_list(true)
        .await
        .unwrap_or_default();
    let mut items = String::new();
    for a in &arts {
        items.push_str(&format!(
            r#"<div class="fcard"><h3><a href="/artikel/{}">{}</a></h3></div>"#,
            esc(&a.slug),
            esc(&a.title),
        ));
    }
    if items.is_empty() {
        items = "<p class=\"text-muted\">Belum ada artikel yang diterbitkan. Pantau halaman ini untuk panduan praktik hukum.</p>".to_string();
    }
    let body = format!(
        r#"<section class="feat"><span class="badge-soft tag">ARTIKEL</span><h1>Artikel dan Panduan</h1><p class="sub">Tulisan praktis seputar praktik hukum digital.</p></section><div class="fgrid">{items}</div>"#,
        items = items
    );
    html_ok(page_shell(
        "Artikel dan Panduan",
        "Artikel dan panduan LegalRust seputar praktik hukum digital.",
        "/artikel",
        "Artikel",
        &body,
    ))
}

async fn artikel_detail(State(s): State<FullState>, Path(slug): Path<String>) -> impl IntoResponse {
    let found = s
        .stores
        .mkt
        .mkt_article_get(&slug)
        .await
        .ok()
        .filter(|a| a.published);
    match found {
        Some(a) => {
            let mut paras = String::new();
            for line in a.body_md.lines() {
                let t = line.trim().trim_start_matches('#').trim();
                if !t.is_empty() {
                    paras.push_str(&format!("<p>{}</p>", esc(t)));
                }
            }
            let body = format!(
                r#"<section class="feat"><span class="badge-soft tag">ARTIKEL</span><h1>{t}</h1></section><section class="card"><div class="card-body">{p}<p><a href="/artikel">Kembali ke daftar artikel</a></p></div></section>"#,
                t = esc(&a.title),
                p = paras,
            );
            html_ok(page_shell(
                &a.title,
                &format!("Artikel LegalRust: {}", a.title),
                &format!("/artikel/{}", a.slug),
                "Artikel",
                &body,
            ))
        }
        None => html_status(
            StatusCode::NOT_FOUND,
            page_shell(
                "Artikel tidak ditemukan",
                "Artikel yang Anda cari tidak tersedia.",
                &format!("/artikel/{slug}"),
                "Artikel",
                r#"<section class="feat"><h1>Artikel Tidak Ditemukan</h1><p class="sub">Slug tidak cocok dengan artikel yang diterbitkan. <a href="/artikel">Kembali ke daftar artikel</a></p></section>"#,
            ),
        ),
    }
}

async fn faq_page(State(s): State<FullState>) -> impl IntoResponse {
    let faqs = s.stores.mkt.mkt_faq_list().await.unwrap_or_default();
    let mut items = String::new();
    for f in &faqs {
        items.push_str(&format!(
            "<details><summary>{}</summary><p>{}</p></details>",
            esc(&f.question),
            esc(&f.answer),
        ));
    }
    if items.is_empty() {
        items = "<p class=\"text-muted\">Belum ada FAQ yang diterbitkan.</p>".to_string();
    }
    let body = format!(
        r#"<section class="feat"><span class="badge-soft tag">FAQ</span><h1>Pertanyaan Umum: Fitur, Keamanan, dan Status</h1><p class="sub">Jawaban jujur seputar scope live dan integrasi 501.</p></section><section class="card"><div class="card-body">{items}</div></section>"#,
        items = items
    );
    html_ok(page_shell(
        "Pertanyaan Umum",
        "FAQ LegalRust: fitur, keamanan, dan status integrasi 501.",
        "/faq",
        "",
        &body,
    ))
}

// ----- legal -----
async fn privasi() -> impl IntoResponse {
    let body = r#"<section class="feat"><span class="badge-soft tag">LEGAL</span><h1>Kebijakan Privasi</h1><p class="sub">Terakhir diperbarui: November 2026.</p></section><section class="card"><div class="card-body"><h2 class="card-title">Data yang kami kumpulkan</h2><p>Kami mengumpulkan data akun (nama, email, organisasi), data operasional yang Anda masukkan (klien, perkara, dokumen), serta pesan kontak dan permintaan demo. Data tersegregasi per organisasi dan tidak pernah dicampur antar tenant.</p><h2 class="card-title">Penggunaan dan keamanan</h2><p>Data digunakan untuk mengoperasikan layanan. Password di-hash Argon2, sesi JWT dapat dicabut, dan akses API dijaga RBAC per endpoint. Kami tidak menjual data Anda dan tidak menampilkan klaim produksi yang belum terverifikasi.</p></div></section>"#;
    html_ok(page_shell(
        "Kebijakan Privasi",
        "Kebijakan privasi LegalRust: pengumpulan, penggunaan, dan keamanan data.",
        "/kebijakan-privasi",
        "",
        body,
    ))
}

async fn syarat() -> impl IntoResponse {
    let body = r#"<section class="feat"><span class="badge-soft tag">LEGAL</span><h1>Syarat dan Ketentuan</h1><p class="sub">Terakhir diperbarui: November 2026.</p></section><section class="card"><div class="card-body"><h2 class="card-title">Layanan</h2><p>LegalRust menyediakan platform manajemen hukum dalam tahap uji internal, bukan production-ready. Fitur berlabel BUTUH KONFIGURASI mengembalikan 501 hingga dependensi eksternalnya dikonfigurasi.</p><h2 class="card-title">Tanggung jawab pengguna</h2><p>Anda bertanggung jawab atas akurasi data yang dimasukkan dan atas kepatuhan terhadap peraturan jabatan (notaris, PPAT, advokat) yang berlaku. Tanda tangan elektronik tunduk pada ketersediaan provider PSrE.</p></div></section>"#;
    html_ok(page_shell(
        "Syarat dan Ketentuan",
        "Syarat dan ketentuan penggunaan platform LegalRust.",
        "/syarat-ketentuan",
        "",
        body,
    ))
}

// ----- sitemap / robots -----
const PUBLIC_ROUTES: &[(&str, &str, &str)] = &[
    ("/", "weekly", "1.0"),
    ("/fitur", "monthly", "0.8"),
    ("/fitur/manajemen-perkara", "monthly", "0.8"),
    ("/fitur/manajemen-dokumen", "monthly", "0.8"),
    ("/fitur/ocr", "monthly", "0.8"),
    ("/fitur/ai-legal-assistant", "monthly", "0.8"),
    ("/fitur/generasi-akta", "monthly", "0.8"),
    ("/fitur/tanda-tangan", "monthly", "0.8"),
    ("/solusi", "monthly", "0.8"),
    ("/solusi/notaris", "monthly", "0.8"),
    ("/solusi/ppat", "monthly", "0.8"),
    ("/solusi/firma-hukum", "monthly", "0.8"),
    ("/solusi/legal-corporate", "monthly", "0.8"),
    ("/harga", "monthly", "0.9"),
    ("/tentang", "monthly", "0.6"),
    ("/kontak", "monthly", "0.6"),
    ("/demo", "monthly", "0.7"),
    ("/artikel", "weekly", "0.7"),
    ("/faq", "monthly", "0.7"),
    ("/kebijakan-privasi", "yearly", "0.3"),
    ("/syarat-ketentuan", "yearly", "0.3"),
    ("/login", "yearly", "0.2"),
];

async fn sitemap() -> impl IntoResponse {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for (p, freq, prio) in PUBLIC_ROUTES {
        xml.push_str(&format!(
            "<url><loc>{SITE_BASE}{p}</loc><changefreq>{freq}</changefreq><priority>{prio}</priority></url>"
        ));
    }
    xml.push_str("</urlset>");
    (
        StatusCode::OK,
        [("content-type", "text/xml; charset=utf-8")],
        xml,
    )
}

async fn robots() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        format!(
            "User-agent: *\nAllow: /\nDisallow: /app\nDisallow: /api/\nSitemap: {SITE_BASE}/sitemap.xml\n"
        ),
    )
}
// ---------- end public pages ----------

// ---------- marketing: public site + admin pipeline ----------
// Public writes are rate-limited per endpoint via the shared limiter.
// Admin endpoints require Permission::Approve (the domain enum has no
// separate Manage permission) and record mutations via audit().
fn lead_json(l: &MarketingLeadRow) -> Value {
    json!({
        "id": l.id, "org": l.org, "name": l.name, "email": l.email,
        "phone": l.phone, "company": l.company, "source": l.source,
        "status": l.status, "assigned_to": l.assigned_to,
    })
}
fn plan_json(p: &MarketingPlanRow) -> Value {
    let features: Value = serde_json::from_str(&p.features_json)
        .unwrap_or_else(|_| Value::String(p.features_json.clone()));
    json!({
        "id": p.id, "code": p.code, "name": p.name,
        "monthly_minor": p.monthly_minor, "annual_minor": p.annual_minor,
        "features": features, "active": p.active,
    })
}
fn faq_json(f: &MarketingFaqRow) -> Value {
    json!({"id": f.id, "slug": f.slug, "question": f.question, "answer": f.answer, "sort": f.sort})
}
fn article_json(a: &MarketingArticleRow) -> Value {
    json!({"id": a.id, "slug": a.slug, "title": a.title, "body_md": a.body_md, "published": a.published})
}
async fn admin_auth(headers: &HeaderMap, s: &FullState) -> Result<AuthUser, AppError> {
    let u = auth_user(headers, s).await?;
    authorize(&u, Permission::Approve)?;
    Ok(u)
}
fn admin_lead_scope(u: &AuthUser, l: &MarketingLeadRow) -> Result<(), AppError> {
    match &l.org {
        None => Ok(()),
        Some(o) if o == &u.claims.org => Ok(()),
        _ => Err(AppError::NotFound("lead".into())),
    }
}

// ----- public -----
#[derive(Debug, Deserialize, Validate)]
struct ContactReq {
    #[validate(length(min = 2))]
    name: String,
    #[validate(email)]
    email: String,
    #[validate(length(min = 2))]
    subject: String,
    #[validate(length(min = 5))]
    message: String,
}
fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => match (hex_val(b[i + 1]), hex_val(b[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h << 4 | l);
                    i += 3;
                }
                _ => {
                    out.push(b'%');
                    i += 1;
                }
            },
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
/// Accept JSON (API clients) or `application/x-www-form-urlencoded`
/// (plain HTML forms on /kontak and /demo — no JS required).
fn decode_body<T: serde::de::DeserializeOwned>(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<T, AppError> {
    let ct = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if ct.contains("json") {
        serde_json::from_slice(body).map_err(|e| AppError::Validation(e.to_string()))
    } else {
        let mut obj = serde_json::Map::new();
        for pair in String::from_utf8_lossy(body).split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => continue,
            };
            obj.entry(pct_decode(k))
                .or_insert(Value::String(pct_decode(v)));
        }
        serde_json::from_value(Value::Object(obj)).map_err(|e| AppError::Validation(e.to_string()))
    }
}
async fn public_contact(
    State(s): State<FullState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("mkt:contact") {
        return Err(AppError::RateLimited);
    }
    let b: ContactReq = decode_body(&headers, &body)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let c = s
        .stores
        .mkt
        .mkt_contact_create(&b.name, &b.email, &b.subject, &b.message)
        .await
        .map_err(map_store)?;
    let lead = s
        .stores
        .mkt
        .mkt_lead_create(None, &b.name, &b.email, None, None, "contact")
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"id": c.id, "lead_id": lead.id, "status": c.status}),
    ))
}

#[derive(Debug, Deserialize, Validate)]
struct DemoReq {
    #[validate(length(min = 2))]
    name: String,
    #[validate(email)]
    email: String,
    company: Option<String>,
    preferred_at: Option<String>,
}
async fn public_demo(
    State(s): State<FullState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("mkt:demo") {
        return Err(AppError::RateLimited);
    }
    let b: DemoReq = decode_body(&headers, &body)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let company = b.company.clone().unwrap_or_default();
    let lead = s
        .stores
        .mkt
        .mkt_lead_create(
            None,
            &b.name,
            &b.email,
            None,
            Some(company.as_str()).filter(|c| !c.trim().is_empty()),
            "demo",
        )
        .await
        .map_err(map_store)?;
    let d = s
        .stores
        .mkt
        .mkt_demo_create(
            Some(lead.id),
            &b.name,
            &b.email,
            &company,
            b.preferred_at.as_deref(),
        )
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"id": d.id, "lead_id": lead.id, "status": d.status}),
    ))
}

#[derive(Debug, Deserialize, Validate)]
struct SalesReq {
    #[validate(length(min = 2))]
    name: String,
    #[validate(email)]
    email: String,
    company: Option<String>,
    #[validate(length(min = 5))]
    message: String,
}
async fn public_sales(
    State(s): State<FullState>,
    Json(b): Json<SalesReq>,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("mkt:sales") {
        return Err(AppError::RateLimited);
    }
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let lead = s
        .stores
        .mkt
        .mkt_lead_create(
            None,
            &b.name,
            &b.email,
            None,
            b.company.as_deref().filter(|c| !c.trim().is_empty()),
            "sales",
        )
        .await
        .map_err(map_store)?;
    s.stores
        .mkt
        .mkt_note_add(
            lead.id,
            "website",
            &format!(
                "sales inquiry from {} ({}): {}",
                b.name,
                b.company.as_deref().unwrap_or("-"),
                b.message
            ),
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": lead.id, "status": lead.status})))
}

#[derive(Debug, Deserialize, Validate)]
struct SubscribeReq {
    #[validate(email)]
    email: String,
}
async fn public_subscribe(
    State(s): State<FullState>,
    Json(b): Json<SubscribeReq>,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("mkt:subscribe") {
        return Err(AppError::RateLimited);
    }
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let r = s
        .stores
        .mkt
        .mkt_subscribe(&b.email)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"subscribed": true, "email": r.email})))
}

async fn public_pricing(State(s): State<FullState>) -> Result<Json<Value>, AppError> {
    let v = s.stores.mkt.mkt_pricing_list().await.map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .filter(|p| p.active)
        .map(plan_json)
        .collect::<Vec<_>>())))
}

async fn public_features() -> Json<Value> {
    Json(json!([
        {"name": "matters", "description": "Clients, matters and a guarded task lifecycle", "available": true},
        {"name": "documents", "description": "Versioned uploads pinned by SHA-256 with size validation", "available": true},
        {"name": "ocr", "description": "ID and certificate OCR jobs with human review; processing runs only when an engine is configured", "available": true},
        {"name": "deeds", "description": "Deed templates with variable substitution and a guarded deed state machine", "available": true},
        {"name": "billing", "description": "Invoices with an overpay guard and manual payment recording", "available": true},
        {"name": "trust", "description": "Client trust ledger with a double-entry transfer guard", "available": true},
        {"name": "rag", "description": "Full-text index and search over uploaded text documents", "available": true},
        {"name": "ai-usage", "description": "AI chat usage is metered; generation is provider-gated (501 until a provider is configured)", "available": false},
        {"name": "reports", "description": "Matter CSV exports plus a queryable audit log", "available": true},
        {"name": "sharing", "description": "Expiry-scoped document share links plus an SSE event stream", "available": true},
    ]))
}

async fn public_articles(
    State(s): State<FullState>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    // Public-safe default: published only. `published=0` exposes drafts.
    let published_only = q.get("published").map(|v| v != "0").unwrap_or(true);
    let v = s
        .stores
        .mkt
        .mkt_article_list(published_only)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v.iter().map(article_json).collect::<Vec<_>>())))
}
async fn public_article(
    State(s): State<FullState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, AppError> {
    let a = s
        .stores
        .mkt
        .mkt_article_get(&slug)
        .await
        .map_err(map_store)?;
    if !a.published {
        return Err(AppError::NotFound("article".into()));
    }
    Ok(Json(article_json(&a)))
}

async fn public_faqs(State(s): State<FullState>) -> Result<Json<Value>, AppError> {
    let v = s.stores.mkt.mkt_faq_list().await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(faq_json).collect::<Vec<_>>())))
}

// ----- admin -----
#[derive(Debug, Deserialize)]
struct LeadQuery {
    status: Option<String>,
    limit: Option<i64>,
}
async fn admin_lead_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<LeadQuery>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let mut v = s.stores.mkt.mkt_lead_list(None).await.map_err(map_store)?;
    let mut own = s
        .stores
        .mkt
        .mkt_lead_list(Some(&u.claims.org))
        .await
        .map_err(map_store)?;
    v.append(&mut own);
    v.sort_by_key(|l| l.id);
    if let Some(st) = q.status.as_deref() {
        v.retain(|l| l.status == st);
    }
    let lim = q.limit.unwrap_or(100).clamp(1, 500) as usize;
    v.truncate(lim);
    Ok(Json(json!(v.iter().map(lead_json).collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct AdminLeadReq {
    #[validate(length(min = 2))]
    name: String,
    #[validate(email)]
    email: String,
    phone: Option<String>,
    company: Option<String>,
    source: Option<String>,
}
async fn admin_lead_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<AdminLeadReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let l = s
        .stores
        .mkt
        .mkt_lead_create(
            Some(&u.claims.org),
            &b.name,
            &b.email,
            b.phone.as_deref(),
            b.company.as_deref(),
            b.source.as_deref().unwrap_or("manual"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "marketing.lead.created".into(),
            entity: "marketing_lead".into(),
            entity_id: l.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(lead_json(&l)))
}

async fn admin_lead_get(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let l = s.stores.mkt.mkt_lead_get(id).await.map_err(map_store)?;
    admin_lead_scope(&u, &l)?;
    Ok(Json(lead_json(&l)))
}

#[derive(Debug, Deserialize)]
struct LeadStatusReq {
    status: String,
}
async fn admin_lead_put(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<LeadStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let cur = s.stores.mkt.mkt_lead_get(id).await.map_err(map_store)?;
    admin_lead_scope(&u, &cur)?;
    let to = MarketingLeadStatus::parse(&b.status)
        .ok_or_else(|| AppError::Validation("status".into()))?;
    let _ = to;
    let l = s
        .stores
        .mkt
        .mkt_lead_set_status(id, &b.status)
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "marketing.lead.status".into(),
            entity: "marketing_lead".into(),
            entity_id: l.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(lead_json(&l)))
}

async fn admin_lead_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let cur = s.stores.mkt.mkt_lead_get(id).await.map_err(map_store)?;
    admin_lead_scope(&u, &cur)?;
    // Soft delete: no deleted flag in the schema, so park as `lost` + note.
    // `lost` is idempotent here; terminal `won` stays 409 via the guard.
    if cur.status != "lost" {
        s.stores
            .mkt
            .mkt_lead_set_status(id, "lost")
            .await
            .map_err(map_store)?;
    }
    s.stores
        .mkt
        .mkt_note_add(id, &u.claims.sub, "soft-deleted: status -> lost")
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "marketing.lead.deleted".into(),
            entity: "marketing_lead".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id, "status": "lost", "deleted": true})))
}

#[derive(Debug, Deserialize, Validate)]
struct NoteReq {
    #[validate(length(min = 1))]
    note: String,
}
async fn admin_lead_note(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<NoteReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let cur = s.stores.mkt.mkt_lead_get(id).await.map_err(map_store)?;
    admin_lead_scope(&u, &cur)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let n = s
        .stores
        .mkt
        .mkt_note_add(id, &u.claims.sub, &b.note)
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"id": n.id, "lead_id": n.lead_id, "author": n.author, "note": n.note}),
    ))
}

#[derive(Debug, Deserialize)]
struct AssignReq {
    assignee: Option<String>,
}
async fn admin_lead_assign(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<AssignReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let cur = s.stores.mkt.mkt_lead_get(id).await.map_err(map_store)?;
    admin_lead_scope(&u, &cur)?;
    let l = s
        .stores
        .mkt
        .mkt_lead_assign(id, b.assignee.as_deref())
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "marketing.lead.assigned".into(),
            entity: "marketing_lead".into(),
            entity_id: l.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(lead_json(&l)))
}

#[derive(Debug, Deserialize, Validate)]
struct FollowUpReq {
    #[validate(length(min = 1))]
    due_at: String,
    #[validate(length(min = 1))]
    note: String,
}
async fn admin_lead_followup_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<FollowUpReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let cur = s.stores.mkt.mkt_lead_get(id).await.map_err(map_store)?;
    admin_lead_scope(&u, &cur)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let f = s
        .stores
        .mkt
        .mkt_followup_add(id, &b.due_at, &b.note)
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"id": f.id, "lead_id": f.lead_id, "due_at": f.due_at, "done": f.done, "note": f.note}),
    ))
}
async fn admin_lead_followup_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    let cur = s.stores.mkt.mkt_lead_get(id).await.map_err(map_store)?;
    admin_lead_scope(&u, &cur)?;
    let v = s
        .stores
        .mkt
        .mkt_followup_list(id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|f| json!({"id": f.id, "lead_id": f.lead_id, "due_at": f.due_at, "done": f.done, "note": f.note}))
        .collect::<Vec<_>>())))
}

async fn admin_contact_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let _u = admin_auth(&headers, &s).await?;
    let v = s.stores.mkt.mkt_contact_list().await.map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|c| json!({"id": c.id, "name": c.name, "email": c.email, "subject": c.subject, "message": c.message, "status": c.status}))
        .collect::<Vec<_>>())))
}
async fn admin_demo_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let _u = admin_auth(&headers, &s).await?;
    let v = s.stores.mkt.mkt_demo_list().await.map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|d| json!({"id": d.id, "lead_id": d.lead_id, "name": d.name, "email": d.email, "company": d.company, "preferred_at": d.preferred_at, "status": d.status}))
        .collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct PricingReq {
    #[validate(length(min = 1))]
    code: String,
    #[validate(length(min = 1))]
    name: String,
    #[validate(range(min = 0))]
    monthly_minor: i64,
    #[validate(range(min = 0))]
    annual_minor: i64,
    features: Option<Vec<String>>,
    active: Option<bool>,
}
async fn admin_pricing_upsert(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PricingReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let features_json = serde_json::to_string(&b.features.unwrap_or_default())
        .map_err(|_| AppError::Validation("features".into()))?;
    let p = s
        .stores
        .mkt
        .mkt_pricing_upsert(
            &b.code,
            &b.name,
            b.monthly_minor,
            b.annual_minor,
            &features_json,
            b.active.unwrap_or(true),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "marketing.pricing.upserted".into(),
            entity: "marketing_plan".into(),
            entity_id: p.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(plan_json(&p)))
}

#[derive(Debug, Deserialize, Validate)]
struct FaqReq {
    #[validate(length(min = 1))]
    slug: String,
    #[validate(length(min = 1))]
    question: String,
    #[validate(length(min = 1))]
    answer: String,
    sort: Option<i64>,
}
async fn admin_faq_upsert(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<FaqReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let f = s
        .stores
        .mkt
        .mkt_faq_upsert(&b.slug, &b.question, &b.answer, b.sort.unwrap_or(0))
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "marketing.faq.upserted".into(),
            entity: "marketing_faq".into(),
            entity_id: f.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(faq_json(&f)))
}

#[derive(Debug, Deserialize, Validate)]
struct ArticleReq {
    #[validate(length(min = 1))]
    slug: String,
    #[validate(length(min = 1))]
    title: String,
    #[validate(length(min = 1))]
    body_md: String,
    published: Option<bool>,
}
async fn admin_article_upsert(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ArticleReq>,
) -> Result<Json<Value>, AppError> {
    let u = admin_auth(&headers, &s).await?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let a = s
        .stores
        .mkt
        .mkt_article_upsert(&b.slug, &b.title, &b.body_md, b.published.unwrap_or(false))
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "marketing.article.upserted".into(),
            entity: "marketing_article".into(),
            entity_id: a.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(article_json(&a)))
}
// ---------- end marketing ----------

pub fn build_router(state: FullState) -> Router {
    let request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/", get(landing))
        .route("/login", get(login_page))
        .route("/app", get(app_page))
        .route("/fitur", get(fitur_index))
        .route("/fitur/manajemen-perkara", get(fitur_perkara))
        .route("/fitur/manajemen-dokumen", get(fitur_dokumen))
        .route("/fitur/ocr", get(fitur_ocr))
        .route("/fitur/ai-legal-assistant", get(fitur_ai))
        .route("/fitur/generasi-akta", get(fitur_akta))
        .route("/fitur/tanda-tangan", get(fitur_ttd))
        .route("/solusi", get(solusi_index))
        .route("/solusi/notaris", get(solusi_notaris))
        .route("/solusi/ppat", get(solusi_ppat))
        .route("/solusi/firma-hukum", get(solusi_firma))
        .route("/solusi/legal-corporate", get(solusi_corporate))
        .route("/harga", get(harga))
        .route("/tentang", get(tentang))
        .route("/kontak", get(kontak))
        .route("/demo", get(demo))
        .route("/artikel", get(artikel_list))
        .route("/artikel/{slug}", get(artikel_detail))
        .route("/faq", get(faq_page))
        .route("/kebijakan-privasi", get(privasi))
        .route("/syarat-ketentuan", get(syarat))
        .route("/sitemap.xml", get(sitemap))
        .route("/robots.txt", get(robots))
        .route("/static/{*path}", get(static_img))
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
        .route("/api/v1/public/contact", post(public_contact))
        .route("/api/v1/public/demo-requests", post(public_demo))
        .route("/api/v1/public/sales-inquiries", post(public_sales))
        .route(
            "/api/v1/public/newsletter/subscribe",
            post(public_subscribe),
        )
        .route("/api/v1/public/pricing", get(public_pricing))
        .route("/api/v1/public/features", get(public_features))
        .route("/api/v1/public/articles", get(public_articles))
        .route("/api/v1/public/articles/{slug}", get(public_article))
        .route("/api/v1/public/faqs", get(public_faqs))
        .route(
            "/api/v1/admin/leads",
            get(admin_lead_list).post(admin_lead_create),
        )
        .route(
            "/api/v1/admin/leads/{id}",
            get(admin_lead_get)
                .put(admin_lead_put)
                .delete(admin_lead_delete),
        )
        .route("/api/v1/admin/leads/{id}/notes", post(admin_lead_note))
        .route("/api/v1/admin/leads/{id}/assign", post(admin_lead_assign))
        .route(
            "/api/v1/admin/leads/{id}/follow-ups",
            post(admin_lead_followup_add).get(admin_lead_followup_list),
        )
        .route("/api/v1/admin/contacts", get(admin_contact_list))
        .route("/api/v1/admin/demo-requests", get(admin_demo_list))
        .route("/api/v1/admin/pricing", post(admin_pricing_upsert))
        .route("/api/v1/admin/faqs", post(admin_faq_upsert))
        .route("/api/v1/admin/articles", post(admin_article_upsert))
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
        o.clone(),
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
