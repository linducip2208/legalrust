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
    CoreStore, CrmStore, DocRow, MarketingArticleRow, MarketingFaqRow, MarketingLeadRow,
    MarketingPlanRow, MarketingStore, MatterRow, TaskRow, UserStore,
};
use legalrust_database::{DeedRow, InvoiceRow, OcrJobRow, OpsStore, TemplateRow};
use legalrust_domain::{MarketingLeadStatus, Permission, Role};
use legalrust_ocr::OcrEngine;
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
    pub crm: Arc<dyn CrmStore>,
    pub objects: Arc<dyn ObjectStore>,
    pub jwt_secret: Vec<u8>,
    pub integration_secret: Vec<u8>,
    pub integrations: Vec<String>,
    pub tripay: legalrust_integrations::tripay::TripayConfig,
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
        crm: Arc<dyn CrmStore>,
        objects: Arc<dyn ObjectStore>,
        jwt_secret: Vec<u8>,
    ) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(64);
        Self {
            users,
            core,
            ops,
            mkt,
            crm,
            objects,
            jwt_secret,
            integration_secret: Vec::new(),
            integrations: vec!["dukcapil".into(), "bpn".into(), "psre".into()],
            tripay: legalrust_integrations::tripay::TripayConfig::default_env(),
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
/// Own active sessions for the console (F-D16-019). `current` marks the
/// caller's own session, which cannot revoke itself here (use logout).
/// Least-privilege role catalog matrix (F-D16-007): every role with its
/// exact permission set, served as reference for assignment decisions.
async fn role_matrix(
    State(_s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &_s).await?;
    authorize(&u, Permission::View)?;
    Ok(Json(json!(legalrust_domain::Role::all()
        .iter()
        .map(|r| json!({
            "role": r.as_str(),
            "permissions": legalrust_authorization::permissions(*r)
                .iter()
                .map(|p| format!("{p:?}"))
                .collect::<Vec<_>>(),
        }))
        .collect::<Vec<_>>())))
}
async fn session_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let v = s
        .stores
        .users
        .list_sessions(&u.claims.sub)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|x| json!({"id": x.id, "expires_at": x.expires_at, "revoked": x.revoked, "current": x.id == u.claims.sid}))
        .collect::<Vec<_>>())))
}
async fn session_revoke(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    if id == u.claims.sid || id == "api-token" {
        return Err(AppError::Conflict("cannot revoke current session".into()));
    }
    // Ownership check: only own sessions are revocable here.
    let own = s
        .stores
        .users
        .list_sessions(&u.claims.sub)
        .await
        .map_err(map_store)?;
    if !own.iter().any(|x| x.id == id) {
        return Err(AppError::NotFound("session".into()));
    }
    s.stores
        .users
        .revoke_session(&id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"revoked": true})))
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
        .map(|t| json!({"id": t.id, "title": t.title, "status": t.status, "due": t.due}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct TaskDueReq {
    due: Option<String>,
}
async fn task_set_due(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(tid): Path<i64>,
    Json(b): Json<TaskDueReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    let t = s
        .stores
        .core
        .task_set_due(&u.claims.org, tid, b.due.as_deref())
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::NotFound => AppError::NotFound("task".into()),
            legalrust_database::StoreError::Conflict(m) => AppError::Conflict(m),
            _ => AppError::Internal(anyhow::anyhow!(e.to_string())),
        })?;
    Ok(Json(json!({"id": t.id, "due": t.due})))
}
#[derive(Debug, Deserialize)]
struct TaskDueListQ {
    limit: Option<i64>,
}
async fn task_due_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<TaskDueListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .core
        .task_due_list(&u.claims.org, q.limit.unwrap_or(50).clamp(1, 200))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(Json(json!(v
        .iter()
        .map(|t| json!({"id": t.id, "title": t.title, "status": t.status, "due": t.due, "matter_id": t.matter_id}))
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
    let (doc, versions) = s
        .stores
        .core
        .get_doc(&u.claims.org, job.doc)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let latest = versions
        .last()
        .ok_or_else(|| AppError::Validation("no content; upload document bytes first".into()))?;
    if !doc.mime.starts_with("image/") {
        return Err(AppError::Validation(
            "binary parsing not supported (images only; pdf pending)".into(),
        ));
    }
    let bytes = s.stores.objects.get(&latest.object_key).await?;
    let engine = legalrust_ocr::TesseractAdapter;
    let doc_type =
        legalrust_ocr::DocType::parse(&job.doc_type).unwrap_or(legalrust_ocr::DocType::GeneralPdf);
    let fields = engine.recognize(&bytes, doc_type)?;
    let rows: Vec<legalrust_database::OcrFieldRow> = fields
        .iter()
        .map(|f| legalrust_database::OcrFieldRow {
            name: f.name.clone(),
            value: f.value.clone(),
            confidence: f.confidence,
            page: f.page as i64,
        })
        .collect();
    s.stores
        .ops
        .ocr_store_result(&u.claims.org, id, "processed", &rows)
        .await
        .map_err(map_store)?;
    tracing::info!(
        job_id = id,
        doc_type = %job.doc_type,
        lines = fields.len(),
        "ocr process ok: native tesseract"
    );
    notify_user(
        &s,
        &u.claims.org,
        &u.claims.sub,
        "ocr",
        &format!("OCR processed: job {id} ({} lines)", fields.len()),
    )
    .await;
    Ok(Json(json!({
        "status": "processed",
        "lines": fields.len(),
        "fields": fields.iter().map(|f| json!({"name": f.name, "value": f.value, "confidence": f.confidence, "page": f.page})).collect::<Vec<_>>(),
    })))
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
#[derive(Debug, Deserialize, Validate)]
struct ClauseReq {
    #[validate(length(min = 2, max = 200))]
    name: String,
    #[validate(length(max = 60))]
    category: Option<String>,
    #[validate(length(min = 1))]
    body: String,
}
async fn clause_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ClauseReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let r = s
        .stores
        .ops
        .clause_create(
            &u.claims.org,
            b.name.trim(),
            b.category.as_deref().unwrap_or(""),
            &b.body,
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": r.id, "version": r.version})))
}
async fn clause_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .clause_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "name": r.name, "category": r.category, "version": r.version}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct ClausePut {
    #[validate(length(min = 1))]
    body: String,
}
async fn clause_update(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<ClausePut>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let r = s
        .stores
        .ops
        .clause_update(&u.claims.org, id, &b.body)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": r.id, "version": r.version})))
}
async fn clause_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .clause_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
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
        .map(|d| json!({"id": d.id, "number": d.number, "state": d.state, "created": d.created}))
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
        json!({"id": d.id, "matter_id": d.matter_id, "number": d.number, "deed_type": d.deed_type, "state": d.state, "created": d.created}),
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
    // Live path: env-driven endpoint (local Ollama defaults). Unreachable or
    // rejected endpoints surface as 501 — never fake content.
    let ep = legalrust_ai::LiveEndpoint::resolve_from_env();
    let client = ep.http_client().map_err(AppError::from)?;
    let ai_msgs: Vec<legalrust_ai::ChatMessage> = b
        .messages
        .iter()
        .map(|m| legalrust_ai::ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();
    match legalrust_ai::complete(&client, &ep, b.system.as_deref(), &ai_msgs).await {
        Ok(live) => {
            s.stores
                .ops
                .msg_add(conv, "assistant", &live.content)
                .await
                .map_err(map_store)?;
            s.stores
                .ops
                .usage_add(&u.claims.org, 0, live.output_tokens)
                .await
                .map_err(map_store)?;
            notify_user(
                &s,
                &u.claims.org,
                &u.claims.sub,
                "ai",
                &format!(
                    "AI reply ({}): {} output tokens",
                    live.model, live.output_tokens
                ),
            )
            .await;
            Ok(Json(json!({
                "reply": live.content,
                "model": live.model,
                "input_tokens": live.input_tokens,
                "output_tokens": live.output_tokens,
            })))
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("unreachable") || msg.contains("transport") {
                Err(AppError::BlockedExternal(format!(
                    "no AI provider configured at {} (set AI_BASE_URL/AI_MODEL or run local Ollama); estimated input tokens: {est} (logged, zero output)",
                    ep.base_url
                )))
            } else {
                Err(AppError::from(e))
            }
        }
    }
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
    let lim = q.limit.unwrap_or(10).clamp(1, 50);
    let hits = s
        .stores
        .ops
        .rag_search(&u.claims.org, &q.q, lim)
        .await
        .map_err(map_store)?;
    // Rank by keyword relevance, then enforce tenant scope at the API boundary
    // (the MySQL impl already scopes by org; the memory impl scans all docs).
    // RagHit.idx travels in ScoredChunk.version_id so ranking stays lossless.
    let chunks: Vec<legalrust_rag::ScoredChunk> = hits
        .into_iter()
        .map(|h| legalrust_rag::ScoredChunk {
            doc_id: h.doc_id.to_string(),
            version_id: h.idx,
            page: 0,
            text: h.text,
            score: 0.0,
        })
        .collect();
    let mut out = Vec::new();
    for c in legalrust_rag::rank(&q.q, &chunks, lim as usize) {
        let doc_id: i64 = c.doc_id.parse().unwrap_or(-1);
        if s.stores.core.get_doc(&u.claims.org, doc_id).await.is_ok() {
            out.push(json!({"doc_id": doc_id, "idx": c.version_id, "text": c.text}));
        }
    }
    Ok(Json(json!(out)))
}

#[derive(Debug, Clone, serde::Serialize)]
struct CatalogFeature {
    id: String,
    domain: String,
    title: String,
    status: String,
    overall_score: f64,
    verified: bool,
}
/// Transparent 1000-feature scoreboard, merged at compile time from the
/// canonical registry + 8-cat scores. Scores are honest: only genuinely
/// implemented + verified work scores high; the rest show real gaps.
fn catalog() -> &'static [CatalogFeature] {
    static ONCE: std::sync::OnceLock<Vec<CatalogFeature>> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let reg: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/AUTONOMOUS/FEATURE-REGISTRY.json"
        ))
        .unwrap_or(serde_json::Value::Null);
        let sco: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/AUTONOMOUS/AUDIT-1000/FEATURE-SCORES.json"
        ))
        .unwrap_or(serde_json::Value::Null);
        let mut by_score = std::collections::HashMap::new();
        for s in sco.as_array().cloned().unwrap_or_default() {
            if let Some(id) = s.get("feature_id").and_then(|v| v.as_str()) {
                by_score.insert(
                    id.to_string(),
                    (
                        s.get("overall_score")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        s.get("verified_8cat")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    ),
                );
            }
        }
        let mut out: Vec<CatalogFeature> = reg
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|f| {
                let id = f.get("id")?.as_str()?.to_string();
                let (score, verified) = by_score.get(&id).cloned().unwrap_or((0.0, false));
                Some(CatalogFeature {
                    domain: f
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    title: f.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
                    status: f
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    overall_score: score,
                    verified,
                    id,
                })
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    })
}

#[derive(Debug, Deserialize)]
struct ScoreQ {
    q: Option<String>,
    domain: Option<String>,
    status: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}
async fn feature_scores(
    State(_s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<ScoreQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &_s).await?;
    authorize(&u, Permission::View)?;
    if let Some(st) = q.status.as_deref() {
        match st.to_lowercase().as_str() {
            "implemented" | "partial" | "planned" | "not_started" | "configuration_required" => {}
            _ => return Err(AppError::Validation("status".into())),
        }
    }
    if let Some(d) = q.domain.as_deref() {
        let du = d.trim().to_uppercase();
        if du.len() != 3 || !du.starts_with('D') || du[1..].parse::<u8>().is_err() {
            return Err(AppError::Validation("domain".into()));
        }
    }
    let needle = q.q.unwrap_or_default().trim().to_lowercase();
    let per_page = q.per_page.unwrap_or(50).clamp(1, 100) as usize;
    let page = q.page.unwrap_or(1).clamp(1, 100) as usize;
    let all = catalog();
    let filtered: Vec<&CatalogFeature> = all
        .iter()
        .filter(|f| {
            (q.domain
                .as_deref()
                .is_none_or(|d| f.domain.eq_ignore_ascii_case(d.trim())))
                && (q
                    .status
                    .as_deref()
                    .is_none_or(|x| f.status.eq_ignore_ascii_case(x)))
                && (needle.is_empty()
                    || f.id.to_lowercase().contains(&needle)
                    || f.title.to_lowercase().contains(&needle))
        })
        .collect();
    let total = filtered.len();
    let skip = (page - 1) * per_page;
    let items: Vec<Value> = filtered
        .into_iter()
        .skip(skip)
        .take(per_page)
        .map(|f| {
            json!({"id": f.id, "domain": f.domain, "title": f.title,
                   "status": f.status, "overall_score": f.overall_score,
                   "verified": f.verified})
        })
        .collect();
    let mut by_status = serde_json::Map::new();
    let mut implemented = 0usize;
    let mut sum = 0.0;
    for f in all {
        *by_status.entry(f.status.clone()).or_insert(json!(0)) = json!(
            by_status
                .get(&f.status)
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + 1
        );
        if f.status == "IMPLEMENTED" {
            implemented += 1;
        }
        sum += f.overall_score;
    }
    Ok(Json(json!({
        "items": items, "total": total, "page": page, "per_page": per_page,
        "summary": {
            "features": all.len(), "implemented": implemented,
            "by_status": by_status,
            "mean_score": (sum / all.len().max(1) as f64 * 10.0).round() / 10.0,
        },
    })))
}

#[derive(Debug, Deserialize)]
struct SearchQ {
    q: Option<String>,
    limit: Option<i64>,
    page: Option<i64>,
    per_page: Option<i64>,
    types: Option<String>,
}
async fn global_search(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<SearchQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let needle = q.q.unwrap_or_default().trim().to_lowercase();
    if needle.chars().count() < 2 {
        return Err(AppError::Validation("q".into()));
    }
    // `limit` is kept as a legacy alias for `per_page`.
    let per_page = q.per_page.or(q.limit).unwrap_or(10).clamp(1, 50) as usize;
    let page = q.page.unwrap_or(1).clamp(1, 100) as usize;
    let mut want_clients = true;
    let mut want_matters = true;
    let mut want_docs = true;
    if let Some(t) = q.types.as_deref() {
        want_clients = false;
        want_matters = false;
        want_docs = false;
        let mut any = false;
        for part in t.split(',').map(str::trim).filter(|x| !x.is_empty()) {
            match part.to_lowercase().as_str() {
                "clients" => want_clients = true,
                "matters" => want_matters = true,
                "documents" | "docs" => want_docs = true,
                _ => return Err(AppError::Validation("types".into())),
            }
            any = true;
        }
        if !any {
            return Err(AppError::Validation("types".into()));
        }
    }
    let skip = (page - 1) * per_page;
    let mut totals = serde_json::Map::new();
    let mut hc_all: Vec<Value> = Vec::new();
    let mut hm_all: Vec<Value> = Vec::new();
    let mut hd_all: Vec<Value> = Vec::new();
    let clients = s
        .stores
        .core
        .list_clients(&u.claims.org)
        .await
        .map_err(map_store)?;
    let matters = s
        .stores
        .core
        .list_matters(&u.claims.org)
        .await
        .map_err(map_store)?;
    let docs = s
        .stores
        .core
        .list_docs(&u.claims.org, None)
        .await
        .map_err(map_store)?;
    if want_clients {
        hc_all = clients
            .into_iter()
            .filter(|c| c.display_name.to_lowercase().contains(&needle))
            .map(|c| json!({"id": c.id, "name": c.display_name}))
            .collect();
    }
    if want_matters {
        hm_all = matters
            .into_iter()
            .filter(|m| {
                m.number.to_lowercase().contains(&needle)
                    || m.title.to_lowercase().contains(&needle)
            })
            .map(|m| json!({"id": m.id, "number": m.number, "title": m.title}))
            .collect();
    }
    if want_docs {
        hd_all = docs
            .into_iter()
            .filter(|d| d.name.to_lowercase().contains(&needle))
            .map(|d| json!({"id": d.id, "name": d.name}))
            .collect();
    }
    totals.insert("clients".into(), json!(hc_all.len()));
    totals.insert("matters".into(), json!(hm_all.len()));
    totals.insert("documents".into(), json!(hd_all.len()));
    // Backward-compat keys stay arrays; pagination applies per group.
    let paged = |v: Vec<Value>| v.into_iter().skip(skip).take(per_page).collect::<Vec<_>>();
    Ok(Json(json!({
        "clients": paged(hc_all),
        "matters": paged(hm_all),
        "documents": paged(hd_all),
        "totals": totals,
        "page": page,
        "per_page": per_page,
    })))
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
    // Attach paid/balance so the Keuangan tab can record payments honestly.
    let mut out = Vec::new();
    for i in &v {
        let paid: i64 = s
            .stores
            .ops
            .inv_get(&u.claims.org, i.id)
            .await
            .map_err(map_store)
            .map(|(_, ps)| ps.iter().map(|p| p.amount).sum())
            .unwrap_or(0);
        out.push(json!({"id": i.id, "number": i.number, "total": i.total,
                        "paid": paid, "balance": i.total - paid}));
    }
    Ok(Json(json!(out)))
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
        json!({"id": v.id, "total": v.total, "paid": paid, "balance": v.total - paid, "payments": ps.iter().map(|p| json!({"id": p.id, "amount": p.amount})).collect::<Vec<_>>() }),
    ))
}
/// Printable payment receipt for an invoice (F-D15-036). View-gated.
async fn inv_receipt(
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
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.receipt_viewed".into(),
            entity: "report".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({
        "id": v.id, "number": v.number, "matter_id": v.matter_id,
        "total": v.total, "paid": paid, "balance": v.total - paid,
        "currency": v.currency, "status": if v.total - paid == 0 { "lunas" } else { "belum_lunas" },
        "issued_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "payments": ps.iter().map(|p| json!({"id": p.id, "amount": p.amount, "method": p.method})).collect::<Vec<_>>(),
    })))
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
    Json(b): Json<PayGatewayReq>,
) -> Result<Json<Value>, AppError> {
    use legalrust_integrations::tripay;
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let cfg = s.stores.tripay.resolve().map_err(AppError::from)?;
    let (invoice, payments) = s
        .stores
        .ops
        .inv_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    let paid: i64 = payments.iter().map(|p| p.amount).sum();
    let balance = invoice.total - paid;
    if balance <= 0 {
        return Err(AppError::Conflict("invoice already settled".into()));
    }
    let (callback_url, return_url) = s.stores.tripay.callback_urls();
    let req = tripay::TripayCreateReq {
        channel: b.channel.trim().to_uppercase(),
        merchant_ref: tripay::build_merchant_ref(id),
        amount_minor: balance,
        customer_name: b.customer_name.trim().to_string(),
        customer_email: b.customer_email.clone(),
        customer_phone: b.customer_phone.clone(),
        items: vec![tripay::TripayItem::new(
            &invoice.number,
            format!("Pembayaran {}", invoice.number),
            balance,
            1,
        )],
        expired_hours: b.expired_hours.unwrap_or(24),
        callback_url,
        return_url,
    };
    let client = tripay::http_client(30).map_err(AppError::from)?;
    let res = tripay::create_transaction(&client, &cfg, &req)
        .await
        .map_err(AppError::from)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "invoice.tripay_checkout".into(),
            entity: "invoice".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    notify_user(
        &s,
        &u.claims.org,
        &u.claims.sub,
        "invoice",
        &format!("Tripay checkout: {} (Rp {balance})", res.reference),
    )
    .await;
    Ok(Json(json!({
        "reference": res.reference,
        "merchant_ref": res.merchant_ref,
        "checkout_url": res.checkout_url,
        "amount": res.amount_minor,
    })))
}

#[derive(Debug, Deserialize, Validate)]
struct PayGatewayReq {
    #[validate(length(min = 1, max = 30))]
    channel: String,
    #[validate(length(min = 2, max = 120))]
    customer_name: String,
    customer_email: Option<String>,
    customer_phone: Option<String>,
    expired_hours: Option<i64>,
}

/// Best-effort delivery-log writer: logging must never fail the webhook.
async fn log_wh(
    s: &FullState,
    org: Option<&str>,
    provider: &str,
    event: &str,
    status: &str,
    detail: &str,
) {
    let detail: String = detail.chars().take(255).collect();
    let _ = s
        .stores
        .ops
        .webhook_log(org, provider, event, status, &detail)
        .await;
}
async fn tripay_webhook(
    s: &FullState,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    use legalrust_integrations::tripay;
    let private_key = match std::env::var(&s.stores.tripay.private_key_env) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            log_wh(s, None, "tripay", "", "blocked", "callback not configured").await;
            return Err(AppError::BlockedExternal(
                "tripay callback not configured".into(),
            ));
        }
    };
    let sig = headers
        .get("x-callback-signature")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;
    if let Err(e) = tripay::verify_tripay_callback(&private_key, &body, sig) {
        log_wh(s, None, "tripay", "", "rejected", "bad signature").await;
        return Err(AppError::from(e));
    }
    let cb = match tripay::TripayCallback::parse(&body) {
        Ok(v) => v,
        Err(e) => {
            log_wh(s, None, "tripay", "", "rejected", "unparseable body").await;
            return Err(AppError::from(e));
        }
    };
    // Best-effort tenant attribution for the delivery log (unknown → None).
    let mut org_hint: Option<String> = None;
    if let Some(inv) = tripay::parse_merchant_ref(&cb.merchant_ref) {
        if let Ok(org) = s.stores.ops.inv_owner_org(inv).await {
            org_hint = org;
        }
    }
    if !cb.is_paid() {
        log_wh(s, org_hint.as_deref(), "tripay", &cb.status, "accepted", "").await;
        return Ok(Json(
            json!({"accepted": true, "processed": false, "status": cb.status}),
        ));
    }
    let inv_id = tripay::parse_merchant_ref(&cb.merchant_ref)
        .ok_or_else(|| AppError::Validation("unknown merchant_ref".into()))?;
    let org = s
        .stores
        .ops
        .inv_owner_org(inv_id)
        .await
        .map_err(map_store)?
        .ok_or_else(|| AppError::NotFound("invoice".into()))?;
    let (invoice, payments) = s
        .stores
        .ops
        .inv_get(&org, inv_id)
        .await
        .map_err(map_store)?;
    let marker = format!(
        "tripay:{}",
        cb.reference.chars().take(32).collect::<String>()
    );
    if payments.iter().any(|p| p.method == marker) {
        log_wh(s, Some(&org), "tripay", "PAID", "duplicate", &cb.reference).await;
        return Ok(Json(json!({"accepted": true, "duplicate": true})));
    }
    let paid: i64 = payments.iter().map(|p| p.amount).sum();
    let amount = if cb.amount_received > 0 {
        cb.amount_received
    } else {
        invoice.total - paid
    };
    let p = s
        .stores
        .ops
        .pay_add(&org, inv_id, amount, &marker)
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: org.clone(),
            actor_id: None,
            action: "invoice.tripay_paid".into(),
            entity: "invoice".into(),
            entity_id: inv_id,
            ip: None,
        },
    )
    .await;
    notify_user(
        s,
        &org,
        "tripay",
        "invoice",
        &format!("Tripay PAID: {} (Rp {})", cb.reference, p.amount),
    )
    .await;
    log_wh(s, Some(&org), "tripay", "PAID", "accepted", &cb.reference).await;
    Ok(Json(
        json!({"accepted": true, "invoice_id": inv_id, "amount": p.amount}),
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
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.matters_csv_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    Ok((StatusCode::OK, h, csv))
}

async fn clients_csv(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .core
        .list_clients(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let mut csv = String::from("id,display_name,kind\n");
    for c in &v {
        csv.push_str(&format!(
            "{},{},{}\n",
            c.id,
            csv_field(&c.display_name),
            csv_field(&c.kind)
        ));
    }
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.clients_csv_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    Ok((StatusCode::OK, h, csv))
}

#[derive(Debug, Deserialize)]
struct TrustCsvQ {
    account_id: i64,
}
async fn trust_ledger_csv_export(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<TrustCsvQ>,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let (_, entries) = s
        .stores
        .ops
        .trust_get(&u.claims.org, q.account_id)
        .await
        .map_err(map_store)?;
    let rows: Vec<legalrust_reporting::TrustLedgerEntry> = entries
        .iter()
        .map(|e| legalrust_reporting::TrustLedgerEntry {
            id: e.id.to_string(),
            matter_id: e
                .matter_id
                .map(|m| m.to_string())
                .unwrap_or_else(|| "-".into()),
            amount_cents: e.amount,
            memo: format!("{} {}", e.kind, e.ref_type),
        })
        .collect();
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.trust_csv_exported".into(),
            entity: "report".into(),
            entity_id: q.account_id,
            ip: None,
        },
    )
    .await;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    Ok((
        StatusCode::OK,
        h,
        legalrust_reporting::trust_ledger_csv(&rows),
    ))
}

/// Open-invoice AR aging report: unpaid/partially-paid invoices org-wide
/// with paid/balance math. Export-gated + audit-trailed.
async fn ar_aging_csv_export(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let open = s
        .stores
        .ops
        .inv_list_open(&u.claims.org)
        .await
        .map_err(map_store)?;
    let rows: Vec<legalrust_reporting::ArAgingEntry> = open
        .iter()
        .map(|r| legalrust_reporting::ArAgingEntry {
            id: r.id.to_string(),
            number: r.number.clone(),
            matter_id: r.matter_id.to_string(),
            total_minor: r.total,
            paid_minor: r.paid,
            balance_minor: r.balance,
        })
        .collect();
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.ar_csv_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    Ok((StatusCode::OK, h, legalrust_reporting::ar_aging_csv(&rows)))
}
// ---------- reports: approval bottleneck + client profitability ----------
/// Where approvals are stuck: oldest pending request first with its blocking
/// step/role. No fake durations: id order is the age proxy (F-D20-012).
async fn approvals_bottleneck_csv(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let pending = s
        .stores
        .ops
        .appr_list(&u.claims.org, Some("pending"))
        .await
        .map_err(map_store)?;
    let mut rows: Vec<String> = Vec::new();
    for r in &pending {
        let (_, steps) = s
            .stores
            .ops
            .appr_get(&u.claims.org, r.id)
            .await
            .map_err(map_store)?;
        let stuck = steps
            .iter()
            .filter(|x| x.status == "pending")
            .min_by_key(|x| x.step_no);
        let (no, role) = stuck
            .map(|x| (x.step_no.to_string(), x.role.clone()))
            .unwrap_or(("-".into(), "-".into()));
        let open = steps.iter().filter(|x| x.status == "pending").count();
        rows.push(format!(
            "{},{},{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.entity),
            no,
            csv_field(&role),
            open
        ));
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.approvals_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    let mut csv = String::from("id,title,entity,stuck_step,stuck_role,pending_steps\n");
    for r in rows {
        csv.push_str(&r);
    }
    Ok((StatusCode::OK, h, csv))
}
/// Per-client matter count + open invoice balance (F-D20-014).
/// Deadline compliance per matter + compliance posture snapshot (F-D20-013, F-D20-017).
async fn profitability_csv(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let clients = s
        .stores
        .core
        .list_clients(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let matters = s
        .stores
        .core
        .list_matters(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let open = s
        .stores
        .ops
        .inv_list_open(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut csv = String::from("client_id,client_name,matters,open_balance\n");
    for c in &clients {
        let mids: Vec<i64> = matters
            .iter()
            .filter(|m| m.client_id == Some(c.id))
            .map(|m| m.id)
            .collect();
        let bal: i64 = open
            .iter()
            .filter(|r| mids.contains(&r.matter_id))
            .map(|r| r.balance)
            .sum();
        csv.push_str(&format!(
            "{},{},{},{}\n",
            c.id,
            csv_field(&c.display_name),
            mids.len(),
            bal
        ));
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.profitability_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    Ok((StatusCode::OK, h, csv))
}
/// Deadline compliance per matter: totals, open, and overdue tasks.
/// Overdue = due < now (UTC, lexicographic on validated format) and not done.
async fn deadline_compliance_csv(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let matters = s
        .stores
        .core
        .list_matters(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let now = chrono::Utc::now()
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let mut csv = String::from("matter_id,number,title,total_tasks,open_tasks,overdue_tasks\n");
    for m in &matters {
        let ts = s
            .stores
            .core
            .list_tasks(&u.claims.org, m.id)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
        let total = ts.len() as i64;
        let open = ts.iter().filter(|t| t.status != "done").count() as i64;
        let overdue = ts
            .iter()
            .filter(|t| t.status != "done" && t.due.as_deref().is_some_and(|d| d < now.as_str()))
            .count() as i64;
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            m.id,
            csv_field(&m.number),
            csv_field(&m.title),
            total,
            open,
            overdue
        ));
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.deadlines_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    Ok((StatusCode::OK, h, csv))
}
/// Compliance posture snapshot: obligations, consents, ROPA, DPIA, breaches.
async fn compliance_posture_csv(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let ob = s
        .stores
        .ops
        .pdp_obligation_list(&u.claims.org, None)
        .await
        .map_err(map_store)?;
    let co = s
        .stores
        .ops
        .consent_list(&u.claims.org, None)
        .await
        .map_err(map_store)?;
    let ropa = s
        .stores
        .ops
        .ropa_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let dpia = s
        .stores
        .ops
        .dpia_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let breach = s
        .stores
        .ops
        .breach_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let count = |v: &[&str], x: &str| v.iter().filter(|s| **s == x).count();
    let ob_s: Vec<&str> = ob.iter().map(|r| r.status.as_str()).collect();
    let co_s: Vec<&str> = co.iter().map(|r| r.status.as_str()).collect();
    let dp_s: Vec<&str> = dpia.iter().map(|r| r.status.as_str()).collect();
    let br_s: Vec<&str> = breach.iter().map(|r| r.status.as_str()).collect();
    let mut csv = String::from("domain,metric,value\n");
    let rows = [
        ("obligation", "open", count(&ob_s, "open").to_string()),
        (
            "obligation",
            "in_progress",
            count(&ob_s, "in_progress").to_string(),
        ),
        ("obligation", "done", count(&ob_s, "done").to_string()),
        ("consent", "granted", count(&co_s, "granted").to_string()),
        (
            "consent",
            "withdrawn",
            count(&co_s, "withdrawn").to_string(),
        ),
        ("ropa", "entries", ropa.len().to_string()),
        ("dpia", "draft", count(&dp_s, "draft").to_string()),
        ("dpia", "review", count(&dp_s, "review").to_string()),
        ("dpia", "approved", count(&dp_s, "approved").to_string()),
        (
            "breach",
            "open",
            (br_s.len() - count(&br_s, "closed")).to_string(),
        ),
        ("breach", "closed", count(&br_s, "closed").to_string()),
    ];
    for (d, m, v) in rows {
        csv.push_str(&format!("{d},{m},{v}\n"));
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.compliance_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    Ok((StatusCode::OK, h, csv))
}

#[derive(Debug, Deserialize)]
struct DeedCsvQ {
    date: Option<String>,
    from: Option<String>,
    to: Option<String>,
    cmp_from: Option<String>,
    cmp_to: Option<String>,
    compare: Option<String>,
}
/// Notary repertorium: deeds CSV, optionally filtered to one calendar day
/// (`YYYY-MM-DD`) or an inclusive period (`from`/`to`, `YYYY-MM-DD`).
/// Export-gated + audit-trailed.
async fn deeds_csv_export(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<DeedCsvQ>,
) -> Result<impl IntoResponse, AppError> {
    fn valid_day(v: &str) -> bool {
        let b = v.as_bytes();
        b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b[..4].iter().all(u8::is_ascii_digit)
            && b[5..7].iter().all(u8::is_ascii_digit)
            && b[8..10].iter().all(u8::is_ascii_digit)
    }
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    if let Some(d) = q.date.as_deref() {
        if !valid_day(d) {
            return Err(AppError::Validation("date".into()));
        }
    }
    for (name, v) in [("from", q.from.as_deref()), ("to", q.to.as_deref())].iter() {
        if let Some(d) = v {
            if !valid_day(d) {
                return Err(AppError::Validation((*name).into()));
            }
        }
    }
    if let (Some(f), Some(t)) = (q.from.as_deref(), q.to.as_deref()) {
        if f > t {
            return Err(AppError::Validation("from>to".into()));
        }
    }
    // Comparative period analysis (F-D20-024): ?compare=1&from=&to=&cmp_from=&cmp_to=.
    let v = s
        .stores
        .ops
        .deed_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    if matches!(q.compare.as_deref(), Some("1") | Some("true")) {
        let (cf, ct) = match (q.cmp_from.as_deref(), q.cmp_to.as_deref()) {
            (Some(a), Some(b)) => (a, b),
            _ => return Err(AppError::Validation("cmp range".into())),
        };
        for (name, day) in [("cmp_from", Some(cf)), ("cmp_to", Some(ct))].iter() {
            if !valid_day(day.unwrap()) {
                return Err(AppError::Validation((*name).into()));
            }
        }
        if cf > ct {
            return Err(AppError::Validation("cmp_from>cmp_to".into()));
        }
        let (f, t) = match (q.from.as_deref(), q.to.as_deref()) {
            (Some(a), Some(b)) => (a, b),
            _ => return Err(AppError::Validation("range".into())),
        };
        let in_range = |day: &str, a: &str, b: &str| day >= a && day <= b;
        let mut cur = 0i64;
        let mut prev = 0i64;
        for d in &v {
            if d.created.len() < 10 {
                continue;
            }
            let day: &str = &d.created[..10];
            if in_range(day, f, t) {
                cur += 1;
            }
            if in_range(day, cf, ct) {
                prev += 1;
            }
        }
        audit(
            &s.pool,
            &s.stores,
            AuditEvent {
                tenant_id: u.claims.org.clone(),
                actor_id: Some(u.claims.sub.clone()),
                action: "report.deeds_compared".into(),
                entity: "report".into(),
                entity_id: 0,
                ip: None,
            },
        )
        .await;
        let mut h = HeaderMap::new();
        h.insert(
            "content-type",
            HeaderValue::from_static("text/csv; charset=utf-8"),
        );
        let csv = format!(
            "metric,current,previous,delta\ncount,{cur},{prev},{}\n",
            cur - prev
        );
        return Ok((StatusCode::OK, h, csv));
    }
    let mut csv = String::from("id,number,deed_type,state,created\n");
    for d in &v {
        if let Some(day) = q.date.as_deref() {
            if !d.created.starts_with(day) {
                continue;
            }
        }
        // Inclusive YYYY-MM-DD period on the created-date prefix (YYYY-MM-DD HH:MM:SS).
        if d.created.len() >= 10 {
            let day: &str = &d.created[..10];
            if let Some(f) = q.from.as_deref() {
                if day < f {
                    continue;
                }
            }
            if let Some(t) = q.to.as_deref() {
                if day > t {
                    continue;
                }
            }
        }
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            d.id,
            csv_field(&d.number),
            csv_field(&d.deed_type),
            csv_field(&d.state),
            csv_field(&d.created)
        ));
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.deeds_csv_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    let mut h = HeaderMap::new();
    h.insert(
        "content-type",
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    Ok((StatusCode::OK, h, csv))
}

// ---------- compliance slice: UU PDP obligations + consent lifecycle ----------
#[derive(Debug, Deserialize, Validate)]
struct PdpObligationReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 60))]
    category: Option<String>,
    #[validate(length(max = 120))]
    owner: Option<String>,
}
async fn pdp_obligation_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PdpObligationReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .pdp_obligation_create(
            &u.claims.org,
            b.title.trim(),
            b.category.as_deref().unwrap_or("general"),
            b.owner.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.obligation_created".into(),
            entity: "pdp_obligation".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id, "status": "open"})))
}
#[derive(Debug, Deserialize)]
struct PdpListQ {
    status: Option<String>,
}
async fn pdp_obligation_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<PdpListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    if let Some(st) = q.status.as_deref() {
        if !["open", "in_progress", "done"].contains(&st) {
            return Err(AppError::Validation("status".into()));
        }
    }
    let v = s
        .stores
        .ops
        .pdp_obligation_list(&u.claims.org, q.status.as_deref())
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "category": r.category, "status": r.status, "owner": r.owner}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct PdpStatusReq {
    status: String,
}
async fn pdp_obligation_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<PdpStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    if !["open", "in_progress", "done"].contains(&b.status.as_str()) {
        return Err(AppError::Validation("status".into()));
    }
    s.stores
        .ops
        .pdp_obligation_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: format!("compliance.obligation_{}", b.status.trim()),
            entity: "pdp_obligation".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id, "status": b.status})))
}
#[derive(Debug, Deserialize, Validate)]
struct ConsentGrantReq {
    #[validate(length(min = 1, max = 120))]
    subject: String,
    #[validate(length(min = 2, max = 200))]
    purpose: String,
}
async fn consent_grant(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ConsentGrantReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .consent_grant(&u.claims.org, b.subject.trim(), b.purpose.trim())
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "consent.granted".into(),
            entity: "consent".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id, "status": "granted"})))
}
async fn consent_withdraw(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .consent_withdraw(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "consent.withdrawn".into(),
            entity: "consent".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id, "status": "withdrawn"})))
}
#[derive(Debug, Deserialize)]
struct ConsentListQ {
    subject: Option<String>,
}

async fn consent_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<ConsentListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .consent_list(&u.claims.org, q.subject.as_deref())
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(
            |r| json!({"id": r.id, "subject": r.subject, "purpose": r.purpose, "status": r.status})
        )
        .collect::<Vec<_>>())))
}
// ---------- end compliance slice: consent ----------

// ---------- compliance slice: UU PDP ROPA register ----------
#[derive(Debug, Deserialize, Validate)]
struct RopaCreateReq {
    #[validate(length(min = 2, max = 200))]
    activity: String,
    #[validate(length(min = 2, max = 200))]
    purpose: String,
    #[validate(length(max = 200))]
    data_categories: Option<String>,
    #[validate(length(max = 120))]
    legal_basis: Option<String>,
    #[validate(length(max = 120))]
    retention: Option<String>,
}
async fn ropa_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<RopaCreateReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .ropa_create(
            &u.claims.org,
            b.activity.trim(),
            b.purpose.trim(),
            b.data_categories.as_deref().unwrap_or(""),
            b.legal_basis.as_deref().unwrap_or("consent"),
            b.retention.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.ropa_created".into(),
            entity: "ropa".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn ropa_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ropa_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({
            "id": r.id, "activity": r.activity, "purpose": r.purpose,
            "data_categories": r.data_categories, "legal_basis": r.legal_basis,
            "retention": r.retention
        }))
        .collect::<Vec<_>>())))
}
async fn ropa_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .ropa_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.ropa_deleted".into(),
            entity: "ropa".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: ROPA ----------

// ---------- compliance slice: UU PDP DPIA + breach log ----------
#[derive(Debug, Deserialize, Validate)]
struct DpiaCreateReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 255))]
    data_types: Option<String>,
    #[validate(length(max = 20))]
    risk_level: Option<String>,
}
async fn dpia_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DpiaCreateReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .dpia_create(
            &u.claims.org,
            b.title.trim(),
            b.data_types.as_deref().unwrap_or(""),
            b.risk_level.as_deref().unwrap_or("medium"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.dpia_created".into(),
            entity: "dpia".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn dpia_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .dpia_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(
            |r| json!({"id": r.id, "title": r.title, "data_types": r.data_types,
                        "risk_level": r.risk_level, "status": r.status})
        )
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct DpiaStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn dpia_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DpiaStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .dpia_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
#[derive(Debug, Deserialize, Validate)]
struct BreachCreateReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 20))]
    discovered: Option<String>,
    affected: Option<i64>,
}
async fn breach_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<BreachCreateReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .breach_create(
            &u.claims.org,
            b.title.trim(),
            b.discovered.as_deref().unwrap_or(""),
            b.affected.unwrap_or(0),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.breach_created".into(),
            entity: "breach".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn breach_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .breach_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(
            |r| json!({"id": r.id, "title": r.title, "discovered": r.discovered_at,
                        "affected": r.affected_count, "status": r.status})
        )
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct BreachStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn breach_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<BreachStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .breach_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
// ---------- end compliance slice: DPIA + breach ----------

// ---------- property slice: PPAT deed register ----------
#[derive(Debug, Deserialize, Validate)]
struct PpatCreateReq {
    #[validate(length(min = 1, max = 60))]
    number: String,
    #[validate(length(min = 1, max = 60))]
    deed_kind: String,
    #[validate(length(max = 200))]
    seller: Option<String>,
    #[validate(length(max = 200))]
    buyer: Option<String>,
    #[validate(length(max = 255))]
    object: Option<String>,
}
async fn ppat_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PpatCreateReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .ppat_create(
            &u.claims.org,
            b.number.trim(),
            b.deed_kind.trim(),
            b.seller.as_deref().unwrap_or(""),
            b.buyer.as_deref().unwrap_or(""),
            b.object.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "ppat.created".into(),
            entity: "ppat".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn ppat_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ppat_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({
            "id": r.id, "number": r.number, "deed_kind": r.deed_kind,
            "seller": r.party_seller, "buyer": r.party_buyer,
            "object": r.object_desc, "status": r.status
        }))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct PpatStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn ppat_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<PpatStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .ppat_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: format!("ppat.{}", b.status.trim()),
            entity: "ppat".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
/// PPAT completeness checklist: required fields per deed kind + flow position.
/// Honest gate before signing (F-D13-003/F-D13-005 evidence).
async fn ppat_checklist(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ppat_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let r = v
        .into_iter()
        .find(|x| x.id == id)
        .ok_or(AppError::NotFound("ppat".into()))?;
    let mut missing: Vec<&str> = Vec::new();
    if r.number.trim().is_empty() {
        missing.push("number");
    }
    if r.party_seller.trim().is_empty() {
        missing.push("seller");
    }
    if r.party_buyer.trim().is_empty() {
        missing.push("buyer");
    }
    if r.object_desc.trim().is_empty() {
        missing.push("object");
    }
    Ok(Json(
        json!({"id": r.id, "status": r.status, "complete": missing.is_empty(), "missing": missing}),
    ))
}
// ---------- end property slice: PPAT ----------

// ---------- approvals: delegation (out-of-office substitutes) ----------
#[derive(Debug, Deserialize, Validate)]
struct DelegReq {
    #[validate(length(min = 1))]
    delegate_uid: String,
    active: Option<bool>,
}
async fn delegation_set(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DelegReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Approve)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let target = s
        .stores
        .users
        .get_user(b.delegate_uid.trim())
        .await
        .map_err(map_store)?
        .ok_or_else(|| AppError::NotFound("user".into()))?;
    if target.org_id != u.claims.org {
        return Err(AppError::NotFound("user".into()));
    }
    let id = s
        .stores
        .ops
        .delegation_set(
            &u.claims.org,
            &u.claims.sub,
            &target.id,
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
            action: "approval.delegated".into(),
            entity: "delegation".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn delegation_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .delegation_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "delegator": r.delegator, "delegate": r.delegate, "active": r.active}))
        .collect::<Vec<_>>())))
}
// ---------- end approvals: delegation ----------

async fn webhook_in(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, AppError> {
    if provider == "tripay" {
        return tripay_webhook(&s, headers, body).await;
    }
    if s.stores.integration_secret.is_empty() {
        log_wh(&s, None, &provider, "", "blocked", "secret not configured").await;
        return Err(AppError::BlockedExternal(
            "integration secret not configured".into(),
        ));
    }
    let sig = match headers.get("x-signature") {
        Some(v) => v,
        None => {
            log_wh(&s, None, &provider, "", "rejected", "missing signature").await;
            return Err(AppError::Unauthorized);
        }
    };
    let sig = match sig.to_str() {
        Ok(v) => v,
        Err(_) => {
            log_wh(
                &s,
                None,
                &provider,
                "",
                "rejected",
                "bad signature encoding",
            )
            .await;
            return Err(AppError::Unauthorized);
        }
    };
    let ts = headers
        .get("x-ts")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if ts.is_empty() {
        log_wh(&s, None, &provider, "", "rejected", "missing ts").await;
        return Err(AppError::Unauthorized);
    }
    if legalrust_integrations::verify_webhook(&s.stores.integration_secret, &body, ts, sig).is_err()
    {
        log_wh(&s, None, &provider, "", "rejected", "bad signature").await;
        return Err(AppError::Unauthorized);
    }
    let event = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("event")
                .or_else(|| v.get("type"))
                .and_then(|x| x.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    log_wh(&s, None, &provider, &event, "accepted", "").await;
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
#[derive(Debug, Deserialize)]
struct WebhookListQ {
    provider: Option<String>,
    status: Option<String>,
}
async fn webhook_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<WebhookListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let mut v = s
        .stores
        .ops
        .webhook_list(&u.claims.org, q.provider.as_deref())
        .await
        .map_err(map_store)?;
    if let Some(st) = q.status.as_deref() {
        v.retain(|r| r.status == st);
    }
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "provider": r.provider, "event": r.event, "status": r.status, "detail": r.detail}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct WebhookEndpointReq {
    #[validate(length(min = 10, max = 500))]
    url: String,
    #[validate(length(max = 255))]
    secret: Option<String>,
    #[validate(length(max = 500))]
    events: Option<String>,
}
async fn webhook_endpoint_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<WebhookEndpointReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .webhook_ep_create(
            &u.claims.org,
            b.url.trim(),
            b.secret.as_deref().unwrap_or(""),
            b.events.as_deref().unwrap_or("*"),
        )
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::Conflict(m) => AppError::Validation(m),
            _ => map_store(e),
        })?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "webhook.endpoint_created".into(),
            entity: "webhook_endpoint".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn webhook_endpoint_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .webhook_ep_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "url": r.url, "events": r.events, "active": r.active}))
        .collect::<Vec<_>>())))
}
async fn webhook_endpoint_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Approve)?;
    s.stores
        .ops
        .webhook_ep_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize)]
struct WebhookTestReq {
    event: Option<String>,
}
async fn webhook_endpoint_test(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<WebhookTestReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let eps = s
        .stores
        .ops
        .webhook_ep_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let ep = eps
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| AppError::NotFound("endpoint".into()))?;
    if !ep.active {
        return Err(AppError::Validation("endpoint disabled".into()));
    }
    let secret = s
        .stores
        .ops
        .webhook_ep_secret(&u.claims.org, id)
        .await
        .map_err(map_store)?
        .unwrap_or_default();
    let event = b.event.clone().unwrap_or_else(|| "ping".into());
    let client = legalrust_integrations::tripay::http_client(15).map_err(|e| match e {
        legalrust_integrations::IntegrationError::Blocked(m) => {
            AppError::Internal(anyhow::anyhow!(m))
        }
        other => AppError::from(other),
    })?;
    let payload = serde_json::json!({
        "event": event,
        "endpoint_id": id,
        "ts": chrono::Utc::now().timestamp(),
    });
    let outcome = legalrust_integrations::post_json(&client, &ep.url, &secret, &payload).await;
    let (status, detail) = match &outcome {
        Ok((code, _)) => ("accepted", format!("HTTP {code}")),
        Err(e) => (
            "rejected",
            e.to_string().chars().take(200).collect::<String>(),
        ),
    };
    let _ = s
        .stores
        .ops
        .webhook_log(Some(&u.claims.org), "outbound", &event, status, &detail)
        .await;
    match outcome {
        Ok((code, _)) => Ok(Json(json!({"accepted": true, "http_status": code}))),
        Err(e) => Err(AppError::from(e)),
    }
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
    // Preference center (F-D18-006): opted-out kinds are suppressed, not sent.
    let prefs = s.stores.ops.notif_pref_list(user).await.unwrap_or_default();
    if prefs.iter().any(|(k, e)| k == kind && !e) {
        audit(
            &s.pool,
            &s.stores,
            AuditEvent {
                tenant_id: org.to_string(),
                actor_id: Some(user.to_string()),
                action: format!("notify.{kind}.suppressed"),
                entity: "notification".into(),
                entity_id: 0,
                ip: None,
            },
        )
        .await;
        return;
    }
    // Flood guard (F-D18-029): per-user cap on the notify path (20/min).
    if !s.stores.limiter.check(&format!("notify:{user}")) {
        audit(
            &s.pool,
            &s.stores,
            AuditEvent {
                tenant_id: org.to_string(),
                actor_id: Some(user.to_string()),
                action: format!("notify.{kind}.rate_limited"),
                entity: "notification".into(),
                entity_id: 0,
                ip: None,
            },
        )
        .await;
        return;
    }
    let nid = s
        .stores
        .ops
        .notify(org, user, kind, title, None)
        .await
        .unwrap_or(0);
    // Notification audit trail: every emitted notification is recorded so
    // delivery can be traced per tenant (F-D18-028).
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: org.to_string(),
            actor_id: Some(user.to_string()),
            action: format!("notify.{kind}.sent"),
            entity: "notification".into(),
            entity_id: nid,
            ip: None,
        },
    )
    .await;
    let _ = s.stores.events.send(format!("notify:{kind}"));
    // Outbound webhook fan-out (best-effort, never blocks the request):
    // endpoints subscribed to this kind (or '*') get a signed JSON ping
    // immediately; failures are queued as durable retry jobs (worker sends
    // with backoff, dead-letters after 5 attempts). Every attempt is logged.
    let st = s.clone();
    let org = org.to_string();
    let kind = kind.to_string();
    let title = title.to_string();
    tokio::spawn(async move {
        let eps = match st.stores.ops.webhook_ep_list(&org).await {
            Ok(v) => v,
            Err(_) => return,
        };
        let targets: Vec<_> = eps
            .into_iter()
            .filter(|e| {
                e.active
                    && (e.events.trim() == "*"
                        || e.events.split(',').map(str::trim).any(|x| x == kind))
            })
            .collect();
        if targets.is_empty() {
            return;
        }
        let client = match legalrust_integrations::tripay::http_client(10) {
            Ok(c) => c,
            Err(_) => return,
        };
        for ep in targets {
            let secret = st
                .stores
                .ops
                .webhook_ep_secret(&org, ep.id)
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let payload = serde_json::json!({
                "event": kind,
                "endpoint_id": ep.id,
                "title": title,
            });
            match legalrust_integrations::post_json(&client, &ep.url, &secret, &payload).await {
                Ok((code, _)) => {
                    let _ = st
                        .stores
                        .ops
                        .webhook_log(
                            Some(&org),
                            "outbound",
                            &kind,
                            "accepted",
                            &format!("HTTP {code}"),
                        )
                        .await;
                }
                Err(e) => {
                    // Immediate send failed: queue a durable retry (worker
                    // retries with backoff, dead-letters after 5 attempts).
                    let payload = serde_json::json!({
                        "endpoint_id": ep.id,
                        "org": org,
                        "event": kind,
                    })
                    .to_string();
                    let delay = legalrust_integrations::webhook_backoff_secs(1);
                    let _ = st
                        .stores
                        .ops
                        .job_enqueue_at(&org, "webhook.dispatch", &payload, delay)
                        .await;
                    let detail: String = format!("retry queued in {delay}s: {e}")
                        .chars()
                        .take(200)
                        .collect();
                    let _ = st
                        .stores
                        .ops
                        .webhook_log(Some(&org), "outbound", &kind, "rejected", &detail)
                        .await;
                }
            }
        }
    });
}

#[derive(Debug, Deserialize, Validate)]
struct TokenReq {
    #[validate(length(min = 2))]
    name: String,
    expires_in_days: Option<i64>,
}
async fn token_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<TokenReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let days = b.expires_in_days.unwrap_or(90);
    if !(1..=365).contains(&days) {
        return Err(AppError::Validation("expires_in_days".into()));
    }
    let raw = format!("lr_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let hash = legalrust_document::sha256_hex(raw.as_bytes());
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::days(days);
    let id = s
        .stores
        .users
        .create_token(&u.claims.sub, &b.name, &hash, Some(expires_at))
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"id": id, "token": raw, "expires_at": expires_at.format("%Y-%m-%d %H:%M:%S").to_string()}),
    ))
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
        .map(|t| json!({"id": t.id, "name": t.name, "expires_at": t.expires_at}))
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
    let target = s
        .stores
        .users
        .get_user(&uid)
        .await
        .map_err(map_store)?
        .ok_or_else(|| AppError::NotFound("user".into()))?;
    // Cross-tenant role grants are forbidden: the target must belong to the
    // caller's org (IDOR hardening, F-D16-029).
    if target.org_id != u.claims.org {
        return Err(AppError::NotFound("user".into()));
    }
    s.stores
        .users
        .assign_role(&u.claims.org, &uid, role.as_str())
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: format!("user.role_assigned:{}", role.as_str()),
            entity: "user".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
    let roles = s.stores.users.user_roles(&uid).await.map_err(map_store)?;
    Ok(Json(json!({"user_id": uid, "roles": roles})))
}
async fn user_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Assign)?;
    let users = s
        .stores
        .users
        .list_users(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut out = Vec::new();
    for x in users {
        let roles = s.stores.users.user_roles(&x.id).await.map_err(map_store)?;
        out.push(
            json!({"id": x.id, "email": x.email, "name": x.name, "status": x.status, "roles": roles}),
        );
    }
    Ok(Json(json!(out)))
}

#[derive(Debug, Deserialize)]
struct NotifyListQ {
    kind: Option<String>,
    unread_only: Option<bool>,
    limit: Option<i64>,
}
async fn notify_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<NotifyListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    if let Some(k) = q.kind.as_deref() {
        if k.trim().is_empty() || k.len() > 64 {
            return Err(AppError::Validation("kind".into()));
        }
    }
    let v = s
        .stores
        .ops
        .notify_list(
            &u.claims.sub,
            q.kind.as_deref(),
            q.unread_only.unwrap_or(false),
            q.limit.unwrap_or(50).clamp(1, 100),
        )
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
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "notification.read".into(),
            entity: "notification".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"read": true})))
}
/// Notification preference center (F-D18-006): per-user kind toggles.
#[derive(Debug, Deserialize, Validate)]
struct NotifPrefReq {
    #[validate(length(min = 1, max = 64))]
    kind: String,
    enabled: bool,
}
async fn notif_pref_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let v = s
        .stores
        .ops
        .notif_pref_list(&u.claims.sub)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|(k, e)| json!({"kind": k, "enabled": e}))
        .collect::<Vec<_>>())))
}
async fn notif_pref_set(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<NotifPrefReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .notif_pref_set(&u.claims.sub, b.kind.trim(), b.enabled)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"kind": b.kind.trim(), "enabled": b.enabled})))
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
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.matters_exported".into(),
            entity: "report".into(),
            entity_id: 0,
            ip: None,
        },
    )
    .await;
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
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "trust.transfer".into(),
            entity: "trust".into(),
            entity_id: a.id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"out": a.id, "in": x.id})))
}

// ---------- approvals engine (generic sequential multi-step) ----------
#[derive(Debug, Deserialize)]
struct ApprStepIn {
    step_no: i64,
    role: String,
}
#[derive(Debug, Deserialize)]
struct ApprCreateReq {
    entity: String,
    entity_id: i64,
    title: String,
    steps: Vec<ApprStepIn>,
}
fn appr_step_row_json(x: &legalrust_database::ApprovalStepRow) -> Value {
    json!({"id": x.id, "step_no": x.step_no, "role": x.role, "status": x.status, "comment": x.comment})
}
async fn appr_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ApprCreateReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    if b.title.trim().is_empty() || b.title.len() > 200 {
        return Err(AppError::Validation("title".into()));
    }
    if b.steps.is_empty() || b.steps.len() > 20 {
        return Err(AppError::Validation("steps".into()));
    }
    if b.steps.iter().any(|x| x.role.trim().is_empty()) {
        return Err(AppError::Validation("role".into()));
    }
    // Referenced entity must exist (no dangling approvals).
    match b.entity.as_str() {
        "matter" => {
            s.stores
                .core
                .get_matter(&u.claims.org, b.entity_id)
                .await
                .map_err(|_| AppError::NotFound("matter".into()))?;
        }
        "invoice" => {
            s.stores
                .ops
                .inv_get(&u.claims.org, b.entity_id)
                .await
                .map_err(|_| AppError::NotFound("invoice".into()))?;
        }
        "deed" => {
            let ds = s
                .stores
                .ops
                .deed_list(&u.claims.org)
                .await
                .map_err(map_store)?;
            if !ds.iter().any(|d| d.id == b.entity_id) {
                return Err(AppError::NotFound("deed".into()));
            }
        }
        _ => return Err(AppError::Validation("entity".into())),
    }
    let steps: Vec<(i64, String)> = b
        .steps
        .iter()
        .map(|x| (x.step_no, x.role.trim().to_string()))
        .collect();
    let id = s
        .stores
        .ops
        .appr_create(
            &u.claims.org,
            b.entity.trim(),
            b.entity_id,
            b.title.trim(),
            &u.claims.sub,
            &steps,
        )
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::Conflict(m) => AppError::Validation(m),
            _ => map_store(e),
        })?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "approval.created".into(),
            entity: "approval".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    notify_user(
        &s,
        &u.claims.org,
        &u.claims.sub,
        "approval",
        &format!("Approval requested: {}", b.title.trim()),
    )
    .await;
    Ok(Json(json!({"id": id, "status": "pending"})))
}
#[derive(Debug, Deserialize)]
struct ApprListQ {
    status: Option<String>,
}
async fn appr_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<ApprListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    if let Some(st) = q.status.as_deref() {
        if !["pending", "approved", "rejected", "needs_rework", "revoked"].contains(&st) {
            return Err(AppError::Validation("status".into()));
        }
    }
    let v = s
        .stores
        .ops
        .appr_list(&u.claims.org, q.status.as_deref())
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "entity": r.entity, "entity_id": r.entity_id, "title": r.title, "status": r.status}))
        .collect::<Vec<_>>())))
}
async fn appr_get(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (r, steps) = s
        .stores
        .ops
        .appr_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({
        "id": r.id, "entity": r.entity, "entity_id": r.entity_id,
        "title": r.title, "status": r.status,
        "steps": steps.iter().map(appr_step_row_json).collect::<Vec<_>>(),
    })))
}
#[derive(Debug, Deserialize)]
struct ApprDecideReq {
    step_no: i64,
    decision: String,
    comment: Option<String>,
}
/// Decider must hold Approve AND (the step role or an admin/owner role).
fn appr_may_decide(user: &AuthUser, step_role: &str) -> bool {
    user.claims.roles.iter().any(|r| {
        let r = r.trim();
        r == step_role
            || matches!(
                legalrust_domain::Role::parse(r).map(|x| x.as_str()),
                Some("super_admin") | Some("org_owner") | Some("office_admin")
            )
    })
}
/// Delegated authority: an active delegate may decide steps any of their
/// delegators could decide (F-D09-003/F-D09-004).
async fn appr_may_decide_delegated(s: &FullState, user: &AuthUser, step_role: &str) -> bool {
    let tors = match s
        .stores
        .ops
        .delegation_delegators(&user.claims.org, &user.claims.sub)
        .await
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    if tors.is_empty() {
        return false;
    }
    for d in tors {
        let roles = match s.stores.users.user_roles(&d).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        if roles.iter().any(|r| {
            let r = r.trim();
            r == step_role
                || matches!(
                    legalrust_domain::Role::parse(r).map(|x| x.as_str()),
                    Some("super_admin") | Some("org_owner") | Some("office_admin")
                )
        }) {
            return true;
        }
    }
    false
}
async fn appr_decide(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<ApprDecideReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    if let Some(c) = b.comment.as_deref() {
        if c.len() > 2000 {
            return Err(AppError::Validation("comment".into()));
        }
    }
    let (_, steps) = s
        .stores
        .ops
        .appr_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    let step = steps
        .iter()
        .find(|x| x.step_no == b.step_no)
        .ok_or_else(|| AppError::NotFound("step".into()))?;
    // Delegated substitutes bypass both the Approve gate and the role gate.
    if !appr_may_decide_delegated(&s, &u, &step.role).await {
        authorize(&u, Permission::Approve)?;
        if !appr_may_decide(&u, &step.role) {
            return Err(AppError::Forbidden);
        }
    }
    let done = s
        .stores
        .ops
        .appr_decide(
            &u.claims.org,
            id,
            b.step_no,
            &u.claims.sub,
            b.decision.trim(),
            b.comment.as_deref(),
        )
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::Conflict(m) => {
                if m == "decision" {
                    AppError::Validation(m)
                } else {
                    AppError::Conflict(m)
                }
            }
            _ => map_store(e),
        })?;
    let (req, _) = s
        .stores
        .ops
        .appr_get(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: format!("approval.{}", done.status),
            entity: "approval".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    notify_user(
        &s,
        &u.claims.org,
        &u.claims.sub,
        "approval",
        &format!("Approval #{} step {}: {}", id, b.step_no, done.status),
    )
    .await;
    Ok(Json(
        json!({"step_no": done.step_no, "status": done.status, "request_status": req.status}),
    ))
}
async fn appr_revoke(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Approve)?;
    s.stores
        .ops
        .appr_revoke(&u.claims.org, id, &u.claims.sub)
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "approval.revoked".into(),
            entity: "approval".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"revoked": true})))
}
// ---------- end approvals engine ----------

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
        "mp4" => "video/mp4",
        "json" => "application/manifest+json",
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

/// CORS allowlist from `CORS_ORIGINS` (comma-separated exact origins).
/// Empty/unset = same-origin only (browser cross-origin calls rejected).
/// Auth uses Bearer tokens (no cookies), so this is defense-in-depth.
fn cors_layer() -> CorsLayer {
    use axum::http::{HeaderValue, Method};
    let origins: Vec<HeaderValue> = std::env::var("CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
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
        r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{t} — LegalRust</title><meta name="description" content="{d}"><link rel="canonical" href="{b}{p}"><meta property="og:title" content="{t} — LegalRust"><meta property="og:description" content="{d}"><meta property="og:type" content="website"><meta property="og:url" content="{b}{p}"><link rel="manifest" href="/static/manifest.json"><meta name="theme-color" content="#1D5BD8"><link rel="apple-touch-icon" href="/static/hero-office.png"><link rel="stylesheet" href="/static/vendor/tabler.min.css?v=2"><link rel="stylesheet" href="/static/vendor/tabler-fonts/tabler-icons.min.css?v=2"><link rel="stylesheet" href="/static/site.css?v=2"><script type="application/ld+json">{{"@context":"https://schema.org","@type":"Organization","name":"LegalRust","url":"{b}","description":"Platform manajemen hukum untuk notaris, PPAT, firma hukum, dan profesional legal."}}</script></head><body><a class="skip-link" href="#main">Lewati ke konten</a><div class="container-xl"><div class="topbar"><div class="logo">LegalRust<small>Smart Legal Solution</small></div>{n}<div class="topact"><a href="/login" class="btn-line sm">Masuk</a><a href="/login" class="btn-pri sm">Coba Gratis</a></div></div></div><main class="container-xl page-main" id="main" tabindex="-1">{body}</main><div class="container-xl"><footer class="site"><span>2026 LegalRust — Smart Legal Solution</span><span><a href="/kebijakan-privasi">Privasi</a> · <a href="/syarat-ketentuan">Syarat</a> · <a href="/faq">FAQ</a> · <a href="/sitemap.xml">Peta Situs</a> · <a href="/healthz">Status</a></span></footer></div><script src="/static/vendor/tabler.min.js?v=2"></script><script src="/static/site.js?v=2"></script></body></html>"##,
        t = esc(title),
        d = esc(desc),
        b = SITE_BASE,
        p = path,
        n = nav,
        body = body,
    )
}

fn app_shell(title: &str, body: &str) -> String {
    format!(
        r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{t} — LegalRust App</title><meta name="robots" content="noindex,nofollow"><link rel="manifest" href="/static/manifest.json"><meta name="theme-color" content="#1D5BD8"><link rel="apple-touch-icon" href="/static/hero-office.png"><link rel="stylesheet" href="/static/vendor/tabler.min.css?v=2"><link rel="stylesheet" href="/static/vendor/tabler-fonts/tabler-icons.min.css?v=2"><link rel="stylesheet" href="/static/site.css?v=2"></head><body><a class="skip-link" href="#main">Lewati ke konten utama</a>{b}<script src="/static/vendor/tabler.min.js?v=2"></script><script src="/static/site.js?v=2"></script></body></html>"##,
        t = title,
        b = body,
    )
}

fn auth_shell(title: &str, desc: &str, body: &str) -> String {
    format!(
        r##"<!doctype html><html lang="id"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{t} — LegalRust</title><meta name="description" content="{d}"><meta name="robots" content="noindex,nofollow"><link rel="manifest" href="/static/manifest.json"><meta name="theme-color" content="#1D5BD8"><link rel="apple-touch-icon" href="/static/hero-office.png"><link rel="stylesheet" href="/static/vendor/tabler.min.css?v=2"><link rel="stylesheet" href="/static/vendor/tabler-fonts/tabler-icons.min.css?v=2"><link rel="stylesheet" href="/static/site.css?v=2"></head><body><a class="skip-link" href="#main">Lewati ke konten</a><div class="container-xl"><div class="topbar"><div class="logo">LegalRust<small>Smart Legal Solution</small></div><div class="topact"><a href="/" class="btn-line sm">← Beranda</a></div></div></div><main class="container-xl" id="main" tabindex="-1">{b}</main><div class="container-xl"><footer class="site"><span>2026 LegalRust</span><span><a href="/healthz">Status</a></span></footer></div><script src="/static/vendor/tabler.min.js?v=2"></script><script src="/static/site.js?v=2"></script></body></html>"##,
        t = esc(title),
        d = esc(desc),
        b = body,
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
async fn landing(State(s): State<FullState>) -> impl IntoResponse {
    let faqs = s.stores.mkt.mkt_faq_list().await.unwrap_or_default();
    let mut top: Vec<(String, String)> = faqs
        .into_iter()
        .take(6)
        .map(|f| (f.question, f.answer))
        .collect();
    if top.is_empty() {
        top = vec![
            (
                "Apakah data kantor kami tercampur dengan organisasi lain?".into(),
                "Tidak. Seluruh data tersegregasi per organisasi dan setiap endpoint dijaga RBAC.".into(),
            ),
            (
                "Bagaimana kata sandi dan sesi diamankan?".into(),
                "Kata sandi di-hash Argon2, sesi JWT dapat dicabut, dan brute-force dikunci setelah 5x percobaan dalam 5 menit.".into(),
            ),
            (
                "Fitur apa saja yang belum aktif (501)?".into(),
                "OCR native live bila tesseract terinstall, AI chat live bila endpoint menjawab, Tripay live bila kredensial diisi; yang masih 501: e-sign PSrE. Webhook butuh signature masing-masing provider.".into(),
            ),
            (
                "Apakah LegalRust sudah production-ready?".into(),
                "Belum. Status uji internal 31/32; OCR+AI live di host dev, Tripay wired, menunggu counsel sign-off dan lisensi deployment.".into(),
            ),
            (
                "Berapa lama tautan share dokumen berlaku?".into(),
                "Tautan share berlaku 1-720 jam, dapat dicabut kapan saja, dan unduhan dijaga RBAC.".into(),
            ),
            (
                "Apakah pembayaran online tersedia?".into(),
                "Belum. Pembayaran manual anti-overpay dan trust ledger double-entry live; gateway pembayaran masih 501.".into(),
            ),
        ];
    }
    let mut faq_html = String::from(r#"<div class="faq-list">"#);
    for (q, a) in &top {
        faq_html.push_str(&format!(
            "<details><summary>{}</summary><p>{}</p></details>",
            esc(q),
            esc(a)
        ));
    }
    faq_html.push_str("</div>");
    let body = r##"<section class="hero" id="hero"><div class="hero-in"><div><span class="badge-soft rv">Solusi LegalTech Terdepan di Indonesia</span><h1 class="rv">Kelola Perkara, Dokumen, dan Akta dalam Satu Platform Aman</h1><p class="lead rv">LegalRust membantu notaris, PPAT, dan firma hukum mengelola perkara, dokumen berversi dengan pin SHA-256, akta dari template dengan alur ber-guard, verifikasi identitas via OCR job plus review manusia, serta keuangan invoice anti-overpay dan trust ledger double-entry.</p><div class="cta-row"><a class="a" href="/app">Lihat Demo</a><a class="b" href="/login">Mulai Gratis Sekarang</a></div><div class="checks rv"><span><i class="tick">v</i>Gratis Selamanya</span><span><i class="tick">v</i>Aman dan Terpercaya</span><span><i class="tick">v</i>Jejak Audit Penuh</span></div><p class="honest rv">Status jujur: uji internal 31/32 — bukan production-ready. OCR native (tesseract ind+eng) live bila terinstall di host, AI chat live bila endpoint menjawab (default Ollama lokal), Tripay live bila kredensial diisi; ketiganya 501 bila tidak. Endpoint 501: POST /api/v1/deeds/esign. Yang live: OCR gambar, AI chat, RAG keyword, meteran token, pembayaran manual + Tripay, dan trust ledger.</p></div><div><video class="hero-photo rv" autoplay muted loop playsinline preload="metadata" poster="/static/hero-office.png" aria-label="Sinematik gedung kantor modern — ilustrasi"><source src="/static/hero-loop.mp4" type="video/mp4"></video></div></div></section><section class="showcase" id="dasbor"><div class="split"><div class="txt"><span class="kicker">DASBOR OPERASIONAL</span><h2>Pratinjau dasbor: Ringkasan, Dokumen, Akta, Keuangan</h2><p class="lead2">Tab Ringkasan menampilkan tabel Perkara Aktif; modul Dokumen, Akta, dan Keuangan masing-masing punya tab sendiri di aplikasi. Angka pada mockup adalah ilustrasi tampilan, bukan data produksi.</p><div class="cta-row"><a class="a" href="/app">Buka Aplikasi</a><a class="b" href="/login">Masuk</a></div></div><div><div class="mock rv"><div class="mock-top"><span class="dot"></span><span class="dot"></span><span class="dot"></span><span>LegalRust — pratinjau dasbor (ilustrasi)</span></div><div class="mock-body"><div class="side"><div class="logo2">LegalRust</div><div class="mi on">Dasbor</div><div class="mi">Perkara</div><div class="mi">Dokumen</div><div class="mi">Akta</div><div class="mi">Keuangan</div></div><div class="dmain"><div class="msearch"><span class="box">Cari perkara, klien, atau dokumen</span><span class="me"><span class="av">B</span><span><b>Budi Santoso</b><br><small>Notaris</small></span></span></div><div class="stat4"><div class="stat">Total Perkara<b>128</b><span class="up">+ 12,5%</span></div><div class="stat">Dokumen Diproses<b>842</b><span class="up">+ 18,7%</span></div><div class="stat">Klien Aktif<b>256</b><span class="up">+ 8,2%</span></div><div class="stat">Pendapatan<b>Rp 248 jt</b><span class="up">+ 14,6%</span></div></div></div></div></div></div></div></section><div class="trust rv" id="kepercayaan"><div><b>Dirancang untuk</b><br>Praktik Hukum Indonesia</div><span class="seg">NOTARIS</span><span class="seg">PPAT</span><span class="seg">FIRMA HUKUM</span><span class="seg">KORPORAT</span><span class="seg">ADVOKAT</span></div><section class="feat" id="fitur"><span class="badge-soft tag rv">FITUR UNGGULAN</span><h2 class="rv">Semua yang Anda Butuhkan dalam Satu Platform</h2><p class="sub rv">Enam kemampuan inti — setiap kartu menaut ke penjelasan jujur di bawah.</p><div class="fgrid"><div class="fcard rv"><div class="ic blue"><i class="ti ti-briefcase" aria-hidden="true"></i></div><h3>Manajemen Perkara Terintegrasi</h3><p>Klien, perkara, dan tugas bertransisi aman. <a href="#perkara">Lihat showcase</a></p></div><div class="fcard rv"><div class="ic purple"><i class="ti ti-files" aria-hidden="true"></i></div><h3>Dokumen Berversi + Share Kedaluwarsa</h3><p>Upload SHA-pin, versi, tautan 1-720 jam. <a href="#dokumen">Lihat showcase</a></p></div><div class="fcard rv"><div class="ic green"><i class="ti ti-scan" aria-hidden="true"></i></div><h3>OCR + Review Manusia</h3><p>Job OCR per tipe dokumen plus keputusan approve/reject. <a href="#ocr">Lihat showcase</a></p></div><div class="fcard rv"><div class="ic orange"><i class="ti ti-file-text" aria-hidden="true"></i></div><h3>Akta dari Template Ber-guard</h3><p>Render variabel dan state machine anti-transisi ilegal. <a href="#akta">Lihat showcase</a></p></div><div class="fcard rv"><div class="ic purple"><i class="ti ti-report" aria-hidden="true"></i></div><h3>Laporan CSV + Pencarian RAG</h3><p>Ekspor CSV perkara dan pencarian keyword per organisasi. <a href="#pelaporan">Lihat showcase</a></p></div><div class="fcard rv"><div class="ic green2"><i class="ti ti-shield-check" aria-hidden="true"></i></div><h3>Keamanan &amp; Audit</h3><p>Argon2, JWT revocable, RBAC per endpoint, audit trail. <a href="#keamanan">Lihat showcase</a></p></div></div></section><section class="showcase alt" id="perkara"><div class="split"><div class="txt"><span class="stbadge st-ok">TERSEDIA</span><br><span class="kicker">PERKARA DAN CRM</span><h2>Klien, perkara, dan tugas dalam satu alur teraudit</h2><p class="lead2">Daftarkan klien individual, korporat, atau pemerintah, buka perkara bernomor otomatis, lalu kelola tugas dengan transisi status yang dijaga — lompatan ilegal ditolak dengan 409. Setiap pembuatan perkara memicu notifikasi in-app.</p><ul class="flow"><li><b>POST /api/v1/clients</b> — buat klien dengan jenis terverifikasi</li><li><b>POST /api/v1/matters</b> — buat perkara bernomor otomatis</li><li><b>POST /api/v1/tasks</b> — tambah dan transisikan tugas dengan aman</li></ul><div class="cta-row"><a class="a" href="/app">Kelola di /app</a><a class="b" href="/fitur/manajemen-perkara">Detail Fitur</a></div></div><div><img class="mini-img" src="/static/office-interior.png" alt="Interior kantor hukum tempat tim mengelola perkara" loading="lazy"><div class="mini"><div class="mhead">Perkara Aktif — ilustrasi tampilan</div><div class="table-responsive"><table><thead><tr><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody><tr><td>M-2026-001A</td><td>Akta Jual Beli — Cibubur</td><td>aktif</td></tr><tr><td>M-2026-002F</td><td>Penetapan Ahli Waris</td><td>berjalan</td></tr></tbody></table></div><div class="cap">Data contoh untuk ilustrasi; bukan data produksi.</div></div></div></div></section><section class="showcase" id="dokumen"><div class="split flip"><div class="txt"><span class="stbadge st-ok">TERSEDIA</span><br><span class="kicker">DOKUMEN BERVERSI</span><h2>Unggah, versikan, pin SHA-256, bagikan</h2><p class="lead2">Isi dokumen dipin dengan hash SHA-256 — ukuran dan hash harus cocok atau unggahan ditolak. Setiap versi tersimpan, dan tautan share kedaluwarsa dalam 1-720 jam serta dapat dicabut.</p><ol class="flow"><li><b>1. Unggah</b> — registrasi metadata plus hash SHA-256</li><li><b>2. Versi</b> — setiap unggahan tercatat sebagai versi baru</li><li><b>3. Verifikasi SHA-256</b> — mismatch ditolak, bukan diperbaiki diam-diam</li><li><b>4. Bagikan</b> — tautan kedaluwarsa 1-720 jam, revocable</li></ol><div class="cta-row"><a class="a" href="/app">Kelola di /app</a><a class="b" href="/fitur/manajemen-dokumen">Detail Fitur</a></div></div><div><img class="mini-img" src="/static/doc-scan.png" alt="Ilustrasi pemindaian dokumen berversi" loading="lazy"><div class="mini"><div class="mhead">Riwayat versi — ilustrasi tampilan</div><div class="table-responsive"><table><thead><tr><th>Versi</th><th>SHA-256</th><th>Status</th></tr></thead><tbody><tr><td>v2</td><td>9f2c…a41b</td><td>terkini</td></tr><tr><td>v1</td><td>77d0…c902</td><td>arsip</td></tr></tbody></table></div><div class="cap">Hash disingkat untuk ilustrasi.</div></div></div></div></section><section class="showcase alt" id="ocr"><div class="split"><div class="txt"><span class="stbadge st-cfg">BUTUH KONFIGURASI</span><br><span class="kicker">OCR + REVIEW MANUSIA</span><h2>Antrean OCR dan keputusan manusia yang teraudit</h2><p class="lead2">Job OCR dibuat per tipe dokumen dan diputuskan lewat review approve/reject oleh pemegang permission Verify. Kami tidak mengklaim akurasi otomatis apa pun: eksekusi OCR live butuh engine yang dikonfigurasi.</p><div class="chips"><span class="chip">KTP</span><span class="chip">KK</span><span class="chip">NPWP</span><span class="chip">Paspor</span><span class="chip">Sertifikat Tanah</span></div><div class="conf"><div class="lab"><span>NIK — contoh tampilan</span><span>0,95</span></div><div class="bar"><i class="w95"></i></div></div><div class="conf"><div class="lab"><span>Nama — contoh tampilan</span><span>0,88</span></div><div class="bar"><i class="w88"></i></div></div><div class="conf"><div class="lab"><span>Alamat — contoh tampilan</span><span>0,72</span></div><div class="bar"><i class="w72"></i></div></div><p class="fineprint">Skor di atas contoh tampilan antarmuka, bukan klaim akurasi. Endpoint POST /api/v1/ocr/jobs/process mengembalikan 501 hingga libtesseract native atau cloud fallback dikonfigurasi; job tetap antre, tidak ada yang diproses diam-diam.</p><div class="cta-row"><a class="a" href="/fitur/ocr">Detail Fitur</a></div></div><div><img class="mini-img" src="/static/ocr-scan.png" alt="Ilustrasi pemindaian OCR dokumen identitas" loading="lazy"><div class="mini"><div class="mhead">Review manusia — ilustrasi tampilan</div><div class="table-responsive"><table><thead><tr><th>Field</th><th>Nilai</th><th>Keputusan</th></tr></thead><tbody><tr><td>NIK</td><td>3174…0012</td><td>approve</td></tr><tr><td>Nama</td><td>erran…</td><td>reject</td></tr></tbody></table></div><div class="cap">Keputusan review butuh permission Verify.</div></div></div></div></section><section class="showcase" id="akta"><div class="split flip"><div class="txt"><span class="stbadge st-ltd">TERSEDIA TERBATAS</span><br><span class="kicker">GENERASI AKTA</span><h2>Dari template berversi ke signing bertahap</h2><p class="lead2">Template berversi di-render dengan substitusi variabel, lalu akta berjalan lewat state machine ber-guard: transisi ilegal ditolak dengan 409. Langkah signing menunggu provider PSrE dan tidak pernah diklaim bekerja.</p><ol class="stepper"><li><b>Template</b> — buat dan versikan template akta</li><li><b>Render</b> — substitusi variabel menjadi draf akta</li><li><b>Collection</b> — kumpulkan dokumen pendukung</li><li><b>Verifikasi</b> — periksa kelengkapan berkas</li><li><b>Review</b> — telaah dan koreksi draf</li><li><b>Approval</b> — setujui untuk signing</li><li class="pend"><b>Signing</b> — menunggu PSrE (501 hingga dikonfigurasi)</li></ol><div class="cta-row"><a class="a" href="/app">Kelola di /app</a><a class="b" href="/fitur/generasi-akta">Detail Fitur</a></div></div><div><img class="mini-img" src="/static/signature.png" alt="Ilustrasi penandatanganan akta" loading="lazy"><div class="mini"><div class="mhead">Status akta — ilustrasi tampilan</div><div class="table-responsive"><table><thead><tr><th>Nomor</th><th>Tahap</th></tr></thead><tbody><tr><td>D-2026-014</td><td>review</td></tr><tr><td>D-2026-015</td><td>approved</td></tr></tbody></table></div><div class="cap">Tahap mengikuti 10 state backend, bukan slug tampilan.</div></div></div></div></section><section class="showcase alt" id="keuangan"><div class="split"><div class="txt"><span class="stbadge st-ltd">TERSEDIA TERBATAS</span><br><span class="kicker">KEUANGAN DAN TRUST</span><h2>Invoice anti-overpay dan trust double-entry</h2><p class="lead2">Invoice per matter dibayar manual dengan guard anti-overpay: pembayaran melebihi sisa tagihan ditolak dengan 409. Dana klien dicatat di trust ledger double-entry anti-negatif. Gateway pembayaran online masih menunggu konfigurasi.</p><div class="mini"><div class="mhead">Invoice — ilustrasi tampilan</div><div class="table-responsive"><table><thead><tr><th>Total</th><th>Dibayar</th><th>Sisa</th></tr></thead><tbody><tr><td>Rp 1.000.000</td><td>Rp 400.000</td><td>Rp 600.000</td></tr></tbody></table></div><div class="cap">Overpay 700.000 pada sisa 600.000 ditolak (409).</div></div><div class="mini"><div class="mhead">Transfer trust — ilustrasi tampilan</div><div class="table-responsive"><table><thead><tr><th>Dari</th><th>Ke</th><th>Nominal</th></tr></thead><tbody><tr><td>Trust-A</td><td>Trust-B</td><td>Rp 400.000</td></tr></tbody></table></div><div class="cap">Saldo tidak cukup membuat transfer ditolak (409).</div></div><div class="cta-row"><a class="a" href="/app">Kelola di /app</a></div></div><div><img class="mini-img" src="/static/courthouse.png" alt="Gedung pengadilan sebagai simbol kepastian transaksi" loading="lazy"><p class="fineprint">Jujur: POST /api/v1/invoices/pay-gateway mengembalikan 501 hingga billing gateway dikonfigurasi. Notifikasi in-app dan event SSE memberi tahu setiap invoice terbit.</p></div></div></section><section class="showcase" id="keamanan"><div class="split flip"><div class="txt"><span class="stbadge st-ok">TERSEDIA</span><br><span class="kicker">KEAMANAN DAN AUDIT</span><h2>Kontrol akses per endpoint dan jejak audit</h2><p class="lead2">Sandi Argon2, sesi JWT revocable plus token API, lockout brute-force 5x per 5 menit, rate-limit, header CSP dan HSTS, RBAC per endpoint, dan audit trail yang hanya dapat dibaca peran terotorisasi.</p><div class="mini"><div class="mhead">Peran dan hak akses ringkas</div><div class="table-responsive"><table><thead><tr><th>Peran</th><th>Hak akses</th></tr></thead><tbody><tr><td>org_owner</td><td>semua hak akses</td></tr><tr><td>office_admin</td><td>kelola harian plus Assign</td></tr><tr><td>notary / ppat</td><td>akta: Sign, Verify, Approve</td></tr><tr><td>lawyer</td><td>perkara dan dokumen</td></tr><tr><td>finance_officer</td><td>keuangan: Approve</td></tr><tr><td>auditor</td><td>baca audit trail</td></tr></tbody></table></div></div><p class="fineprint">Belum ada endpoint backup otomatis; gunakan ekspor CSV perkara plus audit trail sebagai jejak pemulihan operasional.</p></div><div><img class="mini-img" src="/static/security.png" alt="Ilustrasi lapisan keamanan berlapis LegalRust" loading="lazy"></div></div></section><section class="showcase alt" id="pelaporan"><div class="split"><div class="txt"><span class="stbadge st-ltd">TERSEDIA TERBATAS</span><br><span class="kicker">PELAPORAN DAN PENCARIAN</span><h2>Ekspor CSV dan pencarian keyword RAG</h2><p class="lead2">Unduh perkara sebagai CSV untuk pelaporan, indeks dokumen teks menjadi chunk pencarian, lalu cari dengan pencarian keyword full-text per organisasi. Tanpa klaim analitik dan tanpa chatbot.</p><div class="mini"><div class="mhead">GET /api/v1/reports/matters.csv — contoh</div><div class="table-responsive"><table><thead><tr><th>id</th><th>number</th><th>title</th><th>status</th></tr></thead><tbody><tr><td>1</td><td>M-AB12CD34</td><td>Akta Jual Beli</td><td>aktif</td></tr></tbody></table></div></div><div class="mini"><div class="mhead">GET /api/v1/rag/search?q=sertifikat — contoh</div><div class="table-responsive"><table><thead><tr><th>doc_id</th><th>potongan</th></tr></thead><tbody><tr><td>7</td><td>penjual menyerahkan sertifikat…</td></tr></tbody></table></div></div><p class="fineprint">Jujur: POST /api/v1/ai/chat mengembalikan 501 — belum ada provider AI. Yang live hanya indeks dan pencarian RAG plus pencatatan token di GET /api/v1/ai/usage.</p></div><div><img class="mini-img" src="/static/ai-assist.png" alt="Ilustrasi pencarian dokumen berbantuan indeks RAG" loading="lazy"></div></div></section><section class="feat" id="alur"><span class="badge-soft tag rv">CARA KERJA</span><h2 class="rv">Empat langkah dari daftar hingga laporan</h2><p class="sub rv">Alur yang sama untuk semua profesi hukum.</p><div class="steps4"><div class="step4 rv"><span class="n">1</span><h3>Daftar dan masuk</h3><p>Buat organisasi, terima sesi JWT revocable, atur peran tim.</p></div><div class="step4 rv"><span class="n">2</span><h3>Kelola perkara</h3><p>Buka perkara, unggah dokumen SHA-pin, dan tugaskan pekerjaan.</p></div><div class="step4 rv"><span class="n">3</span><h3>Verifikasi dan setujui</h3><p>Review OCR, transisikan akta ber-guard, catat pembayaran.</p></div><div class="step4 rv"><span class="n">4</span><h3>Laporkan dan audit</h3><p>Ekspor CSV, cari dokumen via RAG, telusuri audit trail.</p></div></div></section><section class="feat" id="solusi"><span class="badge-soft tag rv">SOLUSI PROFESI</span><h2 class="rv">Dirancang untuk setiap peran kantor hukum</h2><p class="sub rv">Satu masalah dan satu alur per profesi — tanpa testimoni fiktif.</p><div class="fgrid"><div class="fcard rv"><h3>Notaris</h3><p><b>Masalah:</b> draf akta tercecer antar revisi.<br><b>Alur:</b> template → render → review → approval → signing dalam satu jejak audit. <a href="/solusi/notaris">Pelajari</a></p></div><div class="fcard rv"><h3>PPAT</h3><p><b>Masalah:</b> berkas tanah belum terverifikasi sebelum penomoran.<br><b>Alur:</b> job OCR sertifikat → review manusia → arsip SHA-pin. <a href="/solusi/ppat">Pelajari</a></p></div><div class="fcard rv"><h3>Firma Hukum</h3><p><b>Masalah:</b> perkara, tugas, dan tagihan berjalan terpisah.<br><b>Alur:</b> perkara → tugas → invoice per matter → ekspor CSV. <a href="/solusi/firma-hukum">Pelajari</a></p></div><div class="fcard rv"><h3>Legal Corporate</h3><p><b>Masalah:</b> multi-entitas butuh segregasi dan integrasi.<br><b>Alur:</b> multi-org → peran Owner/Admin/Auditor → token API. <a href="/solusi/legal-corporate">Pelajari</a></p></div><div class="fcard rv"><h3>Staff Operasional</h3><p><b>Masalah:</b> unggahan dan penagihan menumpuk manual.<br><b>Alur:</b> unggah SHA-pin → share kedaluwarsa → catat pembayaran. <a href="/app">Buka aplikasi</a></p></div><div class="fcard rv"><h3>Admin Kantor</h3><p><b>Masalah:</b> siapa mengubah apa tidak terlacak.<br><b>Alur:</b> kelola peran → baca audit trail → ekspor CSV. <a href="/app">Buka aplikasi</a></p></div></div></section><section class="feat" id="status"><span class="badge-soft tag rv">STATUS IMPLEMENTASI</span><h2 class="rv">Peta jujur: apa yang live dan apa yang menunggu</h2><p class="sub rv">Empat label status dipakai konsisten di seluruh situs.</p><div class="legend"><div><code>AVAILABLE</code><br>TERSEDIA — live dan teruji internal.</div><div><code>LIMITED</code><br>TERSEDIA TERBATAS — sebagian live, sisanya 501.</div><div><code>CONFIGURATION_REQUIRED</code><br>BUTUH KONFIGURASI — menunggu kredensial atau provider.</div><div><code>COMING NEXT</code><br>SEGERA HADIR — terencana, belum tersedia.</div></div><ul class="roadmap"><li><b>TERSEDIA:</b> perkara dan CRM, dokumen berversi, keamanan dan RBAC, ekspor CSV, notifikasi in-app, token API, pipeline admin marketing.</li><li><b>TERSEDIA TERBATAS:</b> akta (render dan transisi live; e-sign PSrE 501), keuangan (manual dan trust live; gateway 501), pelaporan RAG (indeks dan search live; chat AI 501).</li><li><b>BUTUH KONFIGURASI:</b> engine OCR live, provider AI, PSrE, billing gateway, integration secret webhook.</li><li><b>SEGERA HADIR:</b> counsel sign-off dan deployment produksi (saat ini uji internal 28/32, bukan production-ready), kredensial eksternal, lisensi deployment, aplikasi mobile (sync-core dalam memori saja).</li></ul></section><section class="feat" id="faq"><span class="badge-soft tag rv">FAQ</span><h2 class="rv">Pertanyaan yang sering ditanyakan</h2><p class="sub rv">Diambil langsung dari database marketing; selengkapnya di <a href="/faq">halaman FAQ</a>.</p>{FAQ_LIST}<p class="sub"><a href="/faq">Lihat semua FAQ</a></p></section><section class="cta rv" id="cta"><div><h2>Siap Meningkatkan Produktivitas Kantor Hukum Anda?</h2><p>Bergabunglah dengan ribuan profesional yang telah mempercayai LegalRust.</p><a class="go" href="/login">Mulai Sekarang</a></div><ul><li>Gratis selamanya</li><li>Setup mudah dalam 1 menit</li><li>Support profesional</li></ul></section>
"##.replace("{FAQ_LIST}", &faq_html);
    html_ok(page_shell(
        "Kelola Perkara, Dokumen dan Akta",
        "Platform manajemen hukum terpadu untuk notaris, PPAT, firma hukum, dan profesional legal.",
        "/",
        "Beranda",
        &body,
    ))
}

async fn login_page() -> impl IntoResponse {
    let body = r#"<div class="page page-center"><div class="container container-tight py-4"><div class="text-center mb-4"><a href="/">LegalRust</a><div class="text-muted">Smart Legal Solution</div></div><div class="card card-md"><div class="card-body"><h1 class="h2 text-center mb-1">Masuk</h1><p class="text-muted text-center">Sesi JWT plus proteksi brute-force 5x percobaan</p><div class="btn-group w-100 mb-3"><button class="btn btn-primary" id="t1">Masuk</button><button class="btn" id="t2">Daftar organisasi</button></div><div id="f_reg" class="hidden"><div class="mb-2"><label class="form-label" for="org">Nama organisasi</label><input class="form-control" id="org"></div><div class="mb-2"><label class="form-label" for="nm">Nama Anda</label><input class="form-control" id="nm"></div></div><div class="mb-2"><label class="form-label" for="email">Email</label><input class="form-control" id="email" type="email" autocomplete="username"></div><div class="mb-2"><label class="form-label" for="pwd">Kata sandi</label><input class="form-control" id="pwd" type="password" autocomplete="current-password"></div><div class="text-danger mb-2" id="err"></div><button class="btn btn-primary w-100" id="go">Masuk aplikasi</button><button class="btn btn-outline-secondary w-100 mt-2" id="demo">Gunakan akun demo</button><div class="text-center mt-2"><a href="/faq">Lupa kata sandi?</a></div><details class="mt-3"><summary class="text-muted">Peran dan hak akses</summary><div class="table-responsive mt-2"><table class="table table-sm"><thead><tr><th>Peran</th><th>Hak akses ringkas</th></tr></thead><tbody><tr><td><span class="badge bg-blue-lt">org_owner</span></td><td>Semua hak akses</td></tr><tr><td><span class="badge bg-blue-lt">office_admin</span></td><td>Kelola harian plus Assign</td></tr><tr><td><span class="badge bg-blue-lt">notary / ppat</span></td><td>Akta: Sign, Verify, Approve</td></tr><tr><td><span class="badge bg-blue-lt">lawyer</span></td><td>Perkara dan dokumen</td></tr><tr><td><span class="badge bg-blue-lt">finance_officer</span></td><td>Keuangan: Approve</td></tr><tr><td><span class="badge bg-blue-lt">auditor</span></td><td>Lihat dan ekspor saja</td></tr></tbody></table></div></details></div></div></div></div>"#;
    html_ok(auth_shell(
        "Masuk",
        "Masuk ke dasbor LegalRust dengan sesi JWT yang aman.",
        body,
    ))
}

async fn app_page() -> impl IntoResponse {
    let body = r#"<div class="dash"><aside class="navbar-vertical dash-side" id="sidebar"><div class="side-brand"><span class="side-logo">LegalRust</span><small>Smart Legal Solution</small><button class="btn btn-sm side-close" id="sbToggle2" type="button">Tutup</button></div><div class="side-org"><label class="side-title" for="orgsel">Organisasi</label><select class="form-select form-select-sm" id="orgsel"><option value="">Memuat…</option></select><input class="form-control form-control-sm mt-2" id="menu-q" placeholder="Cari menu…" autocomplete="off"></div><div class="side-group"><div class="side-title">Utama</div><a class="snav" href="/app" data-tab="ringkas">Dashboard</a></div><div class="side-group"><div class="side-title">Identitas &amp; KYC</div><a class="snav" href="/app" data-tab="ocr">Verifikasi Identitas</a></div><div class="side-group"><div class="side-title">Klien &amp; Perkara</div><a class="snav" href="/app" data-tab="klien">Klien &amp; Perkara</a></div><div class="side-group"><div class="side-title">Keuangan</div><a class="snav" href="/app" data-tab="keu">Keuangan</a></div><div class="side-group"><div class="side-title">Dokumen</div><a class="snav" href="/app" data-tab="dok">Dokumen</a></div><div class="side-group"><div class="side-title">Akta</div><a class="snav" href="/app" data-tab="akta">Akta</a></div><div class="side-group"><div class="side-title">Persetujuan</div><a class="snav" href="/app" data-tab="appr">Persetujuan</a></div><div class="side-group"><div class="side-title">Marketing</div><a class="snav" href="/app" data-tab="mkt">Marketing</a></div><div class="side-group"><div class="side-title">Pengaturan</div><a class="snav" href="/app" data-tab="pengaturan">Pengaturan</a></div><div class="side-group"><div class="side-title">Operasional</div><a class="snav" href="/app" data-tab="audit">Audit</a><a class="snav" href="/app" data-tab="jobs">Jobs</a><a class="snav" href="/skor">Skor 1000 Fitur</a></div><div class="side-group"><div class="side-title">Kepatuhan</div><a class="snav" href="/app" data-tab="kepatuhan">Kepatuhan</a></div><div class="side-user"><span class="avatar" id="avatar2">?</span><span class="side-who"><strong id="uname">Memuat…</strong><small id="urole">…</small></span><button class="btn btn-sm btn-outline-danger logout" type="button">Keluar</button></div></aside><div class="dash-main" id="main" role="main"><header class="dash-top"><button class="btn btn-sm" id="sbToggle" type="button">Menu</button><a href="/" class="navbar-brand">LegalRust</a><ul class="nav mx-auto" id="tabs"><li class="nav-item"><a class="nav-link active" href="/app">Ringkasan</a></li><li class="nav-item"><a class="nav-link" href="/app">Klien</a></li><li class="nav-item"><a class="nav-link" href="/app">Dokumen</a></li><li class="nav-item"><a class="nav-link" href="/app">OCR</a></li><li class="nav-item"><a class="nav-link" href="/app">Akta</a></li><li class="nav-item"><a class="nav-link" href="/app">Keuangan</a></li><li class="nav-item"><a class="nav-link" href="/app">Persetujuan</a></li><li class="nav-item"><a class="nav-link" href="/app">Marketing</a></li><li class="nav-item"><a class="nav-link" href="/app">Pengaturan</a></li><li class="nav-item"><a class="nav-link" href="/app">Audit</a></li><li class="nav-item"><a class="nav-link" href="/app">Jobs</a></li><li class="nav-item"><a class="nav-link" href="/app">Kepatuhan</a></li></ul><div class="ms-2 position-relative"><input class="form-control form-control-sm" id="q" placeholder="Cari klien / perkara / dokumen" autocomplete="off"><div id="qres"></div></div><div class="position-relative"><button class="btn btn-sm" id="bell" type="button">Notifikasi<span class="bell-n" id="bell-n"></span></button><div id="bell-list"></div></div><span class="avatar" id="avatar">?</span><span class="text-muted small" id="orgname">…</span><button class="btn btn-outline-danger btn-sm logout" type="button">Keluar</button></header><div class="container-fluid py-3"><h1 class="page-title" id="hi">Dasbor</h1><div class="text-danger" id="err"></div><div id="p-ringkas"><div class="row g-2" id="g"><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Klien Aktif <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k1">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Perkara Berjalan <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k2">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Tugas Terbuka <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k3">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Dokumen <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k4">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">OCR Menunggu Review <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k5">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Akta Dalam Proses <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k6">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Invoice Outstanding <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k7">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Saldo Trust <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k8">…</div></div></div></div></div><div class="row g-2 mt-1"><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Pipeline Akta</h2></div><div class="card-body" id="funnel"><span class="text-muted">Memuat…</span></div></div></div><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Aktivitas</h2></div><div class="card-body" id="chart"><span class="text-muted">Memuat…</span></div></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Antrean Persetujuan</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="al-q"></tbody></table></div></div><div class="text-muted mt-2" id="ae-q"></div><div class="row g-2 mt-1"><div class="col-md-4"><div class="card"><div class="card-header"><h2 class="card-title">Kalender</h2></div><div class="card-body" id="cal"><span class="text-muted">Memuat…</span></div></div></div><div class="col-md-4"><div class="card"><div class="card-header"><h2 class="card-title">Dokumen Terbaru</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th></tr></thead><tbody id="dl5"></tbody></table></div></div></div><div class="col-md-4"><div class="card"><div class="card-header"><h2 class="card-title">Review OCR</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Tipe</th><th>Status</th></tr></thead><tbody id="ocr-recent"></tbody></table></div></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Perkara Aktif</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody id="m"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Ringkasan Keuangan</h2></div><div class="card-body" id="fin-sum"><span class="text-muted">Memuat…</span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Ekspor Laporan</h2><button class="btn btn-outline-secondary btn-sm" id="rep-matters" type="button">Perkara CSV</button> <button class="btn btn-outline-secondary btn-sm" id="rep-deadlines" type="button">Tenggat CSV</button> <button class="btn btn-outline-secondary btn-sm" id="rep-posture" type="button">Postur Kepatuhan</button> <span class="text-danger" id="rep-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Aksi Cepat</h2></div><div class="card-body qa-grid"><button class="btn btn-outline-primary" id="qa-client" type="button">Tambah Klien</button><button class="btn btn-outline-primary" id="qa-matter" type="button">Buat Perkara</button><button class="btn btn-outline-primary" id="qa-upload" type="button">Upload Dokumen</button><button class="btn btn-outline-primary" id="qa-ocr" type="button">Proses OCR</button><button class="btn btn-outline-primary" id="qa-deed" type="button">Buat Akta</button><button class="btn btn-outline-primary" id="qa-invoice" type="button">Buat Invoice</button></div></div><section class="cta mt-3"><div><h2>Kelola kantor Anda dalam satu dasbor</h2><p>Perkara, dokumen, akta, dan keuangan — semuanya live dari API LegalRust.</p><a class="go" href="/login">Mulai Gratis</a></div><ul><li>Data live, bukan mockup</li><li>Jejak audit penuh</li><li class="dim">Eksternal bertahap, ditandai 501</li></ul></section></div><div id="p-klien" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Klien</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Jenis</th></tr></thead><tbody id="cl"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Tambah Klien</h2><input class="form-control mb-2" id="cli-name" placeholder="nama, mis. PT Maju"><select class="form-control mb-2" id="cli-kind"><option value="individual">individual</option><option value="corporate">corporate</option><option value="government">government</option></select><button class="btn btn-primary" id="cli-btn" type="button">Simpan</button> <button class="btn btn-outline-secondary" id="cli-csv" type="button">Ekspor CSV</button> <span class="text-danger" id="cli-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Daftar Perkara</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody id="m-all"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Buat Perkara</h2><input class="form-control mb-2" id="mat-title" placeholder="judul perkara"><select class="form-control mb-2" id="mat-client"><option value="">Tanpa klien</option></select><button class="btn btn-primary" id="mat-btn" type="button">Buat</button> <span class="text-danger" id="mat-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Tugas</h2></div><div class="card-body"><select class="form-control mb-2" id="task-matter"><option value="">Pilih perkara…</option></select><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Status</th><th>Tenggat</th><th>Aksi</th></tr></thead><tbody id="task-list"></tbody></table></div><input class="form-control mb-2" id="task-title" placeholder="judul tugas baru"><div class="mkt-row"><input class="form-control mkt-filter" id="task-due" type="datetime-local"><button class="btn btn-primary" id="task-btn" type="button">Tambah tugas</button></div> <span class="text-danger" id="task-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Pertemuan klien</h2><input class="form-control mb-2" id="crm-meet-client" placeholder="ID klien" inputmode="numeric"><div class="table-responsive mb-2"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Waktu</th><th>Aksi</th></tr></thead><tbody id="crm-meet-list"><tr><td colspan="4" class="text-muted">Isi ID klien lalu Muat.</td></tr></tbody></table></div><input class="form-control mb-2" id="crm-meet-title" placeholder="judul pertemuan (min 3 huruf)"><input class="form-control mb-2" id="crm-meet-at" type="datetime-local"><textarea class="form-control mb-2" id="crm-meet-notes" rows="2" placeholder="catatan (opsional)"></textarea><button class="btn btn-primary" id="crm-meet-btn" type="button">Simpan pertemuan</button> <button class="btn btn-outline-secondary" id="crm-meet-load" type="button">Muat</button> <span class="text-danger" id="crm-meet-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Tag klien</h2><div class="table-responsive mb-2"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Aksi</th></tr></thead><tbody id="crm-tag-list"></tbody></table></div><input class="form-control mb-2" id="crm-tag-name" placeholder="nama tag baru"><button class="btn btn-primary" id="crm-tag-btn" type="button">Buat tag</button> <span class="text-danger" id="crm-tag-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Tag per klien</h2><input class="form-control mb-2" id="crm-ct-client" placeholder="ID klien" inputmode="numeric"><input class="form-control mb-2" id="crm-ct-tag" placeholder="ID tag" inputmode="numeric"><button class="btn btn-primary" id="crm-ct-attach" type="button">Pasang tag</button> <button class="btn btn-outline-secondary" id="crm-ct-load" type="button">Muat tag</button> <span class="text-danger" id="crm-ct-msg"></span><div class="table-responsive mt-2"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Aksi</th></tr></thead><tbody id="crm-ct-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Pertemuan terbaru</h2><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Klien</th><th>Judul</th><th>Waktu</th></tr></thead><tbody id="crm-recent"></tbody></table></div><button class="btn btn-outline-secondary mt-2" id="crm-recent-btn" type="button">Muat ulang</button> <span class="text-danger" id="crm-recent-msg"></span></div></div></div><div id="p-dok" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Dokumen</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th></tr></thead><tbody id="dl"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Unggah dokumen</h2><input class="form-control mb-2" id="dn" placeholder="nama berkas, mis. kontrak.pdf"><input class="form-control mb-2" type="file" id="df"><button class="btn btn-primary" id="doup" type="button">Unggah</button> <span class="text-danger" id="ue"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Bagikan dokumen</h2><input class="form-control mb-2" id="sh-doc" placeholder="ID dokumen"><input class="form-control mb-2" id="sh-hours" placeholder="masa berlaku (jam, default 72)"><button class="btn btn-secondary" id="sh-btn" type="button">Buat tautan</button> <span class="text-danger" id="sh-msg"></span><div id="sh-list"></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Detail &amp; riwayat versi</h2><div class="mkt-row"><input class="form-control mkt-filter" id="doc-version-id" placeholder="ID dokumen"><button class="btn btn-secondary btn-sm" id="doc-version-btn" type="button">Tampilkan</button></div><div class="text-danger" id="doc-version-msg"></div><div id="doc-version-detail"></div></div></div></div><div id="p-ocr" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Pekerjaan OCR</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Tipe</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="ocr-jobs"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Proses OCR</h2><input class="form-control mb-2" id="ocr-doc" placeholder="ID dokumen"><select class="form-control mb-2" id="ocr-type"><option value="ktp">ktp</option><option value="kk">kk</option><option value="npwp">npwp</option><option value="passport">passport</option><option value="land_certificate">land_certificate</option><option value="deed">deed</option><option value="contract">contract</option><option value="general">general</option></select><button class="btn btn-primary" id="ocr-btn" type="button">Buat job</button> <span class="text-danger" id="ocr-msg"></span><p class="text-muted mt-2">Mesin ekstraksi native (tesseract) live bila terinstall di host — job processed menyimpan baris + confidence; tanpa engine 501.</p></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Detail &amp; Review</h2><div id="ocr-detail"><span class="text-muted">Pilih Lihat pada baris pekerjaan.</span></div></div></div></div><div id="p-akta" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Akta</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="al"></tbody></table></div></div><div class="text-muted mt-2" id="ae"></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Buat Akta dari Template</h2><select class="form-control mb-2" id="deed-tpl"><option value="">Pilih template…</option></select><input class="form-control mb-2" id="deed-matter" placeholder="ID perkara"><input class="form-control mb-2" id="deed-number" placeholder="nomor akta"><input class="form-control mb-2" id="deed-type" placeholder="jenis akta"><input class="form-control mb-2" id="deed-vars" placeholder="variabel JSON, mis. {&#34;hari&#34;:&#34;Senin&#34;}"><button class="btn btn-primary" id="deed-btn" type="button">Buat akta</button> <span class="text-danger" id="deed-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Repertorium (buku harian akta)</h2><div class="mkt-row"><input class="form-control mkt-filter" id="deed-rep-from" placeholder="dari YYYY-MM-DD (opsional)"><input class="form-control mkt-filter" id="deed-rep-to" placeholder="sampai YYYY-MM-DD (opsional)"><button class="btn btn-outline-secondary btn-sm" id="deed-csv" type="button">Ekspor CSV</button><span class="text-danger" id="deed-csv-msg"></span></div><div class="mkt-row mt-2"><input class="form-control mkt-filter" id="deed-cmp-from" placeholder="banding dari YYYY-MM-DD"><input class="form-control mkt-filter" id="deed-cmp-to" placeholder="banding sampai YYYY-MM-DD"><button class="btn btn-outline-secondary btn-sm" id="deed-cmp" type="button">Bandingkan periode</button></div><p class="text-muted">Nomor, jenis, status, dan tanggal pembuatan setiap akta — saring per hari atau rentang tanggal untuk buku harian.</p></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Sertifikat tanah (intake)</h2><div class="mkt-row"><input class="form-control mkt-filter" id="cert-doc" placeholder="ID dokumen sertifikat"><button class="btn btn-primary btn-sm" id="cert-btn" type="button">Buat job sertifikat</button><button class="btn btn-secondary btn-sm" id="cert-reload" type="button">Muat ulang</button></div><div class="text-danger" id="cert-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID job</th><th>Status</th></tr></thead><tbody id="cert-list"></tbody></table></div><p class="text-muted">Hasil ekstraksi direview di tab OCR &amp; Ekstraksi.</p></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Register PPAT</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ppat-number" placeholder="nomor akta, mis. 12/2026"><select class="form-control mkt-filter" id="ppat-kind"><option value="ajb">ajb</option><option value="hibah">hibah</option><option value="tukar_menukar">tukar_menukar</option><option value="apht">apht</option><option value="skmht">skmht</option><option value="sewa">sewa</option><option value="lainnya">lainnya</option></select><input class="form-control" id="ppat-seller" placeholder="penjual/pemberi"><input class="form-control" id="ppat-buyer" placeholder="pembeli/penerima"><input class="form-control" id="ppat-object" placeholder="objek, mis. SHM 001/Bogor"><button class="btn btn-primary btn-sm" id="ppat-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="ppat-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ppat-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Jenis</th><th>Para pihak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="ppat-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pustaka klausa</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="clause-name" placeholder="nama klausa"><input class="form-control mkt-filter" id="clause-cat" placeholder="kategori"><button class="btn btn-primary btn-sm" id="clause-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="clause-reload" type="button">Muat ulang</button></div><div class="text-danger" id="clause-msg"></div><div class="mkt-row"><input class="form-control" id="clause-body" placeholder="isi klausa baru (gunakan {{nama}} untuk variabel)"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Kategori</th><th>v</th><th>Aksi</th></tr></thead><tbody id="clause-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Template Baru</h2><input class="form-control mb-2" id="tpl-name" placeholder="nama template"><input class="form-control mb-2" id="tpl-body" placeholder="isi template, gunakan {{nama}} untuk variabel"><button class="btn btn-secondary" id="tpl-btn" type="button">Simpan template</button> <span class="text-danger" id="tpl-msg"></span><p class="text-muted mt-2">Tanda tangan digital (PSrE) belum dikonfigurasi — endpoint esign 501.</p></div></div></div><div id="p-keu" class="hidden"><div class="row g-2"><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Invoice per perkara</h2></div><div class="card-body"><input class="form-control mb-2" id="km" placeholder="ID perkara"><button class="btn btn-secondary mb-2" id="showinv" type="button">Tampilkan</button><div class="text-danger mb-2" id="il-msg"></div><div id="il"></div></div></div></div><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Trust transfer</h2></div><div class="card-body"><input class="form-control mb-2" id="tf" placeholder="dari rekening ID"><input class="form-control mb-2" id="tt" placeholder="ke rekening ID"><input class="form-control mb-2" id="ta" placeholder="nominal minor"><button class="btn btn-primary" id="dotrf" type="button">Transfer</button> <span class="text-danger" id="te"></span></div></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Buat Invoice</h2><input class="form-control mb-2" id="invm-matter" placeholder="ID perkara"><input class="form-control mb-2" id="invm-total" placeholder="total minor"><button class="btn btn-primary" id="invm-btn" type="button">Buat</button> <span class="text-danger" id="invm-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Ekspor Laporan</h2><button class="btn btn-outline-secondary btn-sm" id="keu-csv" type="button">Perkara CSV</button> <input class="form-control mkt-filter" id="trust-csv-acct" placeholder="ID rekening trust"><button class="btn btn-outline-secondary btn-sm" id="trust-csv-btn" type="button">Trust CSV</button> <span class="text-danger" id="keu-msg"></span><span class="text-danger" id="ar-msg"></span></div><div class="mkt-row mt-2"><strong>Piutang terbuka (AR)</strong><button class="btn btn-outline-secondary btn-sm" id="ar-csv" type="button">Ekspor AR CSV</button><button class="btn btn-outline-secondary btn-sm" id="appr-csv" type="button">Bottleneck approval</button><button class="btn btn-outline-secondary btn-sm" id="prof-csv" type="button">Profitabilitas klien</button></div><p class="text-muted">Semua invoice belum lunas beserta total, terbayar, dan sisa.</p><span class="text-danger" id="trust-csv-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Rekening trust</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Klien</th><th>Saldo</th><th>Aksi</th></tr></thead><tbody id="tl"></tbody></table></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="trust-view-id" placeholder="ID rekening"><button class="btn btn-secondary btn-sm" id="trust-view" type="button">Lihat mutasi</button></div><div class="text-danger" id="trust-view-msg"></div><div id="trust-detail"></div><div class="mkt-row mt-2"><select class="form-control mkt-filter" id="trust-entry-kind"><option value="credit">credit</option><option value="debit">debit</option></select><input class="form-control mkt-filter" id="trust-entry-matter" placeholder="ID perkara (opsional)"><input class="form-control mkt-filter" id="trust-entry-amount" placeholder="nominal minor" inputmode="numeric"><button class="btn btn-primary btn-sm" id="trust-entry-add" type="button">Catat mutasi</button></div><div class="text-danger" id="trust-entry-msg"></div></div></div></div><div id="p-mkt" class="hidden"><div class="text-danger" id="mkt-err"></div><div class="card"><div class="card-header"><h2 class="card-title">Leads pipeline</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="mkt-status-filter">Filter status</label><select class="form-control mkt-filter" id="mkt-status-filter"><option value="">Semua</option><option value="new">new</option><option value="contacted">contacted</option><option value="qualified">qualified</option><option value="demo">demo</option><option value="proposal">proposal</option><option value="negotiation">negotiation</option><option value="won">won</option><option value="lost">lost</option></select><button class="btn btn-secondary btn-sm" id="mkt-reload" type="button">Muat ulang</button><button class="btn btn-outline-secondary btn-sm" id="mkt-csv" type="button">Ekspor CSV</button></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Email</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="mkt-leads"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Permintaan demo</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Email</th><th>Perusahaan</th><th>Status</th></tr></thead><tbody id="mkt-demos"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pesan kontak</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Subjek</th><th>Status</th></tr></thead><tbody id="mkt-contacts"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Newsletter</h2></div><div class="card-body"><div class="mkt-row"><span>Jumlah pelanggan: <strong id="mkt-news-count">0</strong></span><input class="form-control mkt-mail" id="mkt-sub-email" type="email" placeholder="email@kantor.id"><button class="btn btn-primary btn-sm" id="mkt-sub-btn" type="button">Uji subscribe</button><button class="btn btn-outline-secondary btn-sm" id="mkt-unsub-btn" type="button">Berhenti</button></div><div class="text-muted" id="mkt-sub-msg"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Paket harga</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Kode</th><th>Nama</th><th>Bulanan</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="mkt-plans"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">FAQ</h2></div><div class="card-body"><div class="table-responsive"><table class="table card-table"><thead><tr><th>Slug</th><th>Pertanyaan</th></tr></thead><tbody id="mkt-faqs"></tbody></table></div><div class="mkt-row"><input class="form-control" id="faq-slug" placeholder="slug"><input class="form-control" id="faq-q" placeholder="pertanyaan"><input class="form-control" id="faq-a" placeholder="jawaban"><button class="btn btn-primary btn-sm" id="mkt-faq-add" type="button">Tambah FAQ</button></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Artikel</h2></div><div class="card-body"><div class="table-responsive"><table class="table card-table"><thead><tr><th>Slug</th><th>Judul</th><th>Terbit</th><th>Aksi</th></tr></thead><tbody id="mkt-articles"></tbody></table></div><div class="mkt-row"><input class="form-control" id="art-slug" placeholder="slug"><input class="form-control" id="art-title" placeholder="judul"><input class="form-control" id="art-body" placeholder="isi markdown"><label class="form-check"><input class="form-check-input" type="checkbox" id="art-pub"><span class="form-check-label">Terbit</span></label><button class="btn btn-primary btn-sm" id="mkt-art-add" type="button">Simpan artikel</button></div></div></div></div><div id="p-audit" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Jejak Audit</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="audit-entity">Filter entitas</label><select class="form-control mkt-filter" id="audit-entity"><option value="">Semua</option><option value="matter">matter</option><option value="document">document</option><option value="deed">deed</option><option value="notification">notification</option><option value="user">user</option><option value="ppat">ppat</option><option value="report">report</option><option value="trust">trust</option><option value="ropa">ropa</option></select><button class="btn btn-secondary btn-sm" id="audit-reload" type="button">Muat ulang</button></div><div class="text-danger" id="audit-msg"></div><div class="text-muted" id="audit-note"></div></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Aktor</th><th>Aksi</th><th>Entitas</th><th>Waktu</th></tr></thead><tbody id="audit-rows"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Webhook masuk</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="wh-reload" type="button">Muat ulang</button></div><div class="text-danger" id="wh-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Provider</th><th>Event</th><th>Status</th><th>Detail</th></tr></thead><tbody id="wh-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Endpoint outbound</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="whe-url" placeholder="https://mitra.id/hook"><input class="form-control mkt-filter" id="whe-secret" placeholder="secret HMAC (opsional)"><button class="btn btn-primary btn-sm" id="whe-add" type="button">Daftarkan</button><button class="btn btn-secondary btn-sm" id="whe-reload" type="button">Muat ulang</button></div><div class="text-danger" id="whe-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>URL</th><th>Event</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="whe-list"></tbody></table></div></div></div></div><div id="p-jobs" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Antrean Jobs <span class="text-muted small" id="job-summary"></span></h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Jenis</th><th>Status</th><th>Percobaan</th></tr></thead><tbody id="job-rows"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Enqueue Job</h2><input class="form-control mb-2" id="job-kind" placeholder="jenis job, mis. notify.digest"><button class="btn btn-primary" id="job-btn" type="button">Enqueue</button> <span class="text-danger" id="job-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Dead letters (gagal terkirim)</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="wh-dl-reload" type="button">Muat ulang</button></div><div class="text-danger" id="wh-dl-msg"></div></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Provider</th><th>Event</th><th>Detail</th></tr></thead><tbody id="wh-dl-list"></tbody></table></div></div></div><div id="p-appr" class="hidden"><div class="text-danger" id="appr-err"></div><div class="row g-2"><div class="col-md-5"><div class="card"><div class="card-header"><h2 class="card-title">Buat permintaan persetujuan</h2></div><div class="card-body"><label class="form-label" for="appr-entity">Entitas</label><select class="form-control mb-2" id="appr-entity"><option value="matter">matter</option><option value="invoice">invoice</option><option value="deed">deed</option></select><input class="form-control mb-2" id="appr-entity-id" placeholder="ID entitas"><input class="form-control mb-2" id="appr-title" placeholder="judul, mis. Setujui AJB"><input class="form-control mb-2" id="appr-role1" placeholder="peran tahap 1, mis. office_admin"><input class="form-control mb-2" id="appr-role2" placeholder="peran tahap 2 (opsional)"><input class="form-control mb-2" id="appr-role3" placeholder="peran tahap 3 (opsional)"><button class="btn btn-primary" id="appr-btn" type="button">Buat</button> <span class="text-danger" id="appr-msg"></span></div></div></div><div class="col-md-7"><div class="card"><div class="card-header"><h2 class="card-title">Daftar permintaan</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="appr-filter">Filter status</label><select class="form-control mkt-filter" id="appr-filter"><option value="">Semua</option><option value="pending">pending</option><option value="needs_rework">needs_rework</option><option value="approved">approved</option><option value="rejected">rejected</option><option value="revoked">revoked</option></select><button class="btn btn-secondary btn-sm" id="appr-reload" type="button">Muat ulang</button></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Entitas</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="appr-list"></tbody></table></div><div class="mkt-row mt-2"><strong>Notifikasi approval</strong><span class="text-muted" id="appr-notif-unread"></span><label class="form-check"><input class="form-check-input" type="checkbox" id="appr-notif-unread-only"><span class="form-check-label">Hanya belum dibaca</span></label><button class="btn btn-secondary btn-sm" id="appr-notif-reload" type="button">Muat ulang</button></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Dibaca</th><th>Aksi</th></tr></thead><tbody id="appr-notif-list"></tbody></table></div><div class="mkt-row mt-2"><strong>Delegasi (pengganti cuti)</strong><input class="form-control mkt-filter" id="deleg-uid" placeholder="UID penerima kuasa"><button class="btn btn-primary btn-sm" id="deleg-add" type="button">Beri kuasa</button><button class="btn btn-secondary btn-sm" id="deleg-reload" type="button">Muat ulang</button></div><div class="text-danger" id="deleg-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pemberi</th><th>Penerima</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="deleg-list"></tbody></table></div></div></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Detail &amp; keputusan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="appr-id" placeholder="ID permintaan"><button class="btn btn-secondary btn-sm" id="appr-show" type="button">Tampilkan</button></div><div id="appr-detail"></div><div class="mkt-row"><input class="form-control mkt-filter" id="appr-step" placeholder="no. tahap"><input class="form-control mkt-filter" id="appr-comment" placeholder="komentar (opsional)"><button class="btn btn-success btn-sm" id="appr-ok" type="button">Setujui</button><button class="btn btn-danger btn-sm" id="appr-no" type="button">Tolak</button><button class="btn btn-warning btn-sm" id="appr-rw" type="button">Rework</button><button class="btn btn-outline-secondary btn-sm" id="appr-revoke" type="button">Cabut</button></div><div class="text-danger" id="appr-dmsg"></div></div></div></div><div id="p-kepatuhan" class="hidden"><div class="text-danger" id="kep-err"></div><div class="card"><div class="card-header"><h2 class="card-title">Obligasi UU PDP</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="kep-title" placeholder="judul obligasi"><input class="form-control mkt-filter" id="kep-cat" placeholder="kategori (umum)"><input class="form-control mkt-filter" id="kep-owner" placeholder="penanggung jawab"><button class="btn btn-primary btn-sm" id="kep-add" type="button">Tambah</button><select class="form-control mkt-filter" id="kep-filter"><option value="">Semua status</option><option value="open">open</option><option value="in_progress">in_progress</option><option value="done">done</option></select><button class="btn btn-secondary btn-sm" id="kep-reload" type="button">Muat ulang</button></div><div class="text-danger" id="kep-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Kategori</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="kep-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Persetujuan (consent)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="con-subject" placeholder="subjek, mis. NIK/email"><input class="form-control" id="con-purpose" placeholder="tujuan, mis. kyc"><button class="btn btn-primary btn-sm" id="con-grant" type="button">Catat persetujuan</button><input class="form-control mkt-filter" id="con-filter" placeholder="filter subjek"><button class="btn btn-secondary btn-sm" id="con-reload" type="button">Muat ulang</button></div><div class="text-danger" id="con-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Subjek</th><th>Tujuan</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="con-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Register pemrosesan (ROPA, UU PDP)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ropa-activity" placeholder="aktivitas, mis. Pendaftaran klien"><input class="form-control" id="ropa-purpose" placeholder="tujuan, mis. Layanan notaris"><input class="form-control" id="ropa-cats" placeholder="kategori data, mis. NIK, alamat"><input class="form-control" id="ropa-basis" placeholder="dasar hukum, mis. consent"><input class="form-control" id="ropa-ret" placeholder="retensi, mis. 10 tahun"><button class="btn btn-primary btn-sm" id="ropa-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="ropa-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ropa-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Aktivitas</th><th>Tujuan</th><th>Kategori</th><th>Dasar</th><th>Retensi</th><th>Aksi</th></tr></thead><tbody id="ropa-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">DPIA (penilaian dampak)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="dpia-title" placeholder="judul asesmen"><input class="form-control" id="dpia-types" placeholder="jenis data, mis. NIK, biometrik"><select class="form-control mkt-filter" id="dpia-risk"><option value="low">low</option><option value="medium" selected>medium</option><option value="high">high</option></select><button class="btn btn-primary btn-sm" id="dpia-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="dpia-reload" type="button">Muat ulang</button></div><div class="text-danger" id="dpia-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Data</th><th>Risiko</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="dpia-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Insiden kebocoran (72 jam)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="breach-title" placeholder="judul insiden"><input class="form-control mkt-filter" id="breach-date" placeholder="diketahui YYYY-MM-DD"><input class="form-control mkt-filter" id="breach-affected" placeholder="terdampak" inputmode="numeric"><button class="btn btn-primary btn-sm" id="breach-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="breach-reload" type="button">Muat ulang</button></div><div class="text-danger" id="breach-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Diketahui</th><th>Terdampak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="breach-list"></tbody></table></div></div></div></div><div id="p-pengaturan" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Token API</h2></div><div class="card-body"><p class="text-muted">Buat token API untuk integrasi internal. Token penuh hanya ditampilkan sekali saat dibuat.</p><div class="set-row"><input class="form-control" id="tok-name" placeholder="nama token, mis. ci"><input class="form-control mkt-filter" id="tok-days" placeholder="hari, default 90" inputmode="numeric"><button class="btn btn-primary btn-sm" id="tok-btn" type="button">Buat token</button></div><div class="text-danger" id="tok-msg"></div><div class="tok-plain hidden" id="tok-plain"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Kedaluwarsa</th><th>Aksi</th></tr></thead><tbody id="tok-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Sesi aktif</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kedaluwarsa</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="sess-list"></tbody></table></div><div class="card-body"><button class="btn btn-secondary btn-sm" id="sess-reload" type="button">Muat ulang</button> <span class="text-danger" id="sess-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Ubah kata sandi</h2></div><div class="card-body"><input class="form-control mb-2" id="pw-cur" type="password" autocomplete="current-password" placeholder="kata sandi saat ini"><input class="form-control mb-2" id="pw-new" type="password" autocomplete="new-password" placeholder="kata sandi baru (min 12 karakter)"><button class="btn btn-primary btn-sm" id="pw-btn" type="button">Ubah kata sandi</button> <span class="text-danger" id="pw-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Tetapkan peran (admin)</h2></div><div class="card-body"><p class="text-muted">Pilih pengguna satu organisasi lalu tetapkan peran. Membutuhkan hak Assign.</p><div class="set-row"><select class="form-control" id="role-uid"><option value="">Memuat pengguna…</option></select><select class="form-control" id="role-sel"><option value="super_admin">super_admin</option><option value="org_owner">org_owner</option><option value="office_admin">office_admin</option><option value="notary">notary</option><option value="ppat">ppat</option><option value="lawyer">lawyer</option><option value="legal_staff">legal_staff</option><option value="document_officer">document_officer</option><option value="finance_officer">finance_officer</option><option value="auditor">auditor</option><option value="client">client</option><option value="reviewer">reviewer</option></select><button class="btn btn-primary btn-sm" id="role-btn" type="button">Tetapkan</button></div><div class="text-danger" id="role-msg"></div><div class="table-responsive mt-2"><table class="table card-table"><thead><tr><th>Peran</th><th>Hak akses</th></tr></thead><tbody id="role-matrix"></tbody></table></div></div></div></div><div class="mt-3"><a href="/login" class="btn btn-outline-secondary btn-sm logout">Ganti akun</a></div></div></div></div>
"#;
    html_ok(app_shell("Dasbor", body))
}

async fn skor_page() -> impl IntoResponse {
    let body = r#"<div class="dash"><div class="dash-main" id="main" role="main"><header class="dash-top"><a href="/app" class="btn btn-sm">← Kembali ke aplikasi</a><span class="navbar-brand">Skor 1000 Fitur</span></header><div class="container-fluid py-3"><h1 class="page-title">Transparansi skor 1000 fitur</h1><p class="text-muted">Semua fitur katalog ditampilkan dengan skor jujur apa adanya: hanya pekerjaan yang benar-benar terimplementasi dan terverifikasi yang bernilai tinggi. Sisanya mencantumkan status dan bukti nyata.</p><div class="mkt-row" id="skor-chips"></div><div class="mkt-row"><input class="form-control" id="skor-q" placeholder="cari id / judul"><select class="form-control mkt-filter" id="skor-domain"><option value="">Semua domain</option></select><select class="form-control mkt-filter" id="skor-status"><option value="">Semua status</option><option value="IMPLEMENTED">implemented</option><option value="PARTIAL">partial</option><option value="PLANNED">planned</option><option value="NOT_STARTED">not_started</option><option value="CONFIGURATION_REQUIRED">configuration_required</option></select><button class="btn btn-secondary btn-sm" id="skor-reload" type="button">Muat ulang</button></div><div class="text-danger" id="skor-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Domain</th><th>Judul</th><th>Status</th><th>Skor</th><th>Terverifikasi</th></tr></thead><tbody id="skor-list"></tbody></table></div><div class="q-pager"><button class="btn btn-sm" id="skor-prev" type="button">Sebelumnya</button> <span id="skor-page"></span> <button class="btn btn-sm" id="skor-next" type="button">Berikutnya</button></div></div></div></div>"#;
    html_ok(app_shell("Skor Fitur", body))
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
        r#"<section class="feat"><span class="badge-soft tag">FITUR</span><h1>Fitur Lengkap LegalRust</h1><p class="sub">Enam kemampuan inti dalam satu platform teraudit.</p>{}</section><div class="fgrid"><div class="fcard"><h3>Manajemen Perkara Terintegrasi</h3><p>Klien, perkara, tugas bertransisi aman, notifikasi. <a href="/fitur/manajemen-perkara">Pelajari</a></p></div><div class="fcard"><h3>Manajemen Dokumen Berversi</h3><p>Upload SHA-pin, versi, share kedaluwarsa. <a href="/fitur/manajemen-dokumen">Pelajari</a></p></div><div class="fcard"><h3>OCR dan Verifikasi</h3><p>Job OCR plus review manusia. <a href="/fitur/ocr">Pelajari</a></p></div><div class="fcard"><h3>AI Legal Assistant</h3><p>RAG keyword live; chat live via Ollama/provider. <a href="/fitur/ai-legal-assistant">Pelajari</a></p></div><div class="fcard"><h3>Generasi Akta</h3><p>Template berversi plus state machine ber-guard. <a href="/fitur/generasi-akta">Pelajari</a></p></div><div class="fcard"><h3>Tanda Tangan Digital</h3><p>Alur teraudit; PSrE bertahap. <a href="/fitur/tanda-tangan">Pelajari</a></p></div></div>"#,
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
                ("POST /api/v1/ocr/jobs/{id}/process", "Pemrosesan live via tesseract native (ind+eng) — 501 bila engine tidak terinstall di host"),
            ],
            "Jujur: POST /api/v1/ocr/jobs/{id}/process menjalankan tesseract bila tersedia (hasil baris+confidence tersimpan, status processed); tanpa engine mengembalikan 501. Hanya gambar (png/jpg/webp/bmp/tif); PDF ditolak 422.",
        ),
    ))
}

async fn fitur_ai() -> impl IntoResponse {
    html_ok(page_shell(
        "Asisten Legal AI",
        "RAG keyword live plus chat generatif via endpoint OpenAI-compatible.",
        "/fitur/ai-legal-assistant",
        "",
        &feature_body(
            "Asisten Legal AI",
            "TERSEDIA",
            "Indeks dan pencarian RAG live untuk dokumen teks; chat generatif live via endpoint OpenAI-compatible (default Ollama lokal).",
            &[
                ("POST /api/v1/rag/index", "Indeks dokumen teks menjadi chunk pencarian"),
                ("GET /api/v1/rag/search", "Pencarian keyword full-text per organisasi"),
                ("POST /api/v1/ai/chat", "Chat generatif — live bila endpoint menjawab (AI_BASE_URL/AI_MODEL), 501 bila tidak"),
                ("GET /api/v1/ai/usage", "Meteran token input/output per organisasi"),
            ],
            "Jujur: POST /api/v1/ai/chat memanggil endpoint yang dikonfigurasi dan menyimpan jawaban + usage; tanpa endpoint yang menjawab mengembalikan 501. Tidak ada konten yang difabrikasi.",
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
        r#"<section class="feat"><span class="badge-soft tag">TENTANG</span><h1>Tentang LegalRust</h1><p class="sub">Platform manajemen hukum modern yang dibangun dengan Rust: cepat, aman, dan teraudit.</p>{}</section><section class="card"><div class="card-body"><h2 class="card-title">Status jujur</h2><p>LegalRust berada pada tahap uji internal — bukan production-ready. Setiap halaman fitur mencantumkan badge status yang jujur: TERSEDIA, TERSEDIA TERBATAS, atau BUTUH KONFIGURASI. OCR native live bila tesseract terinstall di host, AI chat live bila endpoint menjawab, Tripay live bila kredensial diisi; e-sign PSrE ditandai terbuka dan mengembalikan 501 hingga dikonfigurasi.</p><h2 class="card-title">Prinsip keamanan</h2><p>Password Argon2, sesi JWT revocable, lockout brute-force 5x per 5 menit, RBAC per endpoint, header CSP dan HSTS, serta audit trail yang hanya dapat dibaca peran terotorisasi.</p></div></section>"#,
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
async fn public_unsubscribe(
    State(s): State<FullState>,
    Json(b): Json<SubscribeReq>,
) -> Result<Json<Value>, AppError> {
    if !s.stores.limiter.check("mkt:unsubscribe") {
        return Err(AppError::RateLimited);
    }
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .mkt
        .mkt_unsubscribe(&b.email)
        .await
        .map_err(map_store)?;
    Ok(Json(
        json!({"subscribed": false, "email": b.email.trim().to_lowercase()}),
    ))
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

// ---------- CRM: client meetings + tags (migration 0011) ----------
// Tables client_meetings / client_tags / client_tag_links are created by
// migration 0011_crm.sql, applied automatically on boot via db::migrate.
// Handlers verify client ownership against the caller's org (list_clients)
// before any meeting/tag-link mutation, so a foreign client id yields 404.
fn meeting_json(m: &legalrust_database::MeetingRow) -> Value {
    json!({
        "id": m.id, "client_id": m.client_id, "title": m.title,
        "notes": m.notes, "happened_at": m.happened_at, "created_by": m.created_by,
    })
}
fn crm_tag_json(t: &legalrust_database::ClientTagRow) -> Value {
    json!({"id": t.id, "name": t.name})
}
async fn crm_require_client(org: &str, core: &Arc<dyn CoreStore>, id: i64) -> Result<(), AppError> {
    let v = core
        .list_clients(org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    if v.iter().any(|c| c.id == id) {
        Ok(())
    } else {
        Err(AppError::NotFound("client".into()))
    }
}
#[derive(Debug, Deserialize, Validate)]
struct MeetingReq {
    #[validate(length(min = 3))]
    title: String,
    notes: Option<String>,
    happened_at: String,
}
fn check_happened_at(s: &str) -> Result<(), AppError> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|_| ())
        .map_err(|_| AppError::Validation("happened_at must be YYYY-MM-DD HH:MM:SS".into()))
}
async fn meeting_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<MeetingReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    check_happened_at(&b.happened_at)?;
    crm_require_client(&u.claims.org, &s.stores.core, id).await?;
    let m = s
        .stores
        .crm
        .meeting_create(
            &u.claims.org,
            id,
            b.title.trim(),
            b.notes.as_deref(),
            &b.happened_at,
            &u.claims.sub,
        )
        .await
        .map_err(map_store)?;
    Ok(Json(meeting_json(&m)))
}
async fn meeting_list_by_client(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    crm_require_client(&u.claims.org, &s.stores.core, id).await?;
    let v = s
        .stores
        .crm
        .meeting_list_by_client(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v.iter().map(meeting_json).collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct RecentQ {
    limit: Option<i64>,
}
async fn meeting_recent(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<RecentQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let lim = q.limit.unwrap_or(50).clamp(1, 200) as usize;
    let mut v = s
        .stores
        .crm
        .meeting_list_by_org(&u.claims.org)
        .await
        .map_err(map_store)?;
    v.sort_by_key(|m| -m.id);
    v.truncate(lim);
    Ok(Json(json!(v.iter().map(meeting_json).collect::<Vec<_>>())))
}
async fn meeting_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    s.stores
        .crm
        .meeting_delete(&u.claims.org, id)
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::NotFound => AppError::NotFound("meeting".into()),
            _ => map_store(e),
        })?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct TagReq {
    #[validate(length(min = 1, max = 64))]
    name: String,
}
async fn tag_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<TagReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if b.name.trim().is_empty() {
        return Err(AppError::Validation("name".into()));
    }
    let t = s
        .stores
        .crm
        .tag_create(&u.claims.org, b.name.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(crm_tag_json(&t)))
}
async fn tag_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .crm
        .tag_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v.iter().map(crm_tag_json).collect::<Vec<_>>())))
}
async fn tag_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    s.stores
        .crm
        .tag_delete(&u.claims.org, id)
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::NotFound => AppError::NotFound("tag".into()),
            _ => map_store(e),
        })?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize)]
struct TagAttachReq {
    tag_id: i64,
}
async fn tag_attach(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<TagAttachReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    crm_require_client(&u.claims.org, &s.stores.core, id).await?;
    s.stores
        .crm
        .tag_attach(&u.claims.org, b.tag_id, id)
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::NotFound => AppError::NotFound("tag".into()),
            _ => map_store(e),
        })?;
    Ok(Json(json!({"attached": true})))
}
async fn tag_detach(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path((id, tag_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    crm_require_client(&u.claims.org, &s.stores.core, id).await?;
    s.stores
        .crm
        .tag_detach(&u.claims.org, tag_id, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"detached": true})))
}
async fn tags_for_client(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    crm_require_client(&u.claims.org, &s.stores.core, id).await?;
    let v = s
        .stores
        .crm
        .tags_for_client(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v.iter().map(crm_tag_json).collect::<Vec<_>>())))
}
// ---------- end CRM ----------

pub fn build_router(state: FullState) -> Router {
    let request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/", get(landing))
        .route("/login", get(login_page))
        .route("/app", get(app_page))
        .route("/skor", get(skor_page))
        .route("/api/v1/features/scores", get(feature_scores))
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
        .route("/api/v1/auth/sessions", get(session_list))
        .route("/api/v1/auth/sessions/{id}/revoke", post(session_revoke))
        .route("/api/v1/orgs/me", get(me))
        .route("/api/v1/clients", post(create_client).get(list_clients))
        .route("/api/v1/matters", post(create_matter).get(list_matters))
        .route(
            "/api/v1/matters/{mid}/tasks",
            post(create_task).get(list_tasks),
        )
        .route("/api/v1/tasks/{tid}", post(patch_task))
        .route("/api/v1/tasks/{tid}/due", post(task_set_due))
        .route("/api/v1/tasks/due", get(task_due_list))
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
        .route("/api/v1/deed-clauses", post(clause_create).get(clause_list))
        .route(
            "/api/v1/deed-clauses/{id}",
            put(clause_update).delete(clause_delete),
        )
        .route("/api/v1/deeds/from-template", post(deed_from_template))
        .route("/api/v1/deeds", get(deed_list))
        .route("/api/v1/deeds/{id}", get(deed_get))
        .route("/api/v1/deeds/{id}/esign", post(deed_esign))
        .route("/api/v1/deeds/{id}/transition", post(deed_transition))
        .route("/api/v1/ai/chat", post(ai_chat))
        .route("/api/v1/ai/usage", get(ai_usage))
        .route("/api/v1/rag/index", post(rag_index))
        .route("/api/v1/rag/search", get(rag_search))
        .route("/api/v1/search", get(global_search))
        .route(
            "/api/v1/matters/{mid}/invoices",
            post(inv_create).get(inv_list),
        )
        .route("/api/v1/invoices/{id}", get(inv_get))
        .route("/api/v1/invoices/{id}/receipt", get(inv_receipt))
        .route("/api/v1/invoices/{id}/payments", post(pay_add))
        .route("/api/v1/invoices/{id}/pay-gateway", post(pay_gateway))
        .route("/api/v1/reports/matters-csv", get(matters_csv))
        .route("/api/v1/reports/ar-aging.csv", get(ar_aging_csv_export))
        .route(
            "/api/v1/reports/approvals-bottleneck.csv",
            get(approvals_bottleneck_csv),
        )
        .route("/api/v1/reports/profitability.csv", get(profitability_csv))
        .route(
            "/api/v1/reports/deadline-compliance.csv",
            get(deadline_compliance_csv),
        )
        .route(
            "/api/v1/reports/compliance-posture.csv",
            get(compliance_posture_csv),
        )
        .route("/api/v1/clients.csv", get(clients_csv))
        .route("/api/v1/webhooks/incoming/{provider}", post(webhook_in))
        .route("/api/v1/webhooks/deliveries", get(webhook_list))
        .route(
            "/api/v1/webhooks/endpoints",
            post(webhook_endpoint_create).get(webhook_endpoint_list),
        )
        .route(
            "/api/v1/webhooks/endpoints/{id}",
            delete(webhook_endpoint_delete),
        )
        .route(
            "/api/v1/webhooks/endpoints/{id}/test",
            post(webhook_endpoint_test),
        )
        .route(
            "/api/v1/compliance/obligations",
            post(pdp_obligation_create).get(pdp_obligation_list),
        )
        .route(
            "/api/v1/compliance/obligations/{id}/status",
            post(pdp_obligation_status),
        )
        .route("/api/v1/consents", post(consent_grant).get(consent_list))
        .route("/api/v1/consents/{id}/withdraw", post(consent_withdraw))
        .route("/api/v1/compliance/ropa", post(ropa_create).get(ropa_list))
        .route("/api/v1/compliance/ropa/{id}", delete(ropa_delete))
        .route("/api/v1/compliance/dpia", post(dpia_create).get(dpia_list))
        .route("/api/v1/compliance/dpia/{id}/status", post(dpia_set_status))
        .route(
            "/api/v1/compliance/breach",
            post(breach_create).get(breach_list),
        )
        .route(
            "/api/v1/compliance/breach/{id}/status",
            post(breach_set_status),
        )
        .route("/api/v1/ppat", post(ppat_create).get(ppat_list))
        .route("/api/v1/ppat/{id}/status", post(ppat_set_status))
        .route("/api/v1/ppat/{id}/checklist", get(ppat_checklist))
        .route(
            "/api/v1/delegations",
            post(delegation_set).get(delegation_list),
        )
        .route("/api/v1/integrations", get(integrations))
        .route("/api/v1/auth/tokens", post(token_create).get(token_list))
        .route("/api/v1/auth/tokens/{id}", delete(token_revoke))
        .route("/api/v1/auth/password", post(password_change))
        .route("/api/v1/users", get(user_list))
        .route("/api/v1/roles/matrix", get(role_matrix))
        .route("/api/v1/users/{uid}/roles", post(user_assign_role))
        .route("/api/v1/notifications", get(notify_list))
        .route(
            "/api/v1/notifications/prefs",
            get(notif_pref_list).post(notif_pref_set),
        )
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
        .route(
            "/api/v1/reports/trust-ledger.csv",
            get(trust_ledger_csv_export),
        )
        .route("/api/v1/reports/deeds.csv", get(deeds_csv_export))
        .route("/api/v1/approvals", post(appr_create).get(appr_list))
        .route("/api/v1/approvals/{id}", get(appr_get))
        .route("/api/v1/approvals/{id}/decisions", post(appr_decide))
        .route("/api/v1/approvals/{id}/revoke", post(appr_revoke))
        .route("/api/v1/events", get(sse_events))
        .route("/api/v1/public/contact", post(public_contact))
        .route("/api/v1/public/demo-requests", post(public_demo))
        .route("/api/v1/public/sales-inquiries", post(public_sales))
        .route(
            "/api/v1/public/newsletter/subscribe",
            post(public_subscribe),
        )
        .route(
            "/api/v1/public/newsletter/unsubscribe",
            post(public_unsubscribe),
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
        .route(
            "/api/v1/clients/{id}/meetings",
            post(meeting_create).get(meeting_list_by_client),
        )
        .route("/api/v1/meetings", get(meeting_recent))
        .route("/api/v1/meetings/{id}", delete(meeting_delete))
        .route("/api/v1/tags", post(tag_create).get(tag_list))
        .route("/api/v1/tags/{id}", delete(tag_delete))
        .route(
            "/api/v1/clients/{id}/tags",
            post(tag_attach).get(tags_for_client),
        )
        .route("/api/v1/clients/{id}/tags/{tag_id}", delete(tag_detach))
        .fallback(not_found)
        .layer(RequestBodyLimitLayer::new(
            (MAX_UPLOAD + 1024 * 1024) as usize,
        ))
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
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
