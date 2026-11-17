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
    let path = std::path::PathBuf::from("apps/server/static").join(&name);
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| AppError::NotFound("asset".into()))?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, ctype)],
        bytes,
    ))
}
async fn not_found() -> AppError {
    AppError::NotFound("route".into())
}

// ---------- public pages (landing + login + app shell) ----------
const PAGE_CSS: &str = "@font-face{font-family:'Plus Jakarta Sans';src:url('/static/fonts/PlusJakartaSans.ttf') format('truetype');font-weight:100 900;font-display:swap}:root{--paper:#FAF7F1;--ink:#1C1917;--muted:#78716C;--gold:#B45309;--emerald:#0C4A3B;--line:rgba(28,25,23,.1);--card:#FFFFFF}\
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

const LANDING_HTML: &str = r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>LegalRust — Smart Legal Solution</title><meta name="description" content="Platform manajemen hukum terpadu untuk notaris, PPAT, firma hukum, dan profesional legal."><link rel="stylesheet" href="/static/vendor/tabler.min.css"><link rel="stylesheet" href="/static/vendor/tabler-fonts/tabler-icons.min.css"><style>
body{font-family:'Plus Jakarta Sans',system-ui,sans-serif}
@font-face{font-family:'Plus Jakarta Sans';src:url('/static/fonts/PlusJakartaSans.ttf') format('truetype');font-weight:100 900;font-display:swap}
.hero{background:linear-gradient(100deg,rgba(247,250,255,.96) 30%,rgba(247,250,255,.55) 60%,rgba(247,250,255,.15)),url('/static/hero-office.png') center/cover}
.hero h1{letter-spacing:-.02em}
.mock-shadow{box-shadow:0 40px 80px -32px rgba(15,40,120,.35)}
.cta-bg{background:linear-gradient(100deg,rgba(11,43,168,.88),rgba(29,91,216,.55)),url('/static/cta-building.png') center/cover}
.rv{opacity:0;transform:translateY(26px);transition:opacity .8s cubic-bezier(.32,.72,0,1),transform .8s cubic-bezier(.32,.72,0,1)}
.rv.in{opacity:1;transform:none}
</style></head><body>
<div class="container-xl"><div class="d-flex align-items-center gap-3 py-3"><a href="/" class="d-flex align-items-center gap-2 text-decoration-none"><span style="font-size:26px">⚖️</span><span><strong>LegalRust</strong><span class="d-block text-muted" style="font-size:11px">Smart Legal Solution</span></span></a><nav class="d-none d-md-flex gap-3 mx-auto"><a class="btn btn-primary btn-sm" href="/">Beranda</a><a class="btn btn-ghost-secondary btn-sm" href="#fitur">Fitur</a><a class="btn btn-ghost-secondary btn-sm" href="/app">Solusi</a><a class="btn btn-ghost-secondary btn-sm" href="#harga">Harga</a><a class="btn btn-ghost-secondary btn-sm" href="#fitur">Tentang</a><a class="btn btn-ghost-secondary btn-sm" href="/login">Kontak</a></nav><div class="ms-auto d-flex gap-2"><a href="/login" class="btn btn-outline-primary">Masuk</a><a href="/login" class="btn btn-primary">Coba Gratis <i class="ti ti-arrow-right"></i></a></div></div></div>
<section class="hero"><div class="container-xl"><div class="row align-items-center py-5">
<div class="col-lg-6 py-4"><span class="badge bg-white text-primary border rv">🚀 Solusi LegalTech Terdepan di Indonesia</span>
<h1 class="display-5 fw-bolder mt-3 rv">Kelola Perkara Hukum Lebih Mudah, <span class="text-primary">Profesional dan Terpercaya.</span></h1>
<p class="text-muted fs-4 mt-3 rv">Platform manajemen hukum terpadu untuk notaris, PPAT, firma hukum, dan profesional legal. Kelola perkara, dokumen, akta, OCR, dan proses administrasi dalam satu platform.</p>
<div class="d-flex gap-2 mt-4 rv"><a href="/login" class="btn btn-primary btn-lg">▶ Mulai Gratis</a><a href="/app" class="btn btn-outline-primary btn-lg">Lihat Demo</a></div>
<div class="d-flex gap-4 mt-4 text-muted flex-wrap rv"><span><i class="ti ti-check text-primary"></i> Platform Terintegrasi</span><span><i class="ti ti-check text-primary"></i> Workflow Lebih Efisien</span><span><i class="ti ti-check text-primary"></i> Keamanan Berlapis</span></div></div>
<div class="col-lg-6 py-4"><div class="card mock-shadow rv"><div class="card-header"><span class="badge bg-blue-lt">Dasbor live Anda — ilustrasi</span></div>
<div class="card-body"><div class="row g-2">
<div class="col-6 col-md-3"><div class="card"><div class="card-body p-2 text-center"><div class="text-muted small">Total Perkara</div><div class="h2 m-0">128</div><div class="text-success small">↑ 12.5%</div></div></div></div>
<div class="col-6 col-md-3"><div class="card"><div class="card-body p-2 text-center"><div class="text-muted small">Dokumen</div><div class="h2 m-0">842</div><div class="text-success small">↑ 18.7%</div></div></div></div>
<div class="col-6 col-md-3"><div class="card"><div class="card-body p-2 text-center"><div class="text-muted small">Klien Aktif</div><div class="h2 m-0">256</div><div class="text-success small">↑ 8.2%</div></div></div></div>
<div class="col-6 col-md-3"><div class="card"><div class="card-body p-2 text-center"><div class="text-muted small">Pendapatan</div><div class="h2 m-0">Rp 248 jt</div><div class="text-success small">↑ 14.6%</div></div></div></div>
</div><div class="table-responsive mt-3"><table class="table table-sm"><thead><tr><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody>
<tr><td><strong>M-1047</strong></td><td>Akta Jual Beli Tanah</td><td><span class="badge bg-green-lt">review</span></td></tr>
<tr><td><strong>M-1046</strong></td><td>Pendirian PT Maju Bersama</td><td><span class="badge bg-yellow-lt">draft</span></td></tr>
<tr><td><strong>M-1045</strong></td><td>Perjanjian Kredit Bank</td><td><span class="badge bg-blue-lt">approved</span></td></tr>
</tbody></table></div></div></div></div>
</div></div></section>
<div class="container-xl"><div class="card mt-3 mb-2 rv"><div class="card-body d-flex align-items-center gap-3 flex-wrap py-3">
<div><strong>Dipercaya oleh</strong><div class="text-muted">Ribuan Profesional</div></div>
<span class="badge bg-azure-lt">NOTARIS</span><span class="badge bg-azure-lt">PPAT</span><span class="badge bg-azure-lt">FIRMA HUKUM</span><span class="badge bg-azure-lt">KORPORAT</span><span class="badge bg-azure-lt">ADVOKAT</span></div></div>
<div class="text-center mt-5" id="fitur"><span class="badge bg-blue-lt rv">FITUR UNGGULAN</span><h2 class="mt-2 rv">Semua yang Anda Butuhkan dalam Satu Platform</h2><p class="text-muted rv">Dari manajemen perkara hingga laporan analitik, semua terintegrasi dengan sempurna.</p></div>
<div class="row g-3 mt-2">
<div class="col-md-4"><div class="card h-100 rv"><div class="card-body"><i class="ti ti-scale fs-2 text-primary"></i><h3 class="mt-2">Manajemen Perkara</h3><p class="text-muted">Klien, tugas, jadwal, dan status perkara dalam satu alur tertib dan teraudit. <a href="/app">Buka dasbor →</a></p></div></div></div>
<div class="col-md-4"><div class="card h-100 rv"><div class="card-body"><i class="ti ti-file-search fs-2 text-primary"></i><h3 class="mt-2">OCR &amp; Verifikasi</h3><p class="text-muted">Ekstraksi KTP/KK/NPWP/sertifikat dengan skor kepercayaan + review manusia. <a href="/app">Lihat alur →</a></p></div></div></div>
<div class="col-md-4"><div class="card h-100 rv"><div class="card-body"><i class="ti ti-writing fs-2 text-primary"></i><h3 class="mt-2">Generasi Akta</h3><p class="text-muted">Template render variabel dengan guard transisi draft → review → approved → signed. <a href="/app">Lihat alur →</a></p></div></div></div>
<div class="col-md-4"><div class="card h-100 rv"><div class="card-body"><i class="ti ti-wallet fs-2 text-primary"></i><h3 class="mt-2">Keuangan &amp; Trust</h3><p class="text-muted">Invoice anti-overpay dan rekening dana klien anti-saldo-negatif. <a href="/app">Lihat alur →</a></p></div></div></div>
<div class="col-md-4"><div class="card h-100 rv"><div class="card-body"><i class="ti ti-chart-bar fs-2 text-primary"></i><h3 class="mt-2">Laporan &amp; Analitik</h3><p class="text-muted">Ekspor CSV perkara dan trust ledger siap olah. <a href="/api/v1/version">Cek API →</a></p></div></div></div>
<div class="col-md-4"><div class="card h-100 rv"><div class="card-body"><i class="ti ti-shield-check fs-2 text-primary"></i><h3 class="mt-2">Keamanan Berlapis</h3><p class="text-muted">Argon2, JWT + lockout, RBAC per endpoint, jejak audit. <a href="/api/v1/version">Cek API →</a></p></div></div></div>
</div>
<div class="card cta-bg text-white mt-5 rv" id="harga"><div class="card-body p-5"><div class="row align-items-center">
<div class="col-lg-8"><h2>Bangun Praktik Hukum yang Lebih Modern.</h2><p class="opacity-75">Mulai gratis — harga paket menyusul dari backend/CMS. <a href="/login" class="text-white text-decoration-underline">Mulai Perjalanan Anda dengan LegalRust →</a></p></div>
<div class="col-lg-4"><a href="/login" class="btn btn-light btn-lg w-100">Mulai Sekarang</a></div></div></div></div>
<footer class="d-flex justify-content-between flex-wrap gap-2 py-4 text-muted"><span>© 2026 LegalRust — Smart Legal Solution</span><span><a href="/kebijakan-privasi">Privasi</a> · <a href="/syarat-ketentuan">Syarat</a> · <a href="/faq">FAQ</a> · <a href="/healthz">Status</a></span></footer></div>
<script src="/static/vendor/tabler.min.js"></script>
<script>const io=new IntersectionObserver(e=>e.forEach(x=>x.isIntersecting&&x.target.classList.add('in')),{threshold:.1});document.querySelectorAll('.rv').forEach(el=>io.observe(el));</script>
</body></html>"##;

const LOGIN_HTML: &str = r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Masuk — LegalRust</title><link rel="stylesheet" href="/static/vendor/tabler.min.css"><link rel="stylesheet" href="/static/vendor/tabler-fonts/tabler-icons.min.css"><style>body{font-family:'Plus Jakarta Sans',system-ui,sans-serif}@font-face{font-family:'Plus Jakarta Sans';src:url('/static/fonts/PlusJakartaSans.ttf') format('truetype');font-weight:100 900;font-display:swap}</style></head><body class="d-flex flex-column">
<div class="page page-center flex-fill"><div class="container container-tight py-4">
<div class="text-center mb-4"><a href="/">⚖️ <strong>LegalRust</strong></a><div class="text-muted">Smart Legal Solution</div></div>
<div class="card card-md"><div class="card-body">
<h2 class="h2 text-center mb-1">Masuk</h2><p class="text-muted text-center">Sesi JWT + proteksi brute-force 5x percobaan</p>
<div class="btn-group w-100 mb-3"><button class="btn btn-primary" id="t1" onclick="tab(1)">Masuk</button><button class="btn" id="t2" onclick="tab(2)">Daftar organisasi</button></div>
<div id="f_reg" style="display:none"><div class="mb-2"><label class="form-label">Nama organisasi</label><input class="form-control" id="org"></div><div class="mb-2"><label class="form-label">Nama Anda</label><input class="form-control" id="nm"></div></div>
<div class="mb-2"><label class="form-label">Email</label><input class="form-control" id="email" type="email" autocomplete="username"></div>
<div class="mb-2"><label class="form-label">Kata sandi <span class="form-label-description"><a href="/faq">Lupa?</a></span></label><input class="form-control" id="pwd" type="password" autocomplete="current-password"></div>
<div class="text-danger mb-2" id="err"></div>
<button class="btn btn-primary w-100" id="go" onclick="go()">Masuk aplikasi</button>
</div></div><div class="text-center text-muted mt-3">Kembali ke <a href="/">Beranda</a></div></div></div>
<script>let mode=1;function tab(m){mode=m;document.getElementById('t1').className=m===1?'btn btn-primary':'btn';document.getElementById('t2').className=m===2?'btn btn-primary':'btn';document.getElementById('f_reg').style.display=m===2?'block':'none';document.getElementById('go').textContent=m===1?'Masuk aplikasi':'Daftar & masuk';}
async function go(){const e=document.getElementById('err');e.textContent='';document.getElementById('go').disabled=true;
const url=mode===1?'/api/v1/auth/login':'/api/v1/auth/register';
const body=mode===1?{email:email.value,password:pwd.value}:{org_name:org.value,email:email.value,name:nm.value,password:pwd.value};
try{const r=await fetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});
const j=await r.json();if(!r.ok){e.textContent=mode===1?'Gagal masuk ('+r.status+'). Periksa email & kata sandi.':'Gagal daftar ('+r.status+').';document.getElementById('go').disabled=false;return;}
localStorage.setItem('lr_token',j.token);location.href='/app';}catch(x){e.textContent='Server tidak terjangkau.';document.getElementById('go').disabled=false;}}</script>
</body></html>"##;

const APP_HTML: &str = r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Dasbor — LegalRust</title><link rel="stylesheet" href="/static/vendor/tabler.min.css"><link rel="stylesheet" href="/static/vendor/tabler-fonts/tabler-icons.min.css"><style>body{font-family:'Plus Jakarta Sans',system-ui,sans-serif}@font-face{font-family:'Plus Jakarta Sans';src:url('/static/fonts/PlusJakartaSans.ttf') format('truetype');font-weight:100 900;font-display:swap}</style></head><body>
<div class="page"><header class="navbar navbar-expand-md navbar-light sticky-top bg-white border-bottom"><div class="container-xl">
<a href="/" class="navbar-brand">⚖️ <strong>LegalRust</strong></a>
<ul class="navbar-nav mx-auto" id="tabs"><li class="nav-item"><a class="nav-link active" href="#" onclick="tab('ringkas');return false">Ringkasan</a></li><li class="nav-item"><a class="nav-link" href="#" onclick="tab('dok');return false">Dokumen</a></li><li class="nav-item"><a class="nav-link" href="#" onclick="tab('akta');return false">Akta</a></li><li class="nav-item"><a class="nav-link" href="#" onclick="tab('keu');return false">Keuangan</a></li></ul>
<button class="btn btn-outline-danger btn-sm" onclick="out()">Keluar</button></div></header>
<div class="page-wrapper"><div class="container-xl py-4">
<h2 class="page-title" id="hi">Dasbor</h2><div class="text-danger" id="err"></div>
<div id="p-ringkas"><div class="row g-2" id="g"></div><div class="card mt-3"><div class="card-header"><h3 class="card-title">Perkara Aktif</h3></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody id="m"></tbody></table></div></div></div>
<div id="p-dok" style="display:none"><div class="card"><div class="card-header"><h3 class="card-title">Dokumen</h3></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th></tr></thead><tbody id="dl"></tbody></table></div></div>
<div class="card mt-3"><div class="card-body"><h3 class="card-title">Unggah dokumen</h3><input class="form-control mb-2" id="dn" placeholder="nama file, mis. kontrak.pdf"><input class="form-control mb-2" type="file" id="df"><button class="btn btn-primary" onclick="up()">Unggah</button> <span class="text-danger" id="ue"></span></div></div></div>
<div id="p-akta" style="display:none"><div class="card"><div class="card-header"><h3 class="card-title">Akta</h3></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="al"></tbody></table></div></div><div class="text-muted mt-2" id="ae"></div></div>
<div id="p-keu" style="display:none"><div class="row g-2"><div class="col-md-6"><div class="card"><div class="card-header"><h3 class="card-title">Invoice per perkara</h3></div><div class="card-body"><input class="form-control mb-2" id="km" placeholder="ID perkara"><button class="btn btn-secondary mb-2" onclick="invs()">Tampilkan</button><div id="il"></div></div></div></div>
<div class="col-md-6"><div class="card"><div class="card-header"><h3 class="card-title">Trust transfer</h3></div><div class="card-body"><input class="form-control mb-2" id="tf" placeholder="dari rekening ID"><input class="form-control mb-2" id="tt" placeholder="ke rekening ID"><input class="form-control mb-2" id="ta" placeholder="nominal minor"><button class="btn btn-primary" onclick="trf()">Transfer</button> <span class="text-danger" id="te"></span></div></div></div></div>
<div class="card mt-3"><div class="card-header"><h3 class="card-title">Rekening trust</h3></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Klien</th><th>Saldo</th></tr></thead><tbody id="tl"></tbody></table></div></div></div>
</div></div></div>
<script src="/static/vendor/tabler.min.js"></script>
<script>
const t=localStorage.getItem('lr_token');if(!t)location.href='/login';
const H={'Authorization':'Bearer '+t};
async function j(u){const r=await fetch(u,{headers:H});if(r.status===401){location.href='/login';return null;}return r.json();}
function tab(x){['ringkas','dok','akta','keu'].forEach(k=>{document.getElementById('p-'+k).style.display=k===x?'block':'none';});document.querySelectorAll('#tabs .nav-link').forEach(a=>a.classList.toggle('active',a.textContent.toLowerCase().includes({ringkas:'ringkasan',dok:'dokumen',akta:'akta',keu:'keuangan'}[x])));}
(async()=>{try{
const me=await j('/api/v1/orgs/me');document.getElementById('hi').textContent='Dasbor — '+me.org_id.slice(0,8)+'…';
const c=await j('/api/v1/clients'),m=await j('/api/v1/matters'),d=await j('/api/v1/documents'),a=await j('/api/v1/deeds'),tr=await j('/api/v1/trust/accounts');
const cards=[['Klien',c.length],['Perkara',m.length],['Dokumen',d.length],['Akta',a.length],['Trust',tr.length],['Saldo trust','Rp '+tr.reduce((s,x)=>s+(x.balance||0),0).toLocaleString('id-ID')]];
document.getElementById('g').innerHTML=cards.map(x=>'<div class="col-6 col-md-4"><div class="card"><div class="card-body"><div class="text-muted">'+x[0]+'</div><div class="h1 m-0">'+x[1]+'</div></div></div></div>').join('');
document.getElementById('m').innerHTML=m.map(x=>'<tr><td><strong>'+x.number+'</strong></td><td>'+x.title+'</td><td><span class="badge bg-green-lt">'+x.status+'</span></td></tr>').join('');
document.getElementById('dl').innerHTML=d.map(x=>'<tr><td>'+x.id+'</td><td>'+x.name+'</td></tr>').join('');
document.getElementById('al').innerHTML=a.map(x=>'<tr><td>'+x.id+'</td><td>'+x.number+'</td><td><span class="badge bg-blue-lt">'+x.state+'</span></td><td><button class="btn btn-sm btn-outline-primary" onclick="adv('+x.id+',\''+x.state+'\')">Maju →</button></td></tr>').join('');
document.getElementById('tl').innerHTML=tr.map(x=>'<tr><td>'+x.id+'</td><td>'+x.client_id+'</td><td>Rp '+(x.balance||0).toLocaleString('id-ID')+'</td></tr>').join('');
}catch(e){document.getElementById('err').textContent='Gagal memuat: '+e;}})();
const NX={draft:'review',review:'approved',approved:'signed'};
async function adv(id,st){const to=NX[st];if(!to){document.getElementById('ae').textContent='Status akhir.';return;}
const r=await fetch('/api/v1/deeds/'+id+'/transition',{method:'POST',headers:{...H,'Content-Type':'application/json'},body:JSON.stringify({to})});
if(!r.ok){document.getElementById('ae').textContent='Gagal ('+r.status+'): transisi ilegal dari '+st;return;}location.reload();}
async function up(){const f=document.getElementById('df').files[0];if(!f){document.getElementById('ue').textContent='Pilih berkas dulu.';return;}
const buf=await f.arrayBuffer();const h=await crypto.subtle.digest('SHA-256',buf);const sha=[...new Uint8Array(h)].map(b=>b.toString(16).padStart(2,'0')).join('');
const meta=await (await fetch('/api/v1/documents',{method:'POST',headers:{...H,'Content-Type':'application/json'},body:JSON.stringify({name:document.getElementById('dn').value||f.name,mime:f.type||'application/octet-stream',size:f.size,sha})})).json();
const r=await fetch('/api/v1/documents/'+meta.id+'/content',{method:'PUT',headers:H,body:buf});
if(!r.ok){document.getElementById('ue').textContent='Gagal unggah ('+r.status+')';return;}location.reload();}
async function invs(){const id=document.getElementById('km').value;const v=await j('/api/v1/matters/'+id+'/invoices');
document.getElementById('il').innerHTML=v.map(x=>'<div>#'+x.number+' — Rp '+x.total.toLocaleString('id-ID')+'</div>').join('')||'Belum ada invoice.';}
async function trf(){const r=await fetch('/api/v1/trust/transfer',{method:'POST',headers:{...H,'Content-Type':'application/json'},body:JSON.stringify({from:+document.getElementById('tf').value,to:+document.getElementById('tt').value,amount_minor:+document.getElementById('ta').value})});
if(!r.ok){document.getElementById('te').textContent='Gagal ('+r.status+'): saldo tidak cukup?';return;}location.reload();}
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
async fn public_contact(
    State(s): State<FullState>,
    Json(b): Json<ContactReq>,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("mkt:contact") {
        return Err(AppError::RateLimited);
    }
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
    Json(b): Json<DemoReq>,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("mkt:demo") {
        return Err(AppError::RateLimited);
    }
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
