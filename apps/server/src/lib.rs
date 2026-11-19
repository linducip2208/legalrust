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
    invite_token: Option<String>,
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
    // Invite path: join an existing org instead of creating one.
    if let Some(tok) = b.invite_token.as_deref().filter(|t| !t.trim().is_empty()) {
        let hash = legalrust_document::sha256_hex(tok.trim().as_bytes());
        // Peek first so a wrong email does not burn the token.
        let peek = s
            .stores
            .ops
            .invite_peek(&hash)
            .await
            .map_err(map_store)?
            .ok_or(AppError::NotFound("invite".into()))?;
        if peek.email != b.email.trim().to_lowercase() {
            return Err(AppError::Forbidden);
        }
        let inv = s.stores.ops.invite_redeem(&hash).await.map_err(map_store)?;
        let role = legalrust_domain::Role::parse(&inv.role)
            .ok_or_else(|| AppError::Validation("role".into()))?;
        let pw_hash = legalrust_auth::hash_password(&b.password)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("hash")))?;
        let uid = s
            .stores
            .users
            .create_org_member(&inv.org, &b.email, &b.name, &pw_hash)
            .await
            .map_err(map_store)?;
        s.stores
            .users
            .assign_role(&inv.org, &uid, role.as_str())
            .await
            .map_err(map_store)?;
        let exp_dt = chrono::Utc::now().naive_utc() + chrono::Duration::hours(8);
        let exp = chrono::Utc::now().timestamp() as usize + 8 * 3600;
        let sid = s
            .stores
            .users
            .create_session(&uid, exp_dt)
            .await
            .map_err(map_store)?;
        let roles = s.stores.users.user_roles(&uid).await.map_err(map_store)?;
        let token = legalrust_auth::issue_token(
            &s.stores.jwt_secret,
            &legalrust_auth::Claims {
                sub: uid.clone(),
                org: inv.org.clone(),
                roles,
                sid: sid.clone(),
                exp,
            },
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
        audit(
            &s.pool,
            &s.stores,
            AuditEvent {
                tenant_id: inv.org.clone(),
                actor_id: Some(uid.clone()),
                action: "org.invite_accepted".into(),
                entity: "organization".into(),
                entity_id: 0,
                ip: None,
            },
        )
        .await;
        return Ok(Json(
            json!({"user_id": uid, "org_id": inv.org, "token": token}),
        ));
    }
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
        .map(|t| json!({"id": t.id, "title": t.title, "status": t.status, "due": t.due, "assignee": t.assignee, "desk": t.desk}))
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
        .map(|t| json!({"id": t.id, "title": t.title, "status": t.status, "due": t.due, "matter_id": t.matter_id, "assignee": t.assignee, "desk": t.desk}))
        .collect::<Vec<_>>())))
}
/// Multi-desk dispatch (F-D12-033): assign a task to a person + desk.
#[derive(Debug, Deserialize, Validate)]
struct TaskAssignReq {
    #[validate(length(max = 120))]
    assignee: Option<String>,
    #[validate(length(max = 60))]
    desk: Option<String>,
}
async fn task_assign(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(tid): Path<i64>,
    Json(b): Json<TaskAssignReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let t = s
        .stores
        .core
        .task_assign(
            &u.claims.org,
            tid,
            b.assignee.as_deref().unwrap_or(""),
            b.desk.as_deref().unwrap_or(""),
        )
        .await
        .map_err(|e| match e {
            legalrust_database::StoreError::NotFound => AppError::NotFound("task".into()),
            legalrust_database::StoreError::Conflict(m) => AppError::Conflict(m),
            _ => AppError::Internal(anyhow::anyhow!(e.to_string())),
        })?;
    Ok(Json(
        json!({"id": t.id, "assignee": t.assignee, "desk": t.desk}),
    ))
}
/// Dispatch board: open (non-done) tasks grouped by desk, unassigned first.
async fn dispatch_board(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .core
        .task_board(&u.claims.org, 500)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let mut by_desk: std::collections::BTreeMap<String, Vec<Value>> =
        std::collections::BTreeMap::new();
    for t in v.iter().filter(|t| t.status != "done") {
        let key = if t.desk.trim().is_empty() {
            "tanpa meja".to_string()
        } else {
            t.desk.clone()
        };
        by_desk.entry(key).or_default().push(
            json!({"id": t.id, "matter_id": t.matter_id, "title": t.title, "status": t.status, "assignee": t.assignee, "due": t.due}),
        );
    }
    Ok(Json(json!(by_desk
        .into_iter()
        .map(|(k, items)| json!({"desk": k, "tasks": items}))
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
/// OCR accuracy trend per document type (F-D20-010): average field
/// confidence across processed jobs. Honest telemetry, no fabrication.
async fn ocr_accuracy_csv_export(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let jobs = s
        .stores
        .ops
        .ocr_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut samples: Vec<(String, i64, f32)> = Vec::new();
    for j in &jobs {
        if j.status != "processed" {
            continue;
        }
        let (_, fields) = s
            .stores
            .ops
            .ocr_get(&u.claims.org, j.id)
            .await
            .map_err(map_store)?;
        if fields.is_empty() {
            continue;
        }
        let mean = fields.iter().map(|f| f.confidence as f64).sum::<f64>() / fields.len() as f64;
        samples.push((j.doc_type.clone(), fields.len() as i64, mean as f32));
    }
    let rows = legalrust_reporting::ocr_accuracy_aggregate(&samples);
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.ocr_exported".into(),
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
    Ok((
        StatusCode::OK,
        h,
        legalrust_reporting::ocr_accuracy_csv(&rows),
    ))
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
/// SLA breach list: overdue open tasks with days-overdue (F-D03-039).
async fn sla_breach_csv(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let now = chrono::Utc::now()
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let matters = s
        .stores
        .core
        .list_matters(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let mut rows: Vec<String> = Vec::new();
    for m in &matters {
        let ts = s
            .stores
            .core
            .list_tasks(&u.claims.org, m.id)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
        for t in ts
            .iter()
            .filter(|t| t.status != "done" && t.due.as_deref().is_some_and(|d| d < now.as_str()))
        {
            let day = t.due.as_deref().unwrap_or("");
            rows.push(format!(
                "{},{},{},{},{},{}\n",
                m.id,
                csv_field(&m.number),
                t.id,
                csv_field(&t.title),
                csv_field(day),
                t.status
            ));
        }
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.sla_exported".into(),
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
    let mut csv = String::from("matter_id,matter_number,task_id,task_title,due,status\n");
    for r in rows {
        csv.push_str(&r);
    }
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

// ---------- compliance slice: vendor register + DPA ----------
#[derive(Debug, Deserialize, Validate)]
struct VendorReq {
    #[validate(length(min = 2, max = 200))]
    name: String,
    #[validate(length(max = 255))]
    service: Option<String>,
}
async fn vendor_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<VendorReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .vendor_create(
            &u.claims.org,
            b.name.trim(),
            b.service.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.vendor_created".into(),
            entity: "vendor".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn vendor_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .vendor_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "name": r.name, "service": r.service, "dpa": r.dpa_status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct VendorDpaReq {
    #[validate(length(min = 1, max = 20))]
    dpa: String,
}
async fn vendor_set_dpa(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<VendorDpaReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .vendor_set_dpa(&u.claims.org, id, b.dpa.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "dpa": b.dpa.trim()})))
}
async fn vendor_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .vendor_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: vendor register ----------

// ---------- compliance slice: incidents + CAPA ----------
#[derive(Debug, Deserialize, Validate)]
struct IncidentReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(min = 1, max = 20))]
    severity: Option<String>,
}
async fn incident_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<IncidentReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .incident_create(
            &u.claims.org,
            b.title.trim(),
            b.severity.as_deref().unwrap_or("medium"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.incident_created".into(),
            entity: "incident".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn incident_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .incident_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "severity": r.severity, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct IncidentStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn incident_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<IncidentStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .incident_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn incident_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .incident_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct CapaReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 255))]
    cause: Option<String>,
    #[validate(length(max = 255))]
    action: Option<String>,
}
async fn capa_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<CapaReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .capa_create(
            &u.claims.org,
            b.title.trim(),
            b.cause.as_deref().unwrap_or(""),
            b.action.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.capa_created".into(),
            entity: "capa".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn capa_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .capa_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "cause": r.cause, "action": r.action, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct CapaStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn capa_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<CapaStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .capa_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn capa_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .capa_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: incidents + CAPA ----------

// ---------- compliance slice: regulatory change tracker ----------
#[derive(Debug, Deserialize, Validate)]
struct RegChangeReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 120))]
    source: Option<String>,
    #[validate(length(max = 10))]
    effective_date: Option<String>,
}
async fn regchange_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<RegChangeReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .regchange_create(
            &u.claims.org,
            b.title.trim(),
            b.source.as_deref().unwrap_or(""),
            b.effective_date.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.regchange_created".into(),
            entity: "regchange".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn regchange_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .regchange_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "source": r.source, "effective_date": r.effective_date, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct RegChangeStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn regchange_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<RegChangeStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .regchange_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn regchange_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .regchange_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: regulatory change tracker ----------

// ---------- compliance slice: training assignments ----------
#[derive(Debug, Deserialize, Validate)]
struct TrainingReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 120))]
    assignee: Option<String>,
    #[validate(length(max = 10))]
    due_date: Option<String>,
}
async fn training_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<TrainingReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .training_create(
            &u.claims.org,
            b.title.trim(),
            b.assignee.as_deref().unwrap_or(""),
            b.due_date.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.training_created".into(),
            entity: "training".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn training_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .training_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "assignee": r.assignee, "due_date": r.due_date, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct TrainingStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn training_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<TrainingStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .training_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn training_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .training_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: training assignments ----------

// ---------- compliance slice: risk register + heatmap ----------
#[derive(Debug, Deserialize, Validate)]
struct RiskReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 60))]
    category: Option<String>,
    #[validate(range(min = 1, max = 5))]
    likelihood: Option<i64>,
    #[validate(range(min = 1, max = 5))]
    impact: Option<i64>,
    #[validate(length(max = 120))]
    owner: Option<String>,
}
async fn risk_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<RiskReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .risk_create(
            &u.claims.org,
            b.title.trim(),
            b.category.as_deref().unwrap_or("operational"),
            b.likelihood.unwrap_or(1),
            b.impact.unwrap_or(1),
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
            action: "compliance.risk_created".into(),
            entity: "risk".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn risk_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .risk_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(
            |r| json!({"id": r.id, "title": r.title, "category": r.category,
            "likelihood": r.likelihood, "impact": r.impact, "score": r.likelihood * r.impact,
            "owner": r.owner, "status": r.status})
        )
        .collect::<Vec<_>>())))
}
/// Risk heatmap (F-D11-038): 5x5 likelihood x impact counts over open risks.
async fn risk_heatmap(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .risk_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut matrix = [[0i64; 5]; 5];
    let mut total = 0i64;
    let mut high = 0i64;
    for r in &v {
        if r.status == "closed" {
            continue;
        }
        let (li, im) = (
            (r.likelihood.clamp(1, 5) - 1) as usize,
            (r.impact.clamp(1, 5) - 1) as usize,
        );
        matrix[li][im] += 1;
        total += 1;
        if r.likelihood * r.impact >= 12 {
            high += 1;
        }
    }
    Ok(Json(
        json!({"matrix": matrix, "total_open": total, "high_count": high}),
    ))
}
#[derive(Debug, Deserialize, Validate)]
struct RiskStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn risk_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<RiskStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .risk_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn risk_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .risk_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: risk register + heatmap ----------

// ---------- compliance slice: data-subject rights (DSAR) ----------
#[derive(Debug, Deserialize, Validate)]
struct DsarReq {
    #[validate(length(min = 2, max = 160))]
    subject: String,
    #[validate(length(min = 1, max = 20))]
    kind: Option<String>,
    #[validate(length(min = 2, max = 2000))]
    detail: String,
    #[validate(length(max = 10))]
    due_date: Option<String>,
}
async fn dsar_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DsarReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .dsar_create(
            &u.claims.org,
            b.subject.trim(),
            b.kind.as_deref().unwrap_or("access"),
            b.detail.trim(),
            b.due_date.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.dsar_created".into(),
            entity: "dsar".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn dsar_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .dsar_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "subject": r.subject, "kind": r.kind, "detail": r.detail, "due_date": r.due_date, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct DsarStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn dsar_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DsarStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .dsar_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn dsar_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .dsar_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: data-subject rights (DSAR) ----------

// ---------- compliance slice: ethics declarations ----------
#[derive(Debug, Deserialize, Validate)]
struct EthicsReq {
    #[validate(length(min = 2, max = 120))]
    declarant: String,
    #[validate(length(max = 4))]
    year: Option<String>,
    #[validate(length(min = 2, max = 2000))]
    statement: String,
}
async fn ethics_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<EthicsReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .ethics_create(
            &u.claims.org,
            b.declarant.trim(),
            b.year.as_deref().unwrap_or(""),
            b.statement.trim(),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.ethics_created".into(),
            entity: "ethics".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn ethics_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ethics_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "declarant": r.declarant, "year": r.year, "statement": r.statement, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct EthicsStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn ethics_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<EthicsStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .ethics_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn ethics_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .ethics_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: ethics declarations ----------

// ---------- compliance slice: gift register ----------
#[derive(Debug, Deserialize, Validate)]
struct GiftReq {
    #[validate(length(min = 1, max = 10))]
    direction: Option<String>,
    #[validate(length(min = 2, max = 255))]
    description: String,
    #[validate(length(max = 120))]
    counterpart: Option<String>,
    #[validate(length(max = 10))]
    given_at: Option<String>,
}
async fn gift_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<GiftReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .gift_create(
            &u.claims.org,
            b.direction.as_deref().unwrap_or("received"),
            b.description.trim(),
            b.counterpart.as_deref().unwrap_or(""),
            b.given_at.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.gift_created".into(),
            entity: "gift".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn gift_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .gift_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "direction": r.direction, "description": r.description, "counterpart": r.counterpart, "given_at": r.given_at, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct GiftStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn gift_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<GiftStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .gift_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn gift_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .gift_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: gift register ----------

// ---------- compliance slice: training evidence + audit findings ----------
#[derive(Debug, Deserialize, Validate)]
struct TevidenceReq {
    #[validate(length(min = 2, max = 500))]
    note: String,
}
async fn tevidence_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(tid): Path<i64>,
    Json(b): Json<TevidenceReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .tevidence_add(&u.claims.org, tid, b.note.trim())
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.tevidence_added".into(),
            entity: "tevidence".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn tevidence_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(tid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .tevidence_list(&u.claims.org, tid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "training_id": r.training_id, "note": r.note}))
        .collect::<Vec<_>>())))
}
async fn tevidence_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .tevidence_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct AfindingReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 60))]
    area: Option<String>,
    #[validate(length(min = 1, max = 20))]
    severity: Option<String>,
}
async fn afinding_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<AfindingReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .afinding_create(
            &u.claims.org,
            b.title.trim(),
            b.area.as_deref().unwrap_or("general"),
            b.severity.as_deref().unwrap_or("medium"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.afinding_created".into(),
            entity: "afinding".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn afinding_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .afinding_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "area": r.area, "severity": r.severity, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct AfindingStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn afinding_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<AfindingStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .afinding_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn afinding_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .afinding_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: training evidence + audit findings ----------

// ---------- compliance slice: whistleblowing + waivers ----------
#[derive(Debug, Deserialize, Validate)]
struct WhistleReq {
    #[validate(length(min = 1, max = 20))]
    channel: Option<String>,
    #[validate(length(max = 60))]
    category: Option<String>,
    #[validate(length(min = 2, max = 2000))]
    summary: String,
    #[validate(length(max = 160))]
    contact: Option<String>,
}
async fn whistle_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<WhistleReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .whistle_create(
            &u.claims.org,
            b.channel.as_deref().unwrap_or("web"),
            b.category.as_deref().unwrap_or("general"),
            b.summary.trim(),
            b.contact.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.whistle_created".into(),
            entity: "whistle".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn whistle_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .whistle_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| {
            let anon = r.contact.trim().is_empty();
            json!({"id": r.id, "channel": r.channel, "category": r.category,
                "summary": r.summary, "contact": if anon { "anonim".to_string() } else { r.contact.clone() },
                "status": r.status})
        })
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct WhistleStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn whistle_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<WhistleStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .whistle_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn whistle_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .whistle_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct WaiverReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(min = 2, max = 2000))]
    justification: String,
    #[validate(length(max = 120))]
    requested_by: Option<String>,
    #[validate(length(max = 10))]
    expires_at: Option<String>,
}
async fn waiver_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<WaiverReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .waiver_create(
            &u.claims.org,
            b.title.trim(),
            b.justification.trim(),
            b.requested_by.as_deref().unwrap_or(""),
            b.expires_at.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.waiver_created".into(),
            entity: "waiver".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn waiver_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .waiver_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "justification": r.justification, "requested_by": r.requested_by, "expires_at": r.expires_at, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct WaiverStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn waiver_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<WaiverStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .waiver_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn waiver_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .waiver_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: whistleblowing + waivers ----------

// ---------- compliance slice: policy library + control library ----------
#[derive(Debug, Deserialize, Validate)]
struct PolicyReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 20))]
    version: Option<String>,
    #[validate(length(min = 2, max = 200_000))]
    body: String,
}
async fn policy_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PolicyReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .policy_create(
            &u.claims.org,
            b.title.trim(),
            b.version.as_deref().unwrap_or("1.0"),
            b.body.trim(),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.policy_created".into(),
            entity: "policy".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn policy_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .policy_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    // Ack counts ride along so the UI shows coverage without N+1 calls.
    let mut out = Vec::with_capacity(v.len());
    for r in &v {
        let acks = s
            .stores
            .ops
            .policy_ack_list(&u.claims.org, r.id)
            .await
            .map_err(map_store)?;
        out.push(json!({"id": r.id, "title": r.title, "version": r.version,
            "status": r.status, "ack_count": acks.len()}));
    }
    Ok(Json(json!(out)))
}
#[derive(Debug, Deserialize, Validate)]
struct PolicyStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn policy_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<PolicyStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .policy_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn policy_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .policy_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
async fn policy_ack(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    let aid = s
        .stores
        .ops
        .policy_ack(&u.claims.org, id, &u.claims.sub)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "ack_id": aid})))
}
async fn policy_ack_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .policy_ack_list(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "policy_id": r.policy_id, "user_id": r.user_id}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct ControlReq {
    #[validate(length(min = 1, max = 40))]
    code: String,
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 60))]
    area: Option<String>,
    #[validate(length(min = 1, max = 20))]
    frequency: Option<String>,
}
async fn control_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ControlReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .control_create(
            &u.claims.org,
            b.code.trim(),
            b.title.trim(),
            b.area.as_deref().unwrap_or("general"),
            b.frequency.as_deref().unwrap_or("quarterly"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.control_created".into(),
            entity: "control".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn control_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .control_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "code": r.code, "title": r.title, "area": r.area, "frequency": r.frequency, "active": r.active}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct ControlActiveReq {
    active: bool,
}
async fn control_set_active(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<ControlActiveReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .control_set_active(&u.claims.org, id, b.active)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "active": b.active})))
}
async fn control_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .control_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: policy library + control library ----------

// ---------- compliance slice: control tests + audit plans ----------
#[derive(Debug, Deserialize, Validate)]
struct CtestReq {
    #[validate(length(max = 10))]
    tested_at: Option<String>,
    #[validate(length(min = 1, max = 20))]
    result: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn ctest_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(cid): Path<i64>,
    Json(b): Json<CtestReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let today = chrono::Utc::now()
        .naive_utc()
        .date()
        .format("%Y-%m-%d")
        .to_string();
    let id = s
        .stores
        .ops
        .ctest_add(
            &u.claims.org,
            cid,
            b.tested_at.as_deref().unwrap_or(&today),
            b.result.as_deref().unwrap_or("pass"),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.ctest_added".into(),
            entity: "ctest".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn ctest_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(cid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ctest_list(&u.claims.org, cid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "control_id": r.control_id, "tested_at": r.tested_at, "result": r.result, "note": r.note}))
        .collect::<Vec<_>>())))
}
async fn ctest_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .ctest_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
/// Control test scheduler view (F-D11-028): per active control, last test,
/// computed next due from frequency, and overdue flag. Never-tested
/// controls are due immediately (honest, no fabricated dates).
async fn controls_due(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    let today = chrono::Utc::now().naive_utc().date();
    let today_s = today.format("%Y-%m-%d").to_string();
    let mut out = Vec::new();
    for c in ops.control_list(org).await.map_err(map_store)? {
        if !c.active {
            continue;
        }
        let tests = ops.ctest_list(org, c.id).await.map_err(map_store)?;
        let last: Option<String> = tests.iter().map(|t| t.tested_at.clone()).max();
        let interval: i64 = match c.frequency.as_str() {
            "daily" => 1,
            "weekly" => 7,
            "monthly" => 30,
            "annual" => 365,
            _ => 90,
        };
        let (next_due, overdue) = match last.clone() {
            Some(d) => match chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d") {
                Ok(dt) => {
                    let due = dt + chrono::Duration::days(interval);
                    (due.format("%Y-%m-%d").to_string(), due < today)
                }
                Err(_) => (today_s.clone(), true),
            },
            None => (today_s.clone(), true),
        };
        out.push(json!({"id": c.id, "code": c.code, "title": c.title,
            "frequency": c.frequency, "last_tested_at": last,
            "next_due": next_due, "overdue": overdue}));
    }
    out.sort_by(|a, b| {
        b["overdue"]
            .as_bool()
            .cmp(&a["overdue"].as_bool())
            .then(a["next_due"].as_str().cmp(&b["next_due"].as_str()))
    });
    Ok(Json(json!(out)))
}
#[derive(Debug, Deserialize, Validate)]
struct AplanReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 20))]
    period: Option<String>,
    #[validate(length(max = 255))]
    scope: Option<String>,
}
async fn aplan_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<AplanReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .aplan_create(
            &u.claims.org,
            b.title.trim(),
            b.period.as_deref().unwrap_or(""),
            b.scope.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.aplan_created".into(),
            entity: "aplan".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn aplan_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .aplan_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "period": r.period, "scope": r.scope, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct AplanStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn aplan_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<AplanStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .aplan_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn aplan_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .aplan_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end compliance slice: control tests + audit plans ----------

// ---------- compliance slice: matter checklist + scorecard ----------
#[derive(Debug, Deserialize, Validate)]
struct ClistReq {
    #[validate(length(min = 2, max = 200))]
    item: String,
}
async fn clist_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(mid): Path<i64>,
    Json(b): Json<ClistReq>,
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
    let id = s
        .stores
        .ops
        .clist_add(&u.claims.org, mid, b.item.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id})))
}
async fn clist_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(mid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    s.stores
        .core
        .get_matter(&u.claims.org, mid)
        .await
        .map_err(|_| AppError::NotFound("matter".into()))?;
    let v = s
        .stores
        .ops
        .clist_list(&u.claims.org, mid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "matter_id": r.matter_id, "item": r.item, "done": r.done}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct ClistToggleReq {
    done: bool,
}
async fn clist_toggle(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<ClistToggleReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .clist_toggle(&u.claims.org, id, b.done)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "done": b.done})))
}
async fn clist_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .clist_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
/// Office compliance scorecard (F-D11-033): completion ratios derived from
/// live registers. `null` when a register is empty (no fabrication).
async fn compliance_scorecard(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    fn ratio(done: usize, total: usize) -> Option<f64> {
        if total == 0 {
            None
        } else {
            Some((done as f64 * 100.0 / total as f64 * 10.0).round() / 10.0)
        }
    }
    let mut parts: Vec<Value> = Vec::new();
    let mut sum = 0.0;
    let mut n = 0i64;
    let tr = ops.training_list(org).await.map_err(map_store)?;
    let r = ratio(
        tr.iter().filter(|x| x.status == "completed").count(),
        tr.len(),
    );
    if let Some(v) = r {
        sum += v;
        n += 1;
    }
    parts.push(json!({"area": "trainings_completed_pct", "value": r}));
    let rk = ops.risk_list(org).await.map_err(map_store)?;
    let r = ratio(
        rk.iter()
            .filter(|x| x.status == "mitigated" || x.status == "closed")
            .count(),
        rk.len(),
    );
    if let Some(v) = r {
        sum += v;
        n += 1;
    }
    parts.push(json!({"area": "risks_treated_pct", "value": r}));
    let af = ops.afinding_list(org).await.map_err(map_store)?;
    let r = ratio(af.iter().filter(|x| x.status == "closed").count(), af.len());
    if let Some(v) = r {
        sum += v;
        n += 1;
    }
    parts.push(json!({"area": "findings_closed_pct", "value": r}));
    let ap = ops.aplan_list(org).await.map_err(map_store)?;
    let r = ratio(
        ap.iter().filter(|x| x.status == "completed").count(),
        ap.len(),
    );
    if let Some(v) = r {
        sum += v;
        n += 1;
    }
    parts.push(json!({"area": "audit_plans_completed_pct", "value": r}));
    let ctl = ops.control_list(org).await.map_err(map_store)?;
    let active = ctl.iter().filter(|x| x.active).count();
    let r = ratio(active, ctl.len());
    if let Some(v) = r {
        sum += v;
        n += 1;
    }
    parts.push(json!({"area": "controls_active_pct", "value": r}));
    let overall = if n == 0 {
        None
    } else {
        Some((sum / n as f64 * 10.0).round() / 10.0)
    };
    Ok(Json(json!({"parts": parts, "overall": overall})))
}
// ---------- end compliance slice: matter checklist + scorecard ----------

// ---------- compliance slice: evidence vault + regulator pack ----------
#[derive(Debug, Deserialize, Validate)]
struct EvidenceReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(min = 1, max = 40))]
    register: String,
    register_id: i64,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn evidence_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<EvidenceReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .evidence_add(
            &u.claims.org,
            b.title.trim(),
            b.register.trim(),
            b.register_id,
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "compliance.evidence_added".into(),
            entity: "evidence".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
#[derive(Debug, Deserialize)]
struct EvidenceListQ {
    register: Option<String>,
    register_id: Option<i64>,
}
async fn evidence_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<EvidenceListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .evidence_list(&u.claims.org, q.register.as_deref(), q.register_id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "register": r.register_name, "register_id": r.register_id, "note": r.note}))
        .collect::<Vec<_>>())))
}
async fn evidence_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .evidence_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
/// Regulator inquiry response pack (F-D11-031): one CSV bundling posture
/// summary, open alerts, and per-register totals with generation metadata.
/// Export-gated and audited like all report exports.
async fn regulator_pack_export(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut csv = format!("regulator_response_pack,org_hash_generated_at\nmeta,,{now}\n\nsection,register,total,detail\n");
    let posture_rows: Vec<(&str, usize)> = vec![
        (
            "obligations",
            ops.pdp_obligation_list(org, None)
                .await
                .map_err(map_store)?
                .len(),
        ),
        (
            "consents",
            ops.consent_list(org, None).await.map_err(map_store)?.len(),
        ),
        ("ropa", ops.ropa_list(org).await.map_err(map_store)?.len()),
        ("dpia", ops.dpia_list(org).await.map_err(map_store)?.len()),
        (
            "breach",
            ops.breach_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "vendors",
            ops.vendor_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "incidents",
            ops.incident_list(org).await.map_err(map_store)?.len(),
        ),
        ("capa", ops.capa_list(org).await.map_err(map_store)?.len()),
        (
            "regchanges",
            ops.regchange_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "trainings",
            ops.training_list(org).await.map_err(map_store)?.len(),
        ),
        ("risks", ops.risk_list(org).await.map_err(map_store)?.len()),
        ("dsar", ops.dsar_list(org).await.map_err(map_store)?.len()),
        (
            "ethics",
            ops.ethics_list(org).await.map_err(map_store)?.len(),
        ),
        ("gifts", ops.gift_list(org).await.map_err(map_store)?.len()),
        (
            "findings",
            ops.afinding_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "whistle",
            ops.whistle_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "waivers",
            ops.waiver_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "policies",
            ops.policy_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "controls",
            ops.control_list(org).await.map_err(map_store)?.len(),
        ),
        (
            "evidence",
            ops.evidence_list(org, None, None)
                .await
                .map_err(map_store)?
                .len(),
        ),
    ];
    for (name, total) in posture_rows {
        csv.push_str(&format!("posture,{name},{total},\n"));
    }
    csv.push_str("\nkind,id,title\n");
    let today = chrono::Utc::now()
        .naive_utc()
        .date()
        .format("%Y-%m-%d")
        .to_string();
    for r in ops.dsar_list(org).await.map_err(map_store)? {
        if !r.due_date.trim().is_empty()
            && r.due_date.as_str() < today.as_str()
            && (r.status == "open" || r.status == "in_review")
        {
            csv.push_str(&format!(
                "alert,{},DSAR {} overdue\n",
                r.id,
                csv_field(&r.subject)
            ));
        }
    }
    for r in ops.risk_list(org).await.map_err(map_store)? {
        if r.status != "closed" && r.likelihood * r.impact >= 12 {
            csv.push_str(&format!(
                "alert,{},high risk {}\n",
                r.id,
                csv_field(&r.title)
            ));
        }
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.regulator_pack_exported".into(),
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
// ---------- end compliance slice: evidence vault + regulator pack ----------

// ---------- property slice: protocol books + custody log ----------
#[derive(Debug, Deserialize, Validate)]
struct PbookReq {
    #[validate(length(min = 1, max = 40))]
    book_no: String,
    #[validate(length(max = 4))]
    year: Option<String>,
}
async fn pbook_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PbookReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .pbook_create(
            &u.claims.org,
            b.book_no.trim(),
            b.year.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.pbook_created".into(),
            entity: "pbook".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn pbook_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .pbook_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "book_no": r.book_no, "year": r.book_year, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct PbookStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn pbook_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<PbookStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .pbook_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn pbook_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .pbook_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct PhandoverReq {
    #[validate(length(min = 1, max = 120))]
    from_holder: String,
    #[validate(length(min = 1, max = 120))]
    to_holder: String,
    #[validate(length(max = 10))]
    handed_at: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn phandover_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(bid): Path<i64>,
    Json(b): Json<PhandoverReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .phandover_add(
            &u.claims.org,
            bid,
            b.from_holder.trim(),
            b.to_holder.trim(),
            b.handed_at.as_deref().unwrap_or(""),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.phandover_added".into(),
            entity: "phandover".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn phandover_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(bid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .phandover_list(&u.claims.org, bid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "book_id": r.book_id, "from_holder": r.from_holder, "to_holder": r.to_holder, "handed_at": r.handed_at, "note": r.note}))
        .collect::<Vec<_>>())))
}
async fn phandover_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .phandover_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: protocol books + custody log ----------

// ---------- office slice: appointments + queue board ----------
#[derive(Debug, Deserialize, Validate)]
struct ApptReq {
    #[validate(length(min = 2, max = 160))]
    client_name: String,
    #[validate(length(max = 255))]
    purpose: Option<String>,
    #[validate(length(min = 16, max = 16))]
    scheduled_at: String,
}
async fn appt_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ApptReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .appt_create(
            &u.claims.org,
            b.client_name.trim(),
            b.purpose.as_deref().unwrap_or(""),
            b.scheduled_at.trim(),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.appt_created".into(),
            entity: "appt".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn appt_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .appt_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "client_name": r.client_name, "purpose": r.purpose, "scheduled_at": r.scheduled_at, "status": r.status}))
        .collect::<Vec<_>>())))
}
/// Queue & waiting-room board (F-D12-011): today's (or `?date=`) scheduled +
/// checked-in appointments in slot order. Pure derivation, no extra table.
#[derive(Debug, Deserialize)]
struct QueueQ {
    date: Option<String>,
}
async fn appt_queue(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<QueueQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let day: String = match q.date {
        Some(d) => {
            chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                .map_err(|_| AppError::Validation("date must be YYYY-MM-DD".into()))?;
            d
        }
        None => chrono::Utc::now()
            .naive_utc()
            .date()
            .format("%Y-%m-%d")
            .to_string(),
    };
    let v = s
        .stores
        .ops
        .appt_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut out: Vec<Value> = v
        .iter()
        .filter(|r| {
            r.scheduled_at.len() >= 10
                && &r.scheduled_at[..10] == day.as_str()
                && (r.status == "scheduled" || r.status == "checked_in")
        })
        .map(|r| json!({"id": r.id, "client_name": r.client_name, "purpose": r.purpose, "scheduled_at": r.scheduled_at, "status": r.status}))
        .collect();
    out.sort_by(|a, b| a["scheduled_at"].as_str().cmp(&b["scheduled_at"].as_str()));
    Ok(Json(json!({"date": day, "queue": out})))
}
#[derive(Debug, Deserialize, Validate)]
struct ApptStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn appt_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<ApptStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .appt_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn appt_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .appt_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: appointments + queue board ----------

// ---------- office slice: price list + quotations ----------
#[derive(Debug, Deserialize, Validate)]
struct PriceReq {
    #[validate(length(min = 1, max = 40))]
    code: String,
    #[validate(length(min = 2, max = 200))]
    name: String,
    amount_minor: i64,
}
async fn price_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PriceReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .price_create(&u.claims.org, b.code.trim(), b.name.trim(), b.amount_minor)
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.price_created".into(),
            entity: "price".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn price_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .price_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "code": r.code, "name": r.name, "amount_minor": r.amount_minor, "active": r.active}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct PriceActiveReq {
    active: bool,
}
async fn price_set_active(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<PriceActiveReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .price_set_active(&u.claims.org, id, b.active)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "active": b.active})))
}
async fn price_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .price_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Serialize, Validate)]
struct QuoteItemReq {
    #[validate(length(min = 1, max = 40))]
    code: String,
    #[validate(range(min = 1, max = 1000))]
    qty: i64,
}
#[derive(Debug, Deserialize, Validate)]
struct QuoteReq {
    #[validate(length(min = 2, max = 160))]
    client_name: String,
    #[validate(length(max = 60))]
    deed_kind: Option<String>,
    #[validate(length(min = 1, max = 50))]
    items: Vec<QuoteItemReq>,
}
async fn quote_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<QuoteReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let items: Vec<(String, i64)> = b
        .items
        .iter()
        .map(|i| (i.code.trim().to_string(), i.qty))
        .collect();
    let id = s
        .stores
        .ops
        .quote_create(
            &u.claims.org,
            b.client_name.trim(),
            b.deed_kind.as_deref().unwrap_or(""),
            &items,
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.quote_created".into(),
            entity: "quote".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn quote_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let ops = &s.stores.ops;
    let v = ops.quote_list(&u.claims.org).await.map_err(map_store)?;
    let mut out = Vec::with_capacity(v.len());
    for r in &v {
        let items = ops
            .quote_items(&u.claims.org, r.id)
            .await
            .map_err(map_store)?;
        out.push(json!({"id": r.id, "client_name": r.client_name, "deed_kind": r.deed_kind,
            "total_minor": r.total_minor, "status": r.status,
            "items": items.iter().map(|i| json!({"price_code": i.price_code, "qty": i.qty, "amount_minor": i.amount_minor})).collect::<Vec<_>>()}));
    }
    Ok(Json(json!(out)))
}
#[derive(Debug, Deserialize, Validate)]
struct QuoteStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn quote_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<QuoteStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .quote_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn quote_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .quote_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: price list + quotations ----------

// ---------- office slice: cash book + advances ----------
#[derive(Debug, Deserialize, Validate)]
struct CashReq {
    #[validate(length(min = 1, max = 10))]
    kind: String,
    amount_minor: i64,
    #[validate(length(max = 255))]
    note: Option<String>,
    #[validate(length(max = 10))]
    occurred_on: Option<String>,
}
async fn cash_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<CashReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .cash_add(
            &u.claims.org,
            b.kind.trim(),
            b.amount_minor,
            b.note.as_deref().unwrap_or(""),
            b.occurred_on.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.cash_added".into(),
            entity: "cash".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn cash_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .cash_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let balance: i64 = v
        .iter()
        .map(|r| {
            if r.kind == "in" {
                r.amount_minor
            } else {
                -r.amount_minor
            }
        })
        .sum();
    Ok(Json(json!({"balance_minor": balance,
        "entries": v.iter().map(|r| json!({"id": r.id, "kind": r.kind, "amount_minor": r.amount_minor, "note": r.note, "occurred_on": r.occurred_on})).collect::<Vec<_>>()})))
}
async fn cash_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .cash_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct AdvanceReq {
    #[validate(length(min = 2, max = 120))]
    holder: String,
    amount_minor: i64,
    #[validate(length(max = 255))]
    purpose: Option<String>,
}
async fn advance_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<AdvanceReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .advance_create(
            &u.claims.org,
            b.holder.trim(),
            b.amount_minor,
            b.purpose.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.advance_created".into(),
            entity: "advance".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn advance_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .advance_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "holder": r.holder, "amount_minor": r.amount_minor, "purpose": r.purpose, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct AdvanceStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn advance_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<AdvanceStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .advance_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn advance_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .advance_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: cash book + advances ----------

// ---------- office slice: inventory + meterai usage ----------
#[derive(Debug, Deserialize, Validate)]
struct StockReq {
    #[validate(length(min = 1, max = 40))]
    code: String,
    #[validate(length(min = 2, max = 200))]
    name: String,
    qty: i64,
    #[validate(length(max = 20))]
    unit: Option<String>,
    min_stock: Option<i64>,
    #[validate(length(max = 20))]
    kind: Option<String>,
}
async fn stock_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<StockReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .stock_create(
            &u.claims.org,
            b.code.trim(),
            b.name.trim(),
            b.qty,
            b.unit.as_deref().unwrap_or("pcs"),
            b.min_stock.unwrap_or(0),
            b.kind.as_deref().unwrap_or("general"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.stock_created".into(),
            entity: "inventory".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
#[derive(Debug, Deserialize)]
struct StockListQ {
    kind: Option<String>,
}
async fn stock_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<StockListQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .stock_list(&u.claims.org, q.kind.as_deref())
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(
            |r| json!({"id": r.id, "kind": r.kind, "code": r.code, "name": r.name, "qty": r.qty, "unit": r.unit,
            "min_stock": r.min_stock, "low": r.qty <= r.min_stock})
        )
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct StockAdjustReq {
    delta: i64,
}
async fn stock_adjust(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<StockAdjustReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    let qty = s
        .stores
        .ops
        .stock_adjust(&u.claims.org, id, b.delta)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "qty": qty})))
}
async fn stock_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .stock_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct StockUseReq {
    qty: i64,
    #[validate(length(max = 255))]
    need: Option<String>,
    #[validate(length(max = 10))]
    used_on: Option<String>,
}
async fn stock_use_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(iid): Path<i64>,
    Json(b): Json<StockUseReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .stock_use_add(
            &u.claims.org,
            iid,
            b.qty,
            b.need.as_deref().unwrap_or(""),
            b.used_on.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.usage_added".into(),
            entity: "usage".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn stock_use_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(iid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .stock_use_list(&u.claims.org, iid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "item_id": r.item_id, "qty": r.qty, "need": r.need, "used_on": r.used_on}))
        .collect::<Vec<_>>())))
}
async fn stock_use_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .stock_use_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: inventory + meterai usage ----------

// ---------- office slice: archive boxes ----------
#[derive(Debug, Deserialize, Validate)]
struct AboxReq {
    #[validate(length(min = 1, max = 40))]
    box_code: String,
    #[validate(length(max = 120))]
    location: Option<String>,
    #[validate(length(max = 255))]
    description: Option<String>,
}
async fn abox_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<AboxReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .abox_create(
            &u.claims.org,
            b.box_code.trim(),
            b.location.as_deref().unwrap_or(""),
            b.description.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.abox_created".into(),
            entity: "abox".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn abox_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .abox_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "box_code": r.box_code, "location": r.location, "description": r.description}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct AboxMoveReq {
    #[validate(length(max = 120))]
    location: String,
}
async fn abox_move(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<AboxMoveReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .abox_move(&u.claims.org, id, b.location.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "location": b.location.trim()})))
}
async fn abox_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .abox_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: archive boxes ----------

// ---------- office slice: performance dashboard + deed volume ----------
/// Office performance dashboard (F-D12-026): live counts across deeds,
/// appointments, quotations, visitors. Pure derivation, no synthetic KPI.
async fn office_performance(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    let deeds = ops.deed_list(org).await.map_err(map_store)?;
    let appts = ops.appt_list(org).await.map_err(map_store)?;
    let quotes = ops.quote_list(org).await.map_err(map_store)?;
    let visitors = ops.visitor_list(org).await.map_err(map_store)?;
    let mut deed_states: std::collections::BTreeMap<String, i64> =
        std::collections::BTreeMap::new();
    for d in &deeds {
        *deed_states.entry(d.state.clone()).or_insert(0) += 1;
    }
    let accepted_sum: i64 = quotes
        .iter()
        .filter(|q| q.status == "accepted")
        .map(|q| q.total_minor)
        .sum();
    Ok(Json(json!({
        "deeds_total": deeds.len() as i64,
        "deeds_by_state": deed_states,
        "appointments_total": appts.len() as i64,
        "appointments_done": appts.iter().filter(|x| x.status == "done").count() as i64,
        "quotations_total": quotes.len() as i64,
        "quotations_accepted_sum_minor": accepted_sum,
        "visitors_total": visitors.len() as i64,
        "visitors_inside": visitors.iter().filter(|x| x.status == "in").count() as i64,
    })))
}
/// Deed volume analytics (F-D12-027): deeds per `YYYY-MM` (or per state with
/// `?group=state`). Derived from repertorium timestamps.
#[derive(Debug, Deserialize)]
struct DeedVolQ {
    group: Option<String>,
}
async fn deed_volume(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<DeedVolQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let deeds = s
        .stores
        .ops
        .deed_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut by: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    match q.group.as_deref() {
        Some("state") => {
            for d in &deeds {
                *by.entry(d.state.clone()).or_insert(0) += 1;
            }
        }
        _ => {
            for d in &deeds {
                let m = if d.created.len() >= 7 {
                    d.created[..7].to_string()
                } else {
                    "unknown".to_string()
                };
                *by.entry(m).or_insert(0) += 1;
            }
        }
    }
    Ok(Json(
        json!({"group": q.group.clone().unwrap_or_else(|| "month".into()),
        "buckets": by.into_iter().map(|(k, v)| json!({"key": k, "count": v})).collect::<Vec<_>>()}),
    ))
}
// ---------- end office slice: performance dashboard + deed volume ----------

// ---------- office slice: visitor log + announcements ----------
#[derive(Debug, Deserialize, Validate)]
struct VisitorReq {
    #[validate(length(min = 2, max = 160))]
    name: String,
    #[validate(length(max = 255))]
    need: Option<String>,
    #[validate(length(min = 16, max = 16))]
    check_in: String,
}
async fn visitor_checkin(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<VisitorReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .visitor_checkin(
            &u.claims.org,
            b.name.trim(),
            b.need.as_deref().unwrap_or(""),
            b.check_in.trim(),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.visitor_checkin".into(),
            entity: "visitor".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn visitor_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .visitor_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "name": r.name, "need": r.need, "check_in": r.check_in, "check_out": r.check_out, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct VisitorCheckoutReq {
    #[validate(length(min = 16, max = 16))]
    check_out: String,
}
async fn visitor_checkout(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<VisitorCheckoutReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .visitor_checkout(&u.claims.org, id, b.check_out.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": "out"})))
}
async fn visitor_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .visitor_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct AnnounceReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 5000))]
    body: Option<String>,
    pinned: Option<bool>,
}
async fn announce_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<AnnounceReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .announce_create(
            &u.claims.org,
            b.title.trim(),
            b.body.as_deref().unwrap_or(""),
            b.pinned.unwrap_or(false),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.announce_created".into(),
            entity: "announce".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn announce_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .announce_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "body": r.body, "pinned": r.pinned, "active": r.active}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct AnnounceSetReq {
    pinned: Option<bool>,
    active: Option<bool>,
}
async fn announce_set(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<AnnounceSetReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .announce_set(&u.claims.org, id, b.pinned, b.active)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id})))
}
async fn announce_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .announce_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: visitor log + announcements ----------

// ---------- office slice: down-payments + klapper ----------
#[derive(Debug, Deserialize, Validate)]
struct DpReq {
    #[validate(length(min = 1, max = 40))]
    receipt_no: String,
    #[validate(length(min = 2, max = 160))]
    client_name: String,
    matter_id: Option<i64>,
    amount_minor: i64,
    #[validate(length(max = 10))]
    issued_on: Option<String>,
}
async fn dp_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DpReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if let Some(mid) = b.matter_id {
        s.stores
            .core
            .get_matter(&u.claims.org, mid)
            .await
            .map_err(|_| AppError::NotFound("matter".into()))?;
    }
    let id = s
        .stores
        .ops
        .dp_create(
            &u.claims.org,
            b.receipt_no.trim(),
            b.client_name.trim(),
            b.matter_id,
            b.amount_minor,
            b.issued_on.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.dp_created".into(),
            entity: "downpayment".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn dp_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .dp_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "receipt_no": r.receipt_no, "client_name": r.client_name,
            "matter_id": r.matter_id, "amount_minor": r.amount_minor, "issued_on": r.issued_on, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct DpStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn dp_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DpStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .dp_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn dp_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .dp_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
/// Klapper — alphabetical index (F-D12-002): clients, deed parties, and deed
/// numbers in one sorted list. Pure derivation, no extra table.
async fn klapper_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let mut out: Vec<Value> = Vec::new();
    for c in s
        .stores
        .core
        .list_clients(&u.claims.org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?
    {
        out.push(json!({"name": c.display_name, "kind": "klien", "reference": format!("klien #{}", c.id)}));
    }
    for p in s
        .stores
        .ops
        .ppat_list(&u.claims.org)
        .await
        .map_err(map_store)?
    {
        if !p.party_seller.trim().is_empty() {
            out.push(json!({"name": p.party_seller.clone(), "kind": "pihak", "reference": format!("akta {}", p.number)}));
        }
        if !p.party_buyer.trim().is_empty() {
            out.push(json!({"name": p.party_buyer.clone(), "kind": "pihak", "reference": format!("akta {}", p.number)}));
        }
    }
    for d in s
        .stores
        .ops
        .deed_list(&u.claims.org)
        .await
        .map_err(map_store)?
    {
        out.push(json!({"name": d.number.clone(), "kind": "akta", "reference": format!("akta #{}", d.id)}));
    }
    out.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
    });
    Ok(Json(json!(out)))
}
// ---------- end office slice: down-payments + klapper ----------

// ---------- office slice: witnesses + re-verification ----------
#[derive(Debug, Deserialize, Validate)]
struct WitnessReq {
    #[validate(length(min = 2, max = 160))]
    name: String,
    #[validate(length(max = 40))]
    id_number: Option<String>,
    #[validate(length(max = 40))]
    phone: Option<String>,
}
async fn witness_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<WitnessReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .witness_create(
            &u.claims.org,
            b.name.trim(),
            b.id_number.as_deref().unwrap_or(""),
            b.phone.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.witness_created".into(),
            entity: "witness".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn witness_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .witness_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "name": r.name, "id_number": r.id_number, "phone": r.phone, "active": r.active}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct WitnessActiveReq {
    active: bool,
}
async fn witness_set_active(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<WitnessActiveReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .witness_set_active(&u.claims.org, id, b.active)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "active": b.active})))
}
async fn witness_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .witness_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct RecheckReq {
    #[validate(length(min = 2, max = 160))]
    subject: String,
    #[validate(length(min = 1, max = 60))]
    method: Option<String>,
    #[validate(length(min = 1, max = 20))]
    result: Option<String>,
    #[validate(length(max = 10))]
    verified_on: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn recheck_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<RecheckReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .recheck_add(
            &u.claims.org,
            b.subject.trim(),
            b.method.as_deref().unwrap_or("document"),
            b.result.as_deref().unwrap_or("pass"),
            b.verified_on.as_deref().unwrap_or(""),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.recheck_added".into(),
            entity: "recheck".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn recheck_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .recheck_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "subject": r.subject, "method": r.method, "result": r.result, "verified_on": r.verified_on, "note": r.note}))
        .collect::<Vec<_>>())))
}
async fn recheck_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .recheck_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: witnesses + re-verification ----------

// ---------- office slice: hours, roster & leaves ----------
#[derive(Debug, Deserialize, Validate)]
struct HoursReq {
    #[validate(range(min = 0, max = 6))]
    weekday: i64,
    #[validate(length(min = 5, max = 5))]
    open_time: String,
    #[validate(length(min = 5, max = 5))]
    close_time: String,
    active: Option<bool>,
}
async fn hours_set(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<HoursReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .hours_set(
            &u.claims.org,
            b.weekday,
            b.open_time.trim(),
            b.close_time.trim(),
            b.active.unwrap_or(true),
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id})))
}
async fn hours_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .hours_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    const DAYS: [&str; 7] = ["Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu", "Ahad"];
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "weekday": r.weekday,
            "day": DAYS.get(r.weekday as usize).copied().unwrap_or("?"),
            "open_time": r.open_time, "close_time": r.close_time, "active": r.active}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct RosterReq {
    #[validate(length(min = 2, max = 160))]
    name: String,
    #[validate(length(max = 60))]
    role: Option<String>,
    #[validate(length(min = 1, max = 20))]
    shift: Option<String>,
}
async fn roster_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<RosterReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .roster_add(
            &u.claims.org,
            b.name.trim(),
            b.role.as_deref().unwrap_or("staff"),
            b.shift.as_deref().unwrap_or("pagi"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.roster_added".into(),
            entity: "roster".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn roster_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .roster_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "name": r.name, "role": r.role, "shift": r.shift, "active": r.active}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct RosterActiveReq {
    active: bool,
}
async fn roster_set_active(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<RosterActiveReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .roster_set_active(&u.claims.org, id, b.active)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "active": b.active})))
}
async fn roster_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .roster_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct LeaveReq {
    #[validate(length(min = 2, max = 120))]
    holder: String,
    #[validate(length(min = 1, max = 20))]
    kind: Option<String>,
    #[validate(length(max = 10))]
    from_date: Option<String>,
    #[validate(length(max = 10))]
    to_date: Option<String>,
    #[validate(length(max = 120))]
    substitute: Option<String>,
}
async fn leave_request(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<LeaveReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .leave_request(
            &u.claims.org,
            b.holder.trim(),
            b.kind.as_deref().unwrap_or("leave"),
            b.from_date.as_deref().unwrap_or(""),
            b.to_date.as_deref().unwrap_or(""),
            b.substitute.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.leave_requested".into(),
            entity: "leave".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn leave_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .leave_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "holder": r.holder, "kind": r.kind, "from_date": r.from_date, "to_date": r.to_date, "substitute": r.substitute, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct LeaveStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn leave_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<LeaveStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .leave_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn leave_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .leave_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: hours, roster & leaves ----------

// ---------- property slice: storage boxes + MPD transfers ----------
#[derive(Debug, Deserialize, Validate)]
struct SboxReq {
    #[validate(length(min = 1, max = 40))]
    box_code: String,
    #[validate(length(max = 120))]
    location: Option<String>,
    capacity: Option<i64>,
}
async fn sbox_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<SboxReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .sbox_create(
            &u.claims.org,
            b.box_code.trim(),
            b.location.as_deref().unwrap_or(""),
            b.capacity.unwrap_or(0),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.sbox_created".into(),
            entity: "sbox".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn sbox_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .sbox_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "box_code": r.box_code, "location": r.location, "capacity": r.capacity, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct SboxStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn sbox_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<SboxStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .sbox_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn sbox_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .sbox_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct MtransferReq {
    book_id: Option<i64>,
    #[validate(length(min = 2, max = 160))]
    recipient: String,
    #[validate(length(max = 10))]
    transfer_date: Option<String>,
    #[validate(length(max = 60))]
    receipt_no: Option<String>,
}
async fn mtransfer_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<MtransferReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if let Some(bid) = b.book_id {
        s.stores
            .ops
            .pbook_list(&u.claims.org)
            .await
            .map_err(map_store)?
            .iter()
            .find(|p| p.id == bid)
            .ok_or_else(|| AppError::NotFound("pbook".into()))?;
    }
    let id = s
        .stores
        .ops
        .mtransfer_create(
            &u.claims.org,
            b.book_id,
            b.recipient.trim(),
            b.transfer_date.as_deref().unwrap_or(""),
            b.receipt_no.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.mtransfer_created".into(),
            entity: "mtransfer".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn mtransfer_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .mtransfer_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "book_id": r.book_id, "recipient": r.recipient, "transfer_date": r.transfer_date, "receipt_no": r.receipt_no, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct MtransferStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn mtransfer_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<MtransferStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .mtransfer_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn mtransfer_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .mtransfer_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: storage boxes + MPD transfers ----------

// ---------- office slice: document checklist + office incidents ----------
#[derive(Debug, Deserialize, Validate)]
struct DclistReq {
    #[validate(length(min = 2, max = 200))]
    item: String,
}
async fn dclist_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(did): Path<i64>,
    Json(b): Json<DclistReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .core
        .get_doc(&u.claims.org, did)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let id = s
        .stores
        .ops
        .dclist_add(&u.claims.org, did, b.item.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id})))
}
async fn dclist_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(did): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    s.stores
        .core
        .get_doc(&u.claims.org, did)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let v = s
        .stores
        .ops
        .dclist_list(&u.claims.org, did)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "doc_id": r.doc_id, "item": r.item, "done": r.done}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct DclistToggleReq {
    done: bool,
}
async fn dclist_toggle(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DclistToggleReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .dclist_toggle(&u.claims.org, id, b.done)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "done": b.done})))
}
async fn dclist_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .dclist_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct OincidentReq {
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(min = 1, max = 20))]
    severity: Option<String>,
}
async fn oincident_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<OincidentReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .oincident_create(
            &u.claims.org,
            b.title.trim(),
            b.severity.as_deref().unwrap_or("medium"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.oincident_created".into(),
            entity: "oincident".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn oincident_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .oincident_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "title": r.title, "severity": r.severity, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct OincidentStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn oincident_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<OincidentStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .oincident_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn oincident_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .oincident_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: document checklist + office incidents ----------

// ---------- office slice: bank facilities + feedback ----------
#[derive(Debug, Deserialize, Validate)]
struct BankfReq {
    #[validate(length(min = 2, max = 120))]
    bank_name: String,
    #[validate(length(min = 2, max = 120))]
    facility: String,
    #[validate(length(max = 20))]
    rate_text: Option<String>,
    #[validate(length(max = 40))]
    tenor: Option<String>,
}
async fn bankf_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<BankfReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .bankf_create(
            &u.claims.org,
            b.bank_name.trim(),
            b.facility.trim(),
            b.rate_text.as_deref().unwrap_or(""),
            b.tenor.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.bankf_created".into(),
            entity: "bankf".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn bankf_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .bankf_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "bank_name": r.bank_name, "facility": r.facility, "rate_text": r.rate_text, "tenor": r.tenor, "active": r.active}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct BankfActiveReq {
    active: bool,
}
async fn bankf_set_active(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<BankfActiveReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .bankf_set_active(&u.claims.org, id, b.active)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "active": b.active})))
}
async fn bankf_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .bankf_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct FeedbackReq {
    #[validate(length(max = 120))]
    name: Option<String>,
    #[validate(length(min = 2, max = 2000))]
    message: String,
    rating: Option<i64>,
}
async fn feedback_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<FeedbackReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .feedback_add(
            &u.claims.org,
            b.name.as_deref().unwrap_or(""),
            b.message.trim(),
            b.rating.unwrap_or(0),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.feedback_added".into(),
            entity: "feedback".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn feedback_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .feedback_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "name": r.name, "message": r.message, "rating": r.rating, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct FeedbackStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn feedback_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<FeedbackStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .feedback_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn feedback_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .feedback_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: bank facilities + feedback ----------

// ---------- office slice: SOP library + backup checks ----------
#[derive(Debug, Deserialize, Validate)]
struct SopReq {
    #[validate(length(min = 1, max = 40))]
    code: String,
    #[validate(length(min = 2, max = 200))]
    title: String,
    #[validate(length(max = 20))]
    version: Option<String>,
    #[validate(length(min = 2, max = 200_000))]
    body: String,
}
async fn sop_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<SopReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .sop_create(
            &u.claims.org,
            b.code.trim(),
            b.title.trim(),
            b.version.as_deref().unwrap_or("1.0"),
            b.body.trim(),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.sop_created".into(),
            entity: "sop".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn sop_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .sop_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "code": r.code, "title": r.title, "version": r.version, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct SopStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn sop_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<SopStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .sop_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn sop_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .sop_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct BcheckReq {
    #[validate(length(min = 16, max = 16))]
    checked_at: String,
    #[validate(length(min = 1, max = 60))]
    method: Option<String>,
    #[validate(length(min = 1, max = 20))]
    result: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn bcheck_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<BcheckReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .bcheck_add(
            &u.claims.org,
            b.checked_at.trim(),
            b.method.as_deref().unwrap_or("dump"),
            b.result.as_deref().unwrap_or("pass"),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.bcheck_added".into(),
            entity: "bcheck".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn bcheck_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .bcheck_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "checked_at": r.checked_at, "method": r.method, "result": r.result, "note": r.note}))
        .collect::<Vec<_>>())))
}
async fn bcheck_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .bcheck_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: SOP library + backup checks ----------

// ---------- office slice: self-check runs + year-end seals ----------
#[derive(Debug, Deserialize, Validate)]
struct SelfcheckReq {
    #[validate(length(min = 1, max = 20))]
    period: String,
    passed_items: i64,
    total_items: i64,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn selfcheck_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<SelfcheckReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .selfcheck_add(
            &u.claims.org,
            b.period.trim(),
            b.passed_items,
            b.total_items,
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.selfcheck_added".into(),
            entity: "selfcheck".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn selfcheck_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .selfcheck_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| {
            let pct = if r.total_items > 0 {
                (r.passed_items as f64 * 100.0 / r.total_items as f64 * 10.0).round() / 10.0
            } else {
                0.0
            };
            json!({"id": r.id, "period": r.period, "passed_items": r.passed_items,
                "total_items": r.total_items, "pct": pct, "note": r.note})
        })
        .collect::<Vec<_>>())))
}
async fn selfcheck_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .selfcheck_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct SealyReq {
    #[validate(length(min = 4, max = 4))]
    year: String,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn sealy_prepare(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<SealyReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .sealy_prepare(
            &u.claims.org,
            b.year.trim(),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "office.sealy_prepared".into(),
            entity: "sealy".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn sealy_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .sealy_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "year": r.year, "book_count": r.book_count, "note": r.note, "status": r.status}))
        .collect::<Vec<_>>())))
}
async fn sealy_seal(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .sealy_seal(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": "sealed"})))
}
async fn sealy_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .sealy_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end office slice: self-check runs + year-end seals ----------

// ---------- property slice: PPAT KYC bundle + handover receipts ----------
#[derive(Debug, Deserialize, Validate)]
struct PpartyReq {
    #[validate(length(min = 1, max = 20))]
    role: Option<String>,
    #[validate(length(min = 2, max = 160))]
    name: String,
    #[validate(length(max = 40))]
    id_number: Option<String>,
}
async fn pparty_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<PpartyReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .pparty_add(
            &u.claims.org,
            pid,
            b.role.as_deref().unwrap_or("seller"),
            b.name.trim(),
            b.id_number.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id})))
}
async fn pparty_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .pparty_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    let done = v.iter().filter(|r| r.verified).count();
    Ok(Json(json!({"parties": v
        .iter()
        .map(|r| json!({"id": r.id, "ppat_id": r.ppat_id, "role": r.role, "name": r.name, "id_number": r.id_number, "verified": r.verified}))
        .collect::<Vec<_>>(),
        "verified_count": done, "total_count": v.len()})))
}
#[derive(Debug, Deserialize)]
struct PpartyVerifyReq {
    verified: bool,
}
async fn pparty_set_verified(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<PpartyVerifyReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .pparty_set_verified(&u.claims.org, id, b.verified)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "verified": b.verified})))
}
async fn pparty_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .pparty_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct ChandoverReq {
    #[validate(length(min = 2, max = 160))]
    handed_to: String,
    #[validate(length(max = 10))]
    handed_at: Option<String>,
    #[validate(length(max = 60))]
    receipt_no: Option<String>,
}
async fn chandover_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<ChandoverReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .chandover_add(
            &u.claims.org,
            pid,
            b.handed_to.trim(),
            b.handed_at.as_deref().unwrap_or(""),
            b.receipt_no.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.chandover_added".into(),
            entity: "chandover".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn chandover_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .chandover_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "ppat_id": r.ppat_id, "handed_to": r.handed_to, "handed_at": r.handed_at, "receipt_no": r.receipt_no, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct ChandoverStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn chandover_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<ChandoverStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .chandover_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn chandover_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .chandover_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: PPAT KYC bundle + handover receipts ----------

// ---------- property slice: APHT/SKMHT charges + expiry monitor ----------
#[derive(Debug, Deserialize, Validate)]
struct ChargeReq {
    #[validate(length(min = 1, max = 20))]
    kind: Option<String>,
    #[validate(length(min = 2, max = 160))]
    creditor: String,
    ceiling_minor: Option<i64>,
    #[validate(length(max = 10))]
    valid_until: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn charge_upsert(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<ChargeReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .charge_upsert(
            &u.claims.org,
            pid,
            b.kind.as_deref().unwrap_or("apht"),
            b.creditor.trim(),
            b.ceiling_minor.unwrap_or(0),
            b.valid_until.as_deref().unwrap_or(""),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.charge_saved".into(),
            entity: "charge".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn charge_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .charge_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(
            |r| json!({"id": r.id, "ppat_id": r.ppat_id, "kind": r.kind, "creditor": r.creditor,
            "ceiling_minor": r.ceiling_minor, "valid_until": r.valid_until, "note": r.note})
        )
        .collect::<Vec<_>>())))
}
/// SKMHT/APHT expiry monitor (F-D11-009): charges expiring within `days`
/// (default 30). Pure date math over stored validity dates.
#[derive(Debug, Deserialize)]
struct ChargeDueQ {
    days: Option<i64>,
}
async fn charge_due(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<ChargeDueQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .charge_due(&u.claims.org, q.days.unwrap_or(30))
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"days": q.days.unwrap_or(30).clamp(0, 365),
        "due": v.iter().map(|r| json!({"id": r.id, "ppat_id": r.ppat_id, "kind": r.kind, "creditor": r.creditor, "valid_until": r.valid_until})).collect::<Vec<_>>()})))
}
async fn charge_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .charge_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: APHT/SKMHT charges + expiry monitor ----------

// ---------- property slice: zones + tax worksheets ----------
#[derive(Debug, Deserialize, Validate)]
struct ZoneReq {
    #[validate(length(min = 1, max = 40))]
    area_code: String,
    #[validate(length(min = 2, max = 200))]
    area_name: String,
    njop_minor: i64,
}
async fn zone_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<ZoneReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .zone_create(
            &u.claims.org,
            b.area_code.trim(),
            b.area_name.trim(),
            b.njop_minor,
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.zone_created".into(),
            entity: "zone".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn zone_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .zone_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "area_code": r.area_code, "area_name": r.area_name, "njop_minor": r.njop_minor}))
        .collect::<Vec<_>>())))
}
async fn zone_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .zone_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
/// PPh worksheet (F-D11-018): final 2.5% of gross transfer value for
/// land/building transfers. Documented assumption; computed server-side.
#[derive(Debug, Deserialize)]
struct PphQ {
    value_minor: Option<i64>,
}
async fn worksheet_pph(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<PphQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = q.value_minor.unwrap_or(0);
    if v <= 0 {
        return Err(AppError::Validation("value_minor must be positive".into()));
    }
    let tax = (v as i128 * 25 / 1000).min(i64::MAX as i128) as i64;
    Ok(Json(
        json!({"value_minor": v, "rate_pct": 2.5, "tax_minor": tax, "rule": "PPh final 2.5% x nilai bruto pengalihan"}),
    ))
}
/// BPHTB worksheet (F-D11-019): 5% x (NPOP - NPOPTKP), floor zero.
/// Default NPOPTKP 60jt follows the statutory minimum; pass explicit
/// npoptkp for regional variants.
#[derive(Debug, Deserialize)]
struct BphtbQ {
    npop: Option<i64>,
    npoptkp: Option<i64>,
}
async fn worksheet_bphtb(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<BphtbQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let npop = q.npop.unwrap_or(0);
    let npoptkp = q.npoptkp.unwrap_or(60_000_000);
    if npop <= 0 || npoptkp < 0 {
        return Err(AppError::Validation("npop/npoptkp invalid".into()));
    }
    let base = (npop - npoptkp).max(0);
    let tax = (base as i128 * 5 / 100).min(i64::MAX as i128) as i64;
    Ok(Json(
        json!({"npop": npop, "npoptkp": npoptkp, "base_minor": base,
        "rate_pct": 5.0, "tax_minor": tax, "rule": "BPHTB 5% x (NPOP - NPOPTKP)"}),
    ))
}
// ---------- end property slice: zones + tax worksheets ----------

// ---------- property slice: PBB checks + PPAT fee worksheet ----------
#[derive(Debug, Deserialize, Validate)]
struct PbbReq {
    #[validate(length(min = 2, max = 40))]
    nop: String,
    #[validate(length(min = 4, max = 4))]
    tax_year: String,
    #[validate(length(min = 1, max = 20))]
    result: Option<String>,
    #[validate(length(max = 10))]
    checked_on: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn pbb_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PbbReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .pbb_add(
            &u.claims.org,
            b.nop.trim(),
            b.tax_year.trim(),
            b.result.as_deref().unwrap_or("unknown"),
            b.checked_on.as_deref().unwrap_or(""),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.pbb_added".into(),
            entity: "pbb".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn pbb_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .pbb_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "nop": r.nop, "tax_year": r.tax_year, "result": r.result, "checked_on": r.checked_on, "note": r.note}))
        .collect::<Vec<_>>())))
}
async fn pbb_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .pbb_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
/// PPAT fee worksheet (F-D12-036): honorarium capped at 1% of transaction
/// value per IPPAT schedule. Documented cap; computed server-side.
#[derive(Debug, Deserialize)]
struct PpatFeeQ {
    value_minor: Option<i64>,
}
async fn worksheet_ppat_fee(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<PpatFeeQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = q.value_minor.unwrap_or(0);
    if v <= 0 {
        return Err(AppError::Validation("value_minor must be positive".into()));
    }
    let fee = (v as i128 / 100).min(i64::MAX as i128) as i64;
    Ok(Json(
        json!({"value_minor": v, "rate_pct": 1.0, "fee_minor": fee, "rule": "Honorarium PPAT maks 1% x nilai transaksi"}),
    ))
}
// ---------- end property slice: PBB checks + PPAT fee worksheet ----------

// ---------- property slice: land split + merge trackers ----------
#[derive(Debug, Deserialize, Validate)]
struct LsplitReq {
    #[validate(length(min = 2, max = 120))]
    source_parcel: String,
    #[validate(length(min = 2, max = 1000))]
    new_parcels: String,
    #[validate(length(max = 10))]
    split_date: Option<String>,
}
async fn lsplit_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<LsplitReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .lsplit_create(
            &u.claims.org,
            b.source_parcel.trim(),
            b.new_parcels.trim(),
            b.split_date.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.lsplit_created".into(),
            entity: "lsplit".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn lsplit_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .lsplit_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "source_parcel": r.source_parcel, "new_parcels": r.new_parcels, "split_date": r.split_date, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct LandStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn lsplit_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<LandStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .lsplit_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn lsplit_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .lsplit_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct LmergeReq {
    #[validate(length(min = 2, max = 1000))]
    source_parcels: String,
    #[validate(length(min = 2, max = 120))]
    result_parcel: String,
    #[validate(length(max = 10))]
    merge_date: Option<String>,
}
async fn lmerge_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<LmergeReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .lmerge_create(
            &u.claims.org,
            b.source_parcels.trim(),
            b.result_parcel.trim(),
            b.merge_date.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.lmerge_created".into(),
            entity: "lmerge".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn lmerge_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .lmerge_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "source_parcels": r.source_parcels, "result_parcel": r.result_parcel, "merge_date": r.merge_date, "status": r.status}))
        .collect::<Vec<_>>())))
}
async fn lmerge_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<LandStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .lmerge_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn lmerge_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .lmerge_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: land split + merge trackers ----------

// ---------- property slice: dispute flags + boundary checks ----------
#[derive(Debug, Deserialize, Validate)]
struct LdisputeReq {
    #[validate(length(min = 2, max = 120))]
    parcel_ref: String,
    #[validate(length(min = 1, max = 40))]
    kind: Option<String>,
    #[validate(length(min = 2, max = 2000))]
    description: String,
}
async fn ldispute_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<LdisputeReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .ldispute_create(
            &u.claims.org,
            b.parcel_ref.trim(),
            b.kind.as_deref().unwrap_or("sengketa"),
            b.description.trim(),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.ldispute_created".into(),
            entity: "ldispute".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn ldispute_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ldispute_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "parcel_ref": r.parcel_ref, "kind": r.kind, "description": r.description, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct LdisputeStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn ldispute_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<LdisputeStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .ldispute_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn ldispute_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .ldispute_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct BndryReq {
    #[validate(length(min = 2, max = 120))]
    parcel_ref: String,
    declared_area: i64,
    measured_area: i64,
    #[validate(length(max = 10))]
    checked_on: Option<String>,
}
async fn bndry_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<BndryReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .bndry_add(
            &u.claims.org,
            b.parcel_ref.trim(),
            b.declared_area,
            b.measured_area,
            b.checked_on.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.bndry_added".into(),
            entity: "bndry".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn bndry_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .bndry_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "parcel_ref": r.parcel_ref, "declared_area": r.declared_area, "measured_area": r.measured_area, "result": r.result, "checked_on": r.checked_on}))
        .collect::<Vec<_>>())))
}
async fn bndry_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .bndry_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: dispute flags + boundary checks ----------

// ---------- property slice: deed consent checks (marital / heir) ----------
#[derive(Debug, Deserialize, Validate)]
struct DconsentReq {
    #[validate(length(min = 1, max = 20))]
    kind: Option<String>,
    #[validate(length(min = 2, max = 160))]
    party: String,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn dconsent_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<DconsentReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .dconsent_add(
            &u.claims.org,
            pid,
            b.kind.as_deref().unwrap_or("marital"),
            b.party.trim(),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.dconsent_added".into(),
            entity: "dconsent".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn dconsent_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .dconsent_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "ppat_id": r.ppat_id, "kind": r.kind, "party": r.party, "verified": r.verified, "note": r.note}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct DconsentVerifyReq {
    verified: bool,
}
async fn dconsent_verify(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DconsentVerifyReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .dconsent_verify(&u.claims.org, id, b.verified)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "verified": b.verified})))
}
async fn dconsent_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .dconsent_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: deed consent checks ----------

// ---------- property slice: land history + unit handover (BAST) ----------
#[derive(Debug, Deserialize, Validate)]
struct LhistReq {
    #[validate(length(min = 2, max = 120))]
    parcel_ref: String,
    #[validate(length(min = 1, max = 40))]
    event: Option<String>,
    #[validate(length(min = 2, max = 2000))]
    detail: String,
    #[validate(length(max = 10))]
    event_date: Option<String>,
}
async fn lhist_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<LhistReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .lhist_add(
            &u.claims.org,
            b.parcel_ref.trim(),
            b.event.as_deref().unwrap_or("alas_hak"),
            b.detail.trim(),
            b.event_date.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.lhist_added".into(),
            entity: "lhist".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
#[derive(Debug, Deserialize)]
struct LhistQ {
    parcel_ref: String,
}
async fn lhist_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<LhistQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .lhist_list(&u.claims.org, q.parcel_ref.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "parcel_ref": r.parcel_ref, "event": r.event, "detail": r.detail, "event_date": r.event_date}))
        .collect::<Vec<_>>())))
}
async fn lhist_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .lhist_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct UhoReq {
    #[validate(length(min = 2, max = 120))]
    unit_ref: String,
    #[validate(length(min = 2, max = 160))]
    handed_from: String,
    #[validate(length(min = 2, max = 160))]
    handed_to: String,
    #[validate(length(min = 2, max = 60))]
    bast_no: String,
    #[validate(length(max = 10))]
    handed_on: Option<String>,
}
async fn uho_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<UhoReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .uho_add(
            &u.claims.org,
            b.unit_ref.trim(),
            b.handed_from.trim(),
            b.handed_to.trim(),
            b.bast_no.trim(),
            b.handed_on.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.uho_added".into(),
            entity: "uho".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn uho_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .uho_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "unit_ref": r.unit_ref, "handed_from": r.handed_from, "handed_to": r.handed_to, "bast_no": r.bast_no, "handed_on": r.handed_on, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct UhoStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn uho_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<UhoStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .uho_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn uho_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .uho_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: land history + unit handover ----------

// ---------- property slice: PPAT monthly report + dashboard ----------
#[derive(Debug, Deserialize)]
struct PpatMonthQ {
    month: Option<String>,
}
async fn ppat_monthly_rep(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<PpatMonthQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let month = q
        .month
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m").to_string());
    let v = s
        .stores
        .ops
        .ppat_monthly(&u.claims.org, month.trim())
        .await
        .map_err(map_store)?;
    let total: i64 = v.iter().map(|r| r.n).sum();
    Ok(Json(json!({"month": month.trim(),
        "total": total,
        "rows": v.iter().map(|r| json!({"kind": r.kind, "status": r.status, "n": r.n})).collect::<Vec<_>>()})))
}
async fn ppat_dashboard_rep(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let d = s
        .stores
        .ops
        .ppat_dashboard(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"total_deeds": d.total_deeds,
        "by_kind": d.by_kind.iter().map(|r| json!({"kind": r.kind, "n": r.n})).collect::<Vec<_>>(),
        "unverified_consents": d.unverified_consents,
        "charges_due_30": d.charges_due_30,
        "open_disputes": d.open_disputes,
        "pending_handovers": d.pending_handovers})))
}
// ---------- end property slice: PPAT monthly report + dashboard ----------

// ---------- property slice: foreign guardrail + notary handoff ----------
/// Foreign-ownership guardrail (F-D13-035): pure statutory matrix over
/// (subject, land-right). No external lookup; blocks what UUPA forbids:
/// WNA may hold Hak Pakai only; foreign-owned (PMA) companies may hold
/// HGB/Hak Pakai but never SHM; WNI/unrestricted subjects pass through.
#[derive(Debug, Deserialize)]
struct FguardQ {
    subject: String,
    right: String,
}
async fn fguard_check(
    State(_s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<FguardQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &_s).await?;
    authorize(&u, Permission::View)?;
    let subject = q.subject.trim().to_lowercase();
    let right = q.right.trim().to_lowercase();
    if !matches!(
        subject.as_str(),
        "wni" | "wna" | "pma" | "pmnd" | "badan_wni"
    ) {
        return Err(AppError::Validation(
            "subject wni|wna|pma|pmnd|badan_wni".into(),
        ));
    }
    if !matches!(
        right.as_str(),
        "shm" | "hgb" | "hak_pakai" | "hak_guna_usaha" | "girik"
    ) {
        return Err(AppError::Validation(
            "right shm|hgb|hak_pakai|hak_guna_usaha|girik".into(),
        ));
    }
    let (allowed, rule) = match (subject.as_str(), right.as_str()) {
        ("wna", "hak_pakai") => (true, "WNA hanya dapat memegang Hak Pakai (UUPA)."),
        ("wna", _) => (
            false,
            "WNA dilarang memegang hak ini; alihkan ke Hak Pakai.",
        ),
        ("pma", "hgb") | ("pma", "hak_pakai") => (true, "PMA dapat memegang HGB/Hak Pakai."),
        ("pma", _) => (
            false,
            "PMA dilarang memegang SHM/hak adat; gunakan HGB/Hak Pakai.",
        ),
        (_, "shm") if subject == "wni" || subject == "badan_wni" => {
            (true, "SHM hanya untuk subjek WNI.")
        }
        (_, "girik") => (
            true,
            "Girik/adat perlu dikonversi sebelum peralihan (catat di riwayat).",
        ),
        _ => (
            true,
            "Kombinasi lolos guardrail; verifikasi dokumen tetap wajib.",
        ),
    };
    Ok(Json(
        json!({"subject": subject, "right": right, "allowed": allowed, "rule": rule}),
    ))
}
#[derive(Debug, Deserialize, Validate)]
struct NhoffReq {
    #[validate(length(min = 2, max = 160))]
    doc_name: String,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn nhoff_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<NhoffReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .nhoff_add(
            &u.claims.org,
            pid,
            b.doc_name.trim(),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.nhoff_added".into(),
            entity: "nhoff".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn nhoff_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .nhoff_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "ppat_id": r.ppat_id, "doc_name": r.doc_name, "received": r.received, "note": r.note}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct NhoffRecvReq {
    received: bool,
}
async fn nhoff_set_received(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<NhoffRecvReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    s.stores
        .ops
        .nhoff_set_received(&u.claims.org, id, b.received)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "received": b.received})))
}
async fn nhoff_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .nhoff_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: foreign guardrail + notary handoff ----------

// ---------- property slice: girik intake + lease terms ----------
#[derive(Debug, Deserialize, Validate)]
struct GirikReq {
    #[validate(length(min = 2, max = 120))]
    letter_c_ref: String,
    #[validate(length(min = 2, max = 160))]
    holder: String,
    area_minor: Option<i64>,
    #[validate(length(max = 120))]
    village: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn girik_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<GirikReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .girik_add(
            &u.claims.org,
            b.letter_c_ref.trim(),
            b.holder.trim(),
            b.area_minor.unwrap_or(0),
            b.village.as_deref().unwrap_or(""),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.girik_added".into(),
            entity: "girik".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn girik_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .girik_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "letter_c_ref": r.letter_c_ref, "holder": r.holder, "area_minor": r.area_minor, "village": r.village, "status": r.status, "note": r.note}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct GirikStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn girik_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<GirikStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .girik_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn girik_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .girik_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct LeaseReq {
    monthly_minor: i64,
    duration_months: i64,
    #[validate(length(max = 10))]
    start_date: Option<String>,
    deposit_minor: Option<i64>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn lease_upsert(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<LeaseReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .lease_upsert(
            &u.claims.org,
            pid,
            b.monthly_minor,
            b.duration_months,
            b.start_date.as_deref().unwrap_or(""),
            b.deposit_minor.unwrap_or(0),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.lease_saved".into(),
            entity: "lease".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn lease_get(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    match s
        .stores
        .ops
        .lease_get(&u.claims.org, pid)
        .await
        .map_err(map_store)?
    {
        Some(r) => Ok(Json(
            json!({"id": r.id, "ppat_id": r.ppat_id, "monthly_minor": r.monthly_minor,
            "duration_months": r.duration_months, "start_date": r.start_date, "deposit_minor": r.deposit_minor,
            "total_minor": r.monthly_minor.saturating_mul(r.duration_months), "note": r.note}),
        )),
        None => Err(AppError::NotFound("lease".into())),
    }
}
async fn lease_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .lease_delete(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: girik intake + lease terms ----------

// ---------- property slice: developer batches + unit stock ----------
#[derive(Debug, Deserialize, Validate)]
struct DbatchReq {
    #[validate(length(min = 2, max = 160))]
    developer: String,
    #[validate(length(min = 2, max = 60))]
    batch_no: String,
    total_units: Option<i64>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn dbatch_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DbatchReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .dbatch_add(
            &u.claims.org,
            b.developer.trim(),
            b.batch_no.trim(),
            b.total_units.unwrap_or(0),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.dbatch_added".into(),
            entity: "dbatch".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn dbatch_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .dbatch_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "developer": r.developer, "batch_no": r.batch_no, "total_units": r.total_units, "status": r.status, "note": r.note}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct DbatchStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn dbatch_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DbatchStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .dbatch_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn dbatch_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .dbatch_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct DunitReq {
    batch_id: Option<i64>,
    #[validate(length(min = 2, max = 60))]
    unit_code: String,
    #[validate(length(min = 1, max = 40))]
    unit_type: Option<String>,
    price_minor: Option<i64>,
}
async fn dunit_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<DunitReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .dunit_add(
            &u.claims.org,
            b.batch_id,
            b.unit_code.trim(),
            b.unit_type.as_deref().unwrap_or("rumah"),
            b.price_minor.unwrap_or(0),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.dunit_added".into(),
            entity: "dunit".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
#[derive(Debug, Deserialize)]
struct DunitQ {
    batch_id: Option<i64>,
}
async fn dunit_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<DunitQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .dunit_list(&u.claims.org, q.batch_id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "batch_id": r.batch_id, "unit_code": r.unit_code, "unit_type": r.unit_type, "price_minor": r.price_minor, "status": r.status}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct DunitStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn dunit_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<DunitStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .dunit_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn dunit_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .dunit_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: developer batches + unit stock ----------

// ---------- property slice: KPR packs + protocol register ----------
#[derive(Debug, Deserialize, Validate)]
struct KprReq {
    #[validate(length(min = 2, max = 120))]
    bank: String,
    amount_minor: Option<i64>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn kpr_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<KprReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .kpr_add(
            &u.claims.org,
            pid,
            b.bank.trim(),
            b.amount_minor.unwrap_or(0),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.kpr_added".into(),
            entity: "kpr".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn kpr_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .kpr_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "ppat_id": r.ppat_id, "bank": r.bank, "amount_minor": r.amount_minor, "status": r.status, "note": r.note}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct KprStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn kpr_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<KprStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .kpr_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn kpr_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .kpr_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
#[derive(Debug, Deserialize, Validate)]
struct PregReq {
    #[validate(length(max = 10))]
    entry_date: Option<String>,
    #[validate(length(min = 2, max = 120))]
    book_ref: String,
    #[validate(length(max = 120))]
    deed_no: Option<String>,
    #[validate(length(min = 2, max = 160))]
    party: String,
    #[validate(length(min = 1, max = 10))]
    direction: Option<String>,
}
async fn preg_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<PregReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .preg_add(
            &u.claims.org,
            b.entry_date.as_deref().unwrap_or(""),
            b.book_ref.trim(),
            b.deed_no.as_deref().unwrap_or(""),
            b.party.trim(),
            b.direction.as_deref().unwrap_or("in"),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.preg_added".into(),
            entity: "preg".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn preg_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .preg_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "entry_date": r.entry_date, "book_ref": r.book_ref, "deed_no": r.deed_no, "party": r.party, "direction": r.direction}))
        .collect::<Vec<_>>())))
}
async fn preg_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .preg_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: KPR packs + protocol register ----------

// ---------- property slice: AJB drafts + PPAT audit trail ----------
#[derive(Debug, Deserialize, Validate)]
struct AjbdReq {
    #[validate(length(max = 120))]
    template_name: Option<String>,
    #[validate(length(min = 20, max = 20000))]
    body: String,
    price_minor: Option<i64>,
}
async fn ajbd_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<AjbdReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .ajbd_add(
            &u.claims.org,
            pid,
            b.template_name.as_deref().unwrap_or(""),
            b.body.trim(),
            b.price_minor.unwrap_or(0),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.ajbd_added".into(),
            entity: "ajbd".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn ajbd_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .ajbd_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| {
            let preview: String = r.body.chars().take(160).collect();
            json!({"id": r.id, "ppat_id": r.ppat_id, "template_name": r.template_name, "preview": preview, "price_minor": r.price_minor})
        })
        .collect::<Vec<_>>())))
}
async fn ajbd_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .ajbd_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
/// PPAT audit trail (F-D13-038): consolidated property.* action feed.
/// Same role gate as the global audit list (owner/admin/auditor only).
const PPAT_AUDIT_ENTITIES: &[&str] = &[
    "charge", "lsplit", "lmerge", "ldispute", "bndry", "dconsent", "lhist", "uho", "girik",
    "lease", "dbatch", "dunit", "kpr", "preg", "nhoff", "ajbd",
];
async fn ppat_audit_trail(
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
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let mut all = Vec::new();
    for ent in PPAT_AUDIT_ENTITIES {
        let mut v = s
            .stores
            .ops
            .audit_list(&u.claims.org, Some(ent), limit)
            .await
            .map_err(map_store)?;
        all.append(&mut v);
    }
    all.sort_by_key(|a: &legalrust_database::AuditRow| -a.id);
    all.truncate(limit as usize);
    Ok(Json(json!({"entities": PPAT_AUDIT_ENTITIES,
        "rows": all.iter().map(|a| json!({"id": a.id, "actor": a.actor, "action": a.action, "entity": a.entity, "entity_id": a.entity_id, "created": a.created})).collect::<Vec<_>>()})))
}
// ---------- end property slice: AJB drafts + PPAT audit trail ----------

// ---------- property slice: certificate scan vault ----------
#[derive(Debug, Deserialize, Validate)]
struct ClinkReq {
    doc_id: i64,
    #[validate(length(max = 120))]
    label: Option<String>,
}
async fn clink_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
    Json(b): Json<ClinkReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    // The scan must exist and belong to this org (cross-store ownership check).
    let (d, _) = s
        .stores
        .core
        .get_doc(&u.claims.org, b.doc_id)
        .await
        .map_err(|_| AppError::NotFound("document".into()))?;
    let id = s
        .stores
        .ops
        .clink_add(
            &u.claims.org,
            pid,
            b.doc_id,
            &d.name,
            b.label.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.clink_added".into(),
            entity: "clink".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
async fn clink_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(pid): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .clink_list(&u.claims.org, pid)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "ppat_id": r.ppat_id, "doc_id": r.doc_id, "doc_name": r.doc_name, "label": r.label}))
        .collect::<Vec<_>>())))
}
async fn clink_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .clink_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: certificate scan vault ----------

// ---------- property slice: BPN coordination tracker (manual mode) ----------
// Manual ledger for BPN touchpoints (verification requests, queue numbers,
// response archive). Automatic sync stays BLOCKED on BPN credentials;
// every row records what the office filed/received by hand.
#[derive(Debug, Deserialize, Validate)]
struct BpnReq {
    #[validate(length(min = 1, max = 20))]
    kind: String,
    #[validate(length(min = 2, max = 120))]
    ref_no: String,
    #[validate(length(max = 120))]
    office: Option<String>,
    #[validate(length(max = 500))]
    note: Option<String>,
}
async fn bpn_add(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<BpnReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let id = s
        .stores
        .ops
        .bpn_add(
            &u.claims.org,
            b.kind.trim(),
            b.ref_no.trim(),
            b.office.as_deref().unwrap_or(""),
            b.note.as_deref().unwrap_or(""),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "property.bpn_added".into(),
            entity: "bpn".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}
#[derive(Debug, Deserialize)]
struct BpnQ {
    kind: Option<String>,
}
async fn bpn_list(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<BpnQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .bpn_list(&u.claims.org, q.kind.as_deref())
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "kind": r.kind, "ref_no": r.ref_no, "office": r.office, "status": r.status, "note": r.note}))
        .collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct BpnStatusReq {
    #[validate(length(min = 1, max = 40))]
    status: String,
}
async fn bpn_set_status(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(b): Json<BpnStatusReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores
        .ops
        .bpn_set_status(&u.claims.org, id, b.status.trim())
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": b.status.trim()})))
}
async fn bpn_delete(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Delete)?;
    s.stores
        .ops
        .bpn_delete(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"deleted": true})))
}
// ---------- end property slice: BPN coordination tracker ----------

// ---------- office slice: staff productivity + waiting analytics ----------
/// Staff productivity (F-D12-028): actions per actor from the audit trail
/// (last 500 entries). Honest activity counts, no fabricated scores.
async fn staff_productivity(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .audit_list(&u.claims.org, None, 500)
        .await
        .map_err(map_store)?;
    let mut by_actor: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut by_action: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for a in &v {
        *by_actor
            .entry(a.actor.clone().unwrap_or_else(|| "sistem".to_string()))
            .or_insert(0) += 1;
        *by_action.entry(a.action.clone()).or_insert(0) += 1;
    }
    let mut actors: Vec<Value> = by_actor
        .into_iter()
        .map(|(k, c)| json!({"actor": k, "actions": c}))
        .collect();
    actors.sort_by(|a, b| b["actions"].as_i64().cmp(&a["actions"].as_i64()));
    let mut actions: Vec<Value> = by_action
        .into_iter()
        .map(|(k, c)| json!({"action": k, "count": c}))
        .collect();
    actions.sort_by(|a, b| b["count"].as_i64().cmp(&a["count"].as_i64()));
    Ok(Json(
        json!({"sampled": v.len() as i64, "by_actor": actors, "by_action": actions}),
    ))
}
/// Client waiting-time analytics (F-D12-029): completed-visit dwell times
/// from visitor check-in/out stamps. `null` averages when no data — never
/// fabricated.
async fn waiting_analytics(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s
        .stores
        .ops
        .visitor_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    let mut minutes: Vec<i64> = Vec::new();
    let mut by_day: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for r in &v {
        if r.status != "out" {
            continue;
        }
        let (Ok(i), Ok(o)) = (
            chrono::NaiveDateTime::parse_from_str(&r.check_in, "%Y-%m-%d %H:%M"),
            chrono::NaiveDateTime::parse_from_str(&r.check_out, "%Y-%m-%d %H:%M"),
        ) else {
            continue;
        };
        let m = (o - i).num_minutes();
        if m < 0 {
            continue;
        }
        minutes.push(m);
        if r.check_in.len() >= 10 {
            *by_day.entry(r.check_in[..10].to_string()).or_insert(0) += 1;
        }
    }
    let avg = if minutes.is_empty() {
        None
    } else {
        Some((minutes.iter().sum::<i64>() as f64 / minutes.len() as f64 * 10.0).round() / 10.0)
    };
    let max = minutes.iter().copied().max();
    Ok(Json(
        json!({"completed_visits": minutes.len() as i64, "avg_minutes": avg, "max_minutes": max,
        "by_day": by_day.into_iter().map(|(k, c)| json!({"day": k, "count": c})).collect::<Vec<_>>()}),
    ))
}
// ---------- end office slice: staff productivity + waiting analytics ----------

// ---------- compliance slice: posture dashboard + exporter ----------
/// Compliance posture (F-D11-029): per-register totals + status breakdown.
/// Pure aggregation over live registers — no synthetic scores.
async fn compliance_posture(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    let mut out: Vec<Value> = Vec::new();
    let obl = ops
        .pdp_obligation_list(org, None)
        .await
        .map_err(map_store)?;
    out.push(posture_row(
        "obligations",
        obl.iter().map(|r| r.status.clone()),
    ));
    let con = ops.consent_list(org, None).await.map_err(map_store)?;
    out.push(posture_row(
        "consents",
        con.iter().map(|r| r.status.clone()),
    ));
    let ropa = ops.ropa_list(org).await.map_err(map_store)?;
    out.push(json!({"register": "ropa", "total": ropa.len(), "by_status": {}}));
    let dpia = ops.dpia_list(org).await.map_err(map_store)?;
    out.push(posture_row("dpia", dpia.iter().map(|r| r.status.clone())));
    let br = ops.breach_list(org).await.map_err(map_store)?;
    out.push(posture_row("breach", br.iter().map(|r| r.status.clone())));
    let ven = ops.vendor_list(org).await.map_err(map_store)?;
    out.push(posture_row(
        "vendors",
        ven.iter().map(|r| r.dpa_status.clone()),
    ));
    let inc = ops.incident_list(org).await.map_err(map_store)?;
    out.push(posture_row(
        "incidents",
        inc.iter().map(|r| r.status.clone()),
    ));
    let capa = ops.capa_list(org).await.map_err(map_store)?;
    out.push(posture_row("capa", capa.iter().map(|r| r.status.clone())));
    let rc = ops.regchange_list(org).await.map_err(map_store)?;
    out.push(posture_row(
        "regchanges",
        rc.iter().map(|r| r.status.clone()),
    ));
    let tr = ops.training_list(org).await.map_err(map_store)?;
    out.push(posture_row(
        "trainings",
        tr.iter().map(|r| r.status.clone()),
    ));
    let rk = ops.risk_list(org).await.map_err(map_store)?;
    out.push(posture_row("risks", rk.iter().map(|r| r.status.clone())));
    let ds = ops.dsar_list(org).await.map_err(map_store)?;
    out.push(posture_row("dsar", ds.iter().map(|r| r.status.clone())));
    let et = ops.ethics_list(org).await.map_err(map_store)?;
    out.push(posture_row("ethics", et.iter().map(|r| r.status.clone())));
    let gi = ops.gift_list(org).await.map_err(map_store)?;
    out.push(posture_row("gifts", gi.iter().map(|r| r.status.clone())));
    let af = ops.afinding_list(org).await.map_err(map_store)?;
    out.push(posture_row("findings", af.iter().map(|r| r.status.clone())));
    let wb = ops.whistle_list(org).await.map_err(map_store)?;
    out.push(posture_row("whistle", wb.iter().map(|r| r.status.clone())));
    let wv = ops.waiver_list(org).await.map_err(map_store)?;
    out.push(posture_row("waivers", wv.iter().map(|r| r.status.clone())));
    let pol = ops.policy_list(org).await.map_err(map_store)?;
    out.push(posture_row(
        "policies",
        pol.iter().map(|r| r.status.clone()),
    ));
    let ctl = ops.control_list(org).await.map_err(map_store)?;
    out.push(posture_row(
        "controls",
        ctl.iter().map(|r| {
            if r.active {
                "active".to_string()
            } else {
                "inactive".to_string()
            }
        }),
    ));
    Ok(Json(json!(out)))
}
fn posture_row(register: &str, statuses: impl Iterator<Item = String>) -> Value {
    let mut by: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut total = 0i64;
    for st in statuses {
        *by.entry(st).or_insert(0) += 1;
        total += 1;
    }
    json!({"register": register, "total": total, "by_status": by})
}
/// Compliance report exporter (F-D11-040): one CSV across all registers.
async fn compliance_csv_export(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    let mut csv = String::from("register,id,title,status,extra\n");
    for r in ops
        .pdp_obligation_list(org, None)
        .await
        .map_err(map_store)?
    {
        csv.push_str(&format!(
            "obligations,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.owner)
        ));
    }
    for r in ops.consent_list(org, None).await.map_err(map_store)? {
        csv.push_str(&format!(
            "consents,{},{},{},{}\n",
            r.id,
            csv_field(&r.subject),
            csv_field(&r.status),
            csv_field(&r.purpose)
        ));
    }
    for r in ops.ropa_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "ropa,{},{},,{}\n",
            r.id,
            csv_field(&r.activity),
            csv_field(&r.legal_basis)
        ));
    }
    for r in ops.dpia_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "dpia,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.risk_level)
        ));
    }
    for r in ops.breach_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "breach,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.discovered_at)
        ));
    }
    for r in ops.vendor_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "vendors,{},{},{},{}\n",
            r.id,
            csv_field(&r.name),
            csv_field(&r.dpa_status),
            csv_field(&r.service)
        ));
    }
    for r in ops.incident_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "incidents,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.severity)
        ));
    }
    for r in ops.capa_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "capa,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.action)
        ));
    }
    for r in ops.regchange_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "regchanges,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.source)
        ));
    }
    for r in ops.training_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "trainings,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.assignee)
        ));
    }
    for r in ops.risk_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "risks,{},{},{},score={}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            r.likelihood * r.impact
        ));
    }
    for r in ops.dsar_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "dsar,{},{},{},{}\n",
            r.id,
            csv_field(&r.subject),
            csv_field(&r.status),
            csv_field(&r.kind)
        ));
    }
    for r in ops.ethics_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "ethics,{},{},{},{}\n",
            r.id,
            csv_field(&r.declarant),
            csv_field(&r.status),
            csv_field(&r.year)
        ));
    }
    for r in ops.gift_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "gifts,{},{},{},{}\n",
            r.id,
            csv_field(&r.description),
            csv_field(&r.status),
            csv_field(&r.direction)
        ));
    }
    for r in ops.afinding_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "findings,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.severity)
        ));
    }
    for r in ops.whistle_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "whistle,{},{},{},{}\n",
            r.id,
            csv_field(&r.summary),
            csv_field(&r.status),
            csv_field(&r.channel)
        ));
    }
    for r in ops.waiver_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "waivers,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.expires_at)
        ));
    }
    for r in ops.policy_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "policies,{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.status),
            csv_field(&r.version)
        ));
    }
    for r in ops.control_list(org).await.map_err(map_store)? {
        csv.push_str(&format!(
            "controls,{},{},{},{}\n",
            r.id,
            csv_field(&r.code),
            if r.active { "active" } else { "inactive" },
            csv_field(&r.frequency)
        ));
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
/// Compliance alert feed (F-D11-034): attention items across registers —
/// overdue DSAR/trainings, unreviewed whistleblowing, high open risks,
/// open critical/high findings, expired approved waivers. Pure derivation,
/// no synthetic scores.
async fn compliance_alerts(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    let today = chrono::Utc::now()
        .naive_utc()
        .date()
        .format("%Y-%m-%d")
        .to_string();
    let mut out: Vec<Value> = Vec::new();
    for r in ops.dsar_list(org).await.map_err(map_store)? {
        if !r.due_date.trim().is_empty()
            && r.due_date.as_str() < today.as_str()
            && (r.status == "open" || r.status == "in_review")
        {
            out.push(json!({"kind": "dsar_overdue", "id": r.id,
                "title": format!("DSAR {} lewat tenggat {}", r.subject, r.due_date)}));
        }
    }
    for r in ops.training_list(org).await.map_err(map_store)? {
        if !r.due_date.trim().is_empty()
            && r.due_date.as_str() < today.as_str()
            && r.status != "completed"
        {
            out.push(json!({"kind": "training_overdue", "id": r.id,
                "title": format!("Pelatihan {} lewat tenggat {}", r.title, r.due_date)}));
        }
    }
    for r in ops.risk_list(org).await.map_err(map_store)? {
        if r.status != "closed" && r.likelihood * r.impact >= 12 {
            out.push(json!({"kind": "risk_high", "id": r.id,
                "title": format!("Risiko tinggi {} (skor {})", r.title, r.likelihood * r.impact)}));
        }
    }
    for r in ops.whistle_list(org).await.map_err(map_store)? {
        if r.status == "received" {
            out.push(json!({"kind": "whistle_unreviewed", "id": r.id,
                "title": format!("Laporan whistleblowing #{} belum ditinjau", r.id)}));
        }
    }
    for r in ops.afinding_list(org).await.map_err(map_store)? {
        if r.status == "open" && (r.severity == "critical" || r.severity == "high") {
            out.push(json!({"kind": "finding_open", "id": r.id,
                "title": format!("Temuan {} ({}) terbuka", r.title, r.severity)}));
        }
    }
    for r in ops.waiver_list(org).await.map_err(map_store)? {
        if r.status == "approved"
            && !r.expires_at.trim().is_empty()
            && r.expires_at.as_str() < today.as_str()
        {
            out.push(json!({"kind": "waiver_expired", "id": r.id,
                "title": format!("Waiver {} kedaluwarsa {}", r.title, r.expires_at)}));
        }
    }
    Ok(Json(json!(out)))
}
// ---------- end compliance slice: posture dashboard + exporter ----------

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
/// Deed-kind readiness (F-D11-006/007 hibah & tukar-menukar support):
/// kind-aware verification rules over register fields + KYC bundle.
/// Pure derivation — the register stays the source of truth.
async fn ppat_readiness(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let ops = &s.stores.ops;
    let org = &u.claims.org;
    let r = ops
        .ppat_list(org)
        .await
        .map_err(map_store)?
        .into_iter()
        .find(|x| x.id == id)
        .ok_or(AppError::NotFound("ppat".into()))?;
    let parties = ops.pparty_list(org, id).await.map_err(map_store)?;
    let verified = parties.iter().filter(|p| p.verified).count();
    let has_role_verified = |role: &str| parties.iter().any(|p| p.role == role && p.verified);
    let mut checks: Vec<Value> = Vec::new();
    let mut push = |key: &str, label: &str, pass: bool, detail: String| {
        checks.push(json!({"key": key, "label": label, "pass": pass, "detail": detail}));
    };
    push(
        "fields",
        "Register lengkap (nomor, penjual, pembeli, objek)",
        !r.number.trim().is_empty()
            && !r.party_seller.trim().is_empty()
            && !r.party_buyer.trim().is_empty()
            && !r.object_desc.trim().is_empty(),
        format!("akta {}", r.number),
    );
    match r.deed_kind.as_str() {
        "hibah" => {
            push(
                "kyc_giver",
                "Pemberi hibah terverifikasi (KYC)",
                has_role_verified("seller"),
                "peran seller".into(),
            );
            push(
                "object",
                "Objek hibah tercatat",
                !r.object_desc.trim().is_empty(),
                r.object_desc.clone(),
            );
        }
        "tukar_menukar" => {
            push(
                "kyc_both",
                "Kedua belah pihak terverifikasi (timbal-balik)",
                has_role_verified("seller") && has_role_verified("buyer"),
                format!("{verified}/{} pihak terverifikasi", parties.len()),
            );
        }
        _ => {
            push(
                "kyc_buyer",
                "Pembeli terverifikasi (KYC)",
                has_role_verified("buyer"),
                "peran buyer".into(),
            );
        }
    }
    let ready = checks.iter().all(|c| c["pass"].as_bool().unwrap_or(false));
    Ok(Json(
        json!({"id": r.id, "deed_kind": r.deed_kind, "status": r.status,
        "verified_parties": verified, "total_parties": parties.len(),
        "ready": ready, "checks": checks}),
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
/// Executive summary pack (F-D20-039): one JSON snapshot across modules.
async fn executive_summary(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let org = &u.claims.org;
    let clients = s
        .stores
        .core
        .list_clients(org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let matters = s
        .stores
        .core
        .list_matters(org)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let deeds = s.stores.ops.deed_list(org).await.map_err(map_store)?;
    let open_inv = s.stores.ops.inv_list_open(org).await.map_err(map_store)?;
    let open_bal: i64 = open_inv.iter().map(|r| r.balance).sum();
    let pending_appr = s
        .stores
        .ops
        .appr_list(org, Some("pending"))
        .await
        .map_err(map_store)?;
    let due = s
        .stores
        .core
        .task_due_list(org, 1000)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    let now = chrono::Utc::now()
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let overdue = due
        .iter()
        .filter(|t| t.status != "done" && t.due.as_deref().is_some_and(|d| d < now.as_str()))
        .count();
    let obligations = s
        .stores
        .ops
        .pdp_obligation_list(org, Some("open"))
        .await
        .map_err(map_store)?;
    Ok(Json(json!({
        "clients": clients.len(), "matters": matters.len(), "deeds": deeds.len(),
        "open_invoices": open_inv.len(), "open_balance": open_bal,
        "pending_approvals": pending_appr.len(), "open_obligations": obligations.len(),
        "tasks_due_tracked": due.len(), "tasks_overdue": overdue,
    })))
}
/// Segregation-of-duties conflict check (F-D14-038): report risky permission
/// pairs held by one user. Advisory (warn, never auto-revoke).
#[derive(Debug, Deserialize)]
struct SodQ {
    user: Option<String>,
}
async fn sod_check(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<SodQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Assign)?;
    let uid = q.user.as_deref().unwrap_or(&u.claims.sub);
    let target = s
        .stores
        .users
        .get_user(uid)
        .await
        .map_err(map_store)?
        .ok_or_else(|| AppError::NotFound("user".into()))?;
    if target.org_id != u.claims.org {
        return Err(AppError::NotFound("user".into()));
    }
    let roles = s
        .stores
        .users
        .user_roles(&target.id)
        .await
        .map_err(map_store)?;
    let mut perms: Vec<String> = Vec::new();
    for r in &roles {
        if let Some(role) = legalrust_domain::Role::parse(r) {
            for p in legalrust_authorization::permissions(role) {
                let name = format!("{p:?}");
                if !perms.contains(&name) {
                    perms.push(name);
                }
            }
        }
    }
    const CONFLICTS: &[(&str, &str, &str)] = &[
        (
            "Assign",
            "Download",
            "dapat memberi hak lalu mengunduh dokumen",
        ),
        (
            "Sign",
            "Verify",
            "menandatangani lalu memverifikasi sendiri",
        ),
        ("Create", "Approve", "membuat lalu menyetujui sendiri"),
    ];
    let has = |p: &str| perms.iter().any(|x| x == p);
    let conflicts: Vec<Value> = CONFLICTS
        .iter()
        .filter(|(a, b, _)| has(a) && has(b))
        .map(|(a, b, why)| json!({"pair": [a, b], "reason": why}))
        .collect();
    Ok(Json(
        json!({"user": target.id, "permissions": perms, "conflicts": conflicts}),
    ))
}
#[derive(Debug, Deserialize, Validate)]
struct InviteReq {
    #[validate(email)]
    email: String,
    #[validate(length(min = 1, max = 60))]
    role: String,
}
async fn invite_create(
    State(s): State<FullState>,
    headers: HeaderMap,
    Json(b): Json<InviteReq>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Assign)?;
    b.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let role = legalrust_domain::Role::parse(b.role.trim())
        .ok_or_else(|| AppError::Validation("role".into()))?;
    let raw = format!("inv_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let hash = legalrust_document::sha256_hex(raw.as_bytes());
    let exp = chrono::Utc::now().naive_utc() + chrono::Duration::hours(72);
    let id = s
        .stores
        .ops
        .invite_create(
            &u.claims.org,
            &b.email,
            role.as_str(),
            &hash,
            &exp.format("%Y-%m-%d %H:%M:%S").to_string(),
        )
        .await
        .map_err(map_store)?;
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "org.invite_created".into(),
            entity: "invite".into(),
            entity_id: id,
            ip: None,
        },
    )
    .await;
    Ok(Json(json!({"id": id, "token": raw})))
}
async fn invite_list(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Assign)?;
    let v = s
        .stores
        .ops
        .invite_list(&u.claims.org)
        .await
        .map_err(map_store)?;
    Ok(Json(json!(v
        .iter()
        .map(|r| json!({"id": r.id, "email": r.email, "role": r.role, "accepted": r.accepted}))
        .collect::<Vec<_>>())))
}
async fn invite_revoke(
    State(s): State<FullState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Assign)?;
    s.stores
        .ops
        .invite_revoke(&u.claims.org, id)
        .await
        .map_err(map_store)?;
    Ok(Json(json!({"revoked": true})))
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
/// Integration health for the admin console (F-D21-031): per-provider
/// readiness WITHOUT exposing secrets (booleans + env var names only).
async fn integration_health(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let t = &s.stores.tripay;
    let tripay_ok = [&t.api_key_env, &t.private_key_env, &t.merchant_code_env]
        .iter()
        .all(|k| !std::env::var(k).unwrap_or_default().trim().is_empty());
    let ai_base = std::env::var("AI_BASE_URL").unwrap_or_default();
    let ai_model = std::env::var("AI_MODEL").unwrap_or_default();
    let tessdata = std::env::var("TESSDATA_PREFIX").unwrap_or_default();
    // Local engine probe with a hard timeout: version output means the
    // binary actually runs here (not just env presence).
    let tesseract_ok = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::process::Command::new("tesseract")
            .arg("--version")
            .output(),
    )
    .await
    .map(|r| r.map(|o| o.status.success()).unwrap_or(false))
    .unwrap_or(false);
    Ok(Json(json!([
        {"name": "tripay", "kind": "payment_gateway", "configured": tripay_ok,
         "detail": "TRIPAY_API_KEY/TRIPAY_PRIVATE_KEY/TRIPAY_MERCHANT_CODE"},
        {"name": "ai", "kind": "llm", "configured": !ai_base.trim().is_empty() && !ai_model.trim().is_empty(),
         "detail": format!("AI_BASE_URL={} AI_MODEL={}", if ai_base.trim().is_empty() { "unset" } else { "set" }, if ai_model.trim().is_empty() { "unset" } else { "set" })},
        {"name": "ocr", "kind": "ocr_engine", "configured": tesseract_ok,
         "detail": format!("tesseract --version {}", if tessdata.trim().is_empty() { "(TESSDATA_PREFIX unset)" } else { "(TESSDATA_PREFIX set)" })},
        {"name": "database", "kind": "mysql", "configured": s.pool.is_some(),
         "detail": if s.pool.is_some() { "MySQL pool connected" } else { "ephemeral dev store (non-production)" }},
    ])))
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
        .map(|r| json!({"id": r.id, "entity": r.entity, "entity_id": r.entity_id, "title": r.title, "status": r.status, "created": r.created}))
        .collect::<Vec<_>>())))
}
/// Overdue approval alerts (F-D09-018): pending requests older than `days`
/// (default 3), oldest first, with the blocking step role (`waiting_on`).
/// Age derives from the request `created` timestamp — no schema change.
#[derive(Debug, Deserialize)]
struct ApprOverdueQ {
    days: Option<i64>,
}
async fn appr_overdue(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<ApprOverdueQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let days = q.days.unwrap_or(3).clamp(0, 90);
    let pending = s
        .stores
        .ops
        .appr_list(&u.claims.org, Some("pending"))
        .await
        .map_err(map_store)?;
    let now = chrono::Utc::now().naive_utc();
    let mut out: Vec<Value> = Vec::new();
    for r in &pending {
        let age: i64 = chrono::NaiveDateTime::parse_from_str(&r.created, "%Y-%m-%d %H:%M:%S")
            .map(|dt| (now - dt).num_days().max(0))
            .unwrap_or(0);
        if age < days {
            continue;
        }
        let waiting_on = s
            .stores
            .ops
            .appr_get(&u.claims.org, r.id)
            .await
            .map(|(_, steps)| {
                steps
                    .iter()
                    .find(|x| x.status == "pending")
                    .map(|x| x.role.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        out.push(
            json!({"id": r.id, "entity": r.entity, "entity_id": r.entity_id,
            "title": r.title, "created": r.created, "age_days": age, "waiting_on": waiting_on}),
        );
    }
    out.sort_by(|a, b| b["age_days"].as_i64().cmp(&a["age_days"].as_i64()));
    Ok(Json(json!(out)))
}
/// Approval calendar view (F-D09-019): pending requests grouped by creation
/// day for a `YYYY-MM` month (default: current month). Powers the Persetujuan
/// calendar card; `waiting_on` shows the blocking step role per request.
#[derive(Debug, Deserialize)]
struct ApprCalQ {
    month: Option<String>,
}
async fn appr_calendar(
    State(s): State<FullState>,
    headers: HeaderMap,
    Query(q): Query<ApprCalQ>,
) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let now = chrono::Utc::now().naive_utc();
    let month: String = match q.month {
        Some(m) => {
            chrono::NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d")
                .map_err(|_| AppError::Validation("month must be YYYY-MM".into()))?;
            m
        }
        None => now.format("%Y-%m").to_string(),
    };
    let pending = s
        .stores
        .ops
        .appr_list(&u.claims.org, Some("pending"))
        .await
        .map_err(map_store)?;
    let mut days: std::collections::BTreeMap<String, Vec<Value>> =
        std::collections::BTreeMap::new();
    for r in &pending {
        if r.created.len() < 10 || !r.created.starts_with(month.as_str()) {
            continue;
        }
        let day = r.created[..10].to_string();
        let waiting_on = s
            .stores
            .ops
            .appr_get(&u.claims.org, r.id)
            .await
            .map(|(_, steps)| {
                steps
                    .iter()
                    .find(|x| x.status == "pending")
                    .map(|x| x.role.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let age: i64 = chrono::NaiveDateTime::parse_from_str(&r.created, "%Y-%m-%d %H:%M:%S")
            .map(|dt| (now - dt).num_days().max(0))
            .unwrap_or(0);
        days.entry(day).or_default().push(
            json!({"id": r.id, "title": r.title, "entity": r.entity, "waiting_on": waiting_on, "age_days": age}),
        );
    }
    Ok(Json(json!({"month": month, "days": days})))
}
/// Auditor approval export (F-D09-035): full request list with step state.
async fn approvals_csv_export(
    State(s): State<FullState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let v = s
        .stores
        .ops
        .appr_list(&u.claims.org, None)
        .await
        .map_err(map_store)?;
    let mut csv = String::from("id,title,entity,status,created,pending_steps\n");
    for r in &v {
        let (_, steps) = s
            .stores
            .ops
            .appr_get(&u.claims.org, r.id)
            .await
            .map_err(map_store)?;
        let open = steps.iter().filter(|x| x.status == "pending").count();
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            r.id,
            csv_field(&r.title),
            csv_field(&r.entity),
            csv_field(&r.status),
            csv_field(&r.created),
            open
        ));
    }
    audit(
        &s.pool,
        &s.stores,
        AuditEvent {
            tenant_id: u.claims.org.clone(),
            actor_id: Some(u.claims.sub.clone()),
            action: "report.approvals_csv_exported".into(),
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
    let body = r#"<div class="dash"><aside class="navbar-vertical dash-side" id="sidebar"><div class="side-brand"><span class="side-logo">LegalRust</span><small>Smart Legal Solution</small><button class="btn btn-sm side-close" id="sbToggle2" type="button">Tutup</button></div><div class="side-org"><label class="side-title" for="orgsel">Organisasi</label><select class="form-select form-select-sm" id="orgsel"><option value="">Memuat…</option></select><input class="form-control form-control-sm mt-2" id="menu-q" placeholder="Cari menu…" autocomplete="off"></div><div class="side-group"><div class="side-title">Utama</div><a class="snav" href="/app" data-tab="ringkas">Dashboard</a></div><div class="side-group"><div class="side-title">Identitas &amp; KYC</div><a class="snav" href="/app" data-tab="ocr">Verifikasi Identitas</a></div><div class="side-group"><div class="side-title">Klien &amp; Perkara</div><a class="snav" href="/app" data-tab="klien">Klien &amp; Perkara</a></div><div class="side-group"><div class="side-title">Keuangan</div><a class="snav" href="/app" data-tab="keu">Keuangan</a></div><div class="side-group"><div class="side-title">Dokumen</div><a class="snav" href="/app" data-tab="dok">Dokumen</a></div><div class="side-group"><div class="side-title">Akta</div><a class="snav" href="/app" data-tab="akta">Akta</a></div><div class="side-group"><div class="side-title">Persetujuan</div><a class="snav" href="/app" data-tab="appr">Persetujuan</a></div><div class="side-group"><div class="side-title">Marketing</div><a class="snav" href="/app" data-tab="mkt">Marketing</a></div><div class="side-group"><div class="side-title">Pengaturan</div><a class="snav" href="/app" data-tab="pengaturan">Pengaturan</a></div><div class="side-group"><div class="side-title">Operasional</div><a class="snav" href="/app" data-tab="audit">Audit</a><a class="snav" href="/app" data-tab="jobs">Jobs</a></div><div class="side-group"><div class="side-title">Kepatuhan</div><a class="snav" href="/app" data-tab="kepatuhan">Kepatuhan</a></div><div class="side-user"><span class="avatar" id="avatar2">?</span><span class="side-who"><strong id="uname">Memuat…</strong><small id="urole">…</small></span><button class="btn btn-sm btn-outline-danger logout" type="button">Keluar</button></div></aside><div class="dash-main" id="main" role="main"><header class="dash-top"><button class="btn btn-sm" id="sbToggle" type="button">Menu</button><a href="/" class="navbar-brand">LegalRust</a><ul class="nav mx-auto" id="tabs"><li class="nav-item"><a class="nav-link active" href="/app">Ringkasan</a></li><li class="nav-item"><a class="nav-link" href="/app">Klien</a></li><li class="nav-item"><a class="nav-link" href="/app">Dokumen</a></li><li class="nav-item"><a class="nav-link" href="/app">OCR</a></li><li class="nav-item"><a class="nav-link" href="/app">Akta</a></li><li class="nav-item"><a class="nav-link" href="/app">Keuangan</a></li><li class="nav-item"><a class="nav-link" href="/app">Persetujuan</a></li><li class="nav-item"><a class="nav-link" href="/app">Marketing</a></li><li class="nav-item"><a class="nav-link" href="/app">Pengaturan</a></li><li class="nav-item"><a class="nav-link" href="/app">Audit</a></li><li class="nav-item"><a class="nav-link" href="/app">Jobs</a></li><li class="nav-item"><a class="nav-link" href="/app">Kepatuhan</a></li></ul><div class="ms-2 position-relative"><input class="form-control form-control-sm" id="q" placeholder="Cari klien / perkara / dokumen" autocomplete="off"><div id="qres"></div></div><div class="position-relative"><button class="btn btn-sm" id="bell" type="button">Notifikasi<span class="bell-n" id="bell-n"></span></button><div id="bell-list"></div></div><span class="avatar" id="avatar">?</span><span class="text-muted small" id="orgname">…</span><button class="btn btn-outline-danger btn-sm logout" type="button">Keluar</button></header><div class="container-fluid py-3"><h1 class="page-title" id="hi">Dasbor</h1><div class="text-danger" id="err"></div><div id="p-ringkas"><div class="row g-2" id="g"><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Klien Aktif <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k1">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Perkara Berjalan <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k2">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Tugas Terbuka <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k3">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Dokumen <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k4">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">OCR Menunggu Review <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k5">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Akta Dalam Proses <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k6">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Invoice Outstanding <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k7">…</div></div></div></div><div class="col-6 col-md-3"><div class="card kpi"><div class="card-body"><div class="text-muted">Saldo Trust <span class="badge bg-green-lt">live</span></div><div class="h1 m-0" id="k8">…</div></div></div></div></div><div class="row g-2 mt-1"><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Pipeline Akta</h2></div><div class="card-body" id="funnel"><span class="text-muted">Memuat…</span></div></div></div><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Aktivitas</h2></div><div class="card-body" id="chart"><span class="text-muted">Memuat…</span></div></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Antrean Persetujuan</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="al-q"></tbody></table></div></div><div class="text-muted mt-2" id="ae-q"></div><div class="row g-2 mt-1"><div class="col-md-4"><div class="card"><div class="card-header"><h2 class="card-title">Kalender</h2></div><div class="card-body" id="cal"><span class="text-muted">Memuat…</span></div></div></div><div class="col-md-4"><div class="card"><div class="card-header"><h2 class="card-title">Dokumen Terbaru</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th></tr></thead><tbody id="dl5"></tbody></table></div></div></div><div class="col-md-4"><div class="card"><div class="card-header"><h2 class="card-title">Review OCR</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Tipe</th><th>Status</th></tr></thead><tbody id="ocr-recent"></tbody></table></div></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Perkara Aktif</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody id="m"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Ringkasan Keuangan</h2></div><div class="card-body" id="fin-sum"><span class="text-muted">Memuat…</span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Ekspor Laporan</h2><button class="btn btn-outline-secondary btn-sm" id="rep-matters" type="button">Perkara CSV</button> <button class="btn btn-outline-secondary btn-sm" id="rep-deadlines" type="button">Tenggat CSV</button> <button class="btn btn-outline-secondary btn-sm" id="rep-sla" type="button">Pelanggaran SLA</button> <button class="btn btn-outline-secondary btn-sm" id="rep-posture" type="button">Postur Kepatuhan</button> <span class="text-danger" id="rep-msg"></span><div class="mkt-row mt-2"><strong>Ringkasan eksekutif</strong><button class="btn btn-outline-secondary btn-sm" id="rep-exec" type="button">Muat ringkasan</button></div><div id="exec-sum"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Aksi Cepat</h2></div><div class="card-body qa-grid"><button class="btn btn-outline-primary" id="qa-client" type="button">Tambah Klien</button><button class="btn btn-outline-primary" id="qa-matter" type="button">Buat Perkara</button><button class="btn btn-outline-primary" id="qa-upload" type="button">Upload Dokumen</button><button class="btn btn-outline-primary" id="qa-ocr" type="button">Proses OCR</button><button class="btn btn-outline-primary" id="qa-deed" type="button">Buat Akta</button><button class="btn btn-outline-primary" id="qa-invoice" type="button">Buat Invoice</button></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pengumuman kantor</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ann-title" placeholder="judul pengumuman"><input class="form-control" id="ann-body" placeholder="isi pengumuman"><button class="btn btn-primary btn-sm" id="ann-add" type="button">Terbitkan</button><button class="btn btn-secondary btn-sm" id="ann-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ann-msg"></div><div id="ann-list"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Insiden kantor</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="oin-title" placeholder="judul insiden"><select class="form-control mkt-filter" id="oin-sev"><option value="low">low</option><option value="medium" selected>medium</option><option value="high">high</option><option value="critical">critical</option></select><button class="btn btn-primary btn-sm" id="oin-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="oin-reload" type="button">Muat ulang</button></div><div class="text-danger" id="oin-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="oin-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Produktivitas staf</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="staff-reload" type="button">Muat ulang</button><span class="text-muted" id="staff-sum"></span></div><div class="text-danger" id="staff-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Aktor</th><th>Aksi</th></tr></thead><tbody id="staff-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Analitik waktu tunggu</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="wait-reload" type="button">Muat ulang</button><span class="text-muted" id="wait-sum"></span></div><div class="text-danger" id="wait-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Hari</th><th>Kunjungan</th></tr></thead><tbody id="wait-list"></tbody></table></div></div></div><section class="cta mt-3"><div><h2>Kelola kantor Anda dalam satu dasbor</h2><p>Perkara, dokumen, akta, dan keuangan — semuanya live dari API LegalRust.</p><a class="go" href="/login">Mulai Gratis</a></div><ul><li>Data live, bukan mockup</li><li>Jejak audit penuh</li><li class="dim">Eksternal bertahap, ditandai 501</li></ul></section></div><div id="p-klien" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Klien</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Jenis</th></tr></thead><tbody id="cl"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Tambah Klien</h2><input class="form-control mb-2" id="cli-name" placeholder="nama, mis. PT Maju"><select class="form-control mb-2" id="cli-kind"><option value="individual">individual</option><option value="corporate">corporate</option><option value="government">government</option></select><button class="btn btn-primary" id="cli-btn" type="button">Simpan</button> <button class="btn btn-outline-secondary" id="cli-csv" type="button">Ekspor CSV</button> <span class="text-danger" id="cli-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Daftar Perkara</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Judul</th><th>Status</th></tr></thead><tbody id="m-all"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Buat Perkara</h2><input class="form-control mb-2" id="mat-title" placeholder="judul perkara"><select class="form-control mb-2" id="mat-client"><option value="">Tanpa klien</option></select><button class="btn btn-primary" id="mat-btn" type="button">Buat</button> <span class="text-danger" id="mat-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Tugas</h2></div><div class="card-body"><select class="form-control mb-2" id="task-matter"><option value="">Pilih perkara…</option></select><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Status</th><th>Tenggat</th><th>Aksi</th></tr></thead><tbody id="task-list"></tbody></table></div><input class="form-control mb-2" id="task-title" placeholder="judul tugas baru"><div class="mkt-row"><input class="form-control mkt-filter" id="task-due" type="datetime-local"><button class="btn btn-primary" id="task-btn" type="button">Tambah tugas</button></div> <span class="text-danger" id="task-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Dispatch multi-desk</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="dsp-task" placeholder="ID tugas" inputmode="numeric"><input class="form-control" id="dsp-who" placeholder="petugas"><input class="form-control mkt-filter" id="dsp-desk" placeholder="meja, mis. meja-1"><button class="btn btn-primary btn-sm" id="dsp-add" type="button">Tugaskan</button><button class="btn btn-secondary btn-sm" id="dsp-reload" type="button">Muat papan</button></div><div class="text-danger" id="dsp-msg"></div><div id="dsp-board"></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Pertemuan klien</h2><input class="form-control mb-2" id="crm-meet-client" placeholder="ID klien" inputmode="numeric"><div class="table-responsive mb-2"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Waktu</th><th>Aksi</th></tr></thead><tbody id="crm-meet-list"><tr><td colspan="4" class="text-muted">Isi ID klien lalu Muat.</td></tr></tbody></table></div><input class="form-control mb-2" id="crm-meet-title" placeholder="judul pertemuan (min 3 huruf)"><input class="form-control mb-2" id="crm-meet-at" type="datetime-local"><textarea class="form-control mb-2" id="crm-meet-notes" rows="2" placeholder="catatan (opsional)"></textarea><button class="btn btn-primary" id="crm-meet-btn" type="button">Simpan pertemuan</button> <button class="btn btn-outline-secondary" id="crm-meet-load" type="button">Muat</button> <span class="text-danger" id="crm-meet-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Tag klien</h2><div class="table-responsive mb-2"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Aksi</th></tr></thead><tbody id="crm-tag-list"></tbody></table></div><input class="form-control mb-2" id="crm-tag-name" placeholder="nama tag baru"><button class="btn btn-primary" id="crm-tag-btn" type="button">Buat tag</button> <span class="text-danger" id="crm-tag-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Tag per klien</h2><input class="form-control mb-2" id="crm-ct-client" placeholder="ID klien" inputmode="numeric"><input class="form-control mb-2" id="crm-ct-tag" placeholder="ID tag" inputmode="numeric"><button class="btn btn-primary" id="crm-ct-attach" type="button">Pasang tag</button> <button class="btn btn-outline-secondary" id="crm-ct-load" type="button">Muat tag</button> <span class="text-danger" id="crm-ct-msg"></span><div class="table-responsive mt-2"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Aksi</th></tr></thead><tbody id="crm-ct-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Pertemuan terbaru</h2><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Klien</th><th>Judul</th><th>Waktu</th></tr></thead><tbody id="crm-recent"></tbody></table></div><button class="btn btn-outline-secondary mt-2" id="crm-recent-btn" type="button">Muat ulang</button> <span class="text-danger" id="crm-recent-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Janji temu penghadap</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="appt-name" placeholder="nama penghadap"><input class="form-control" id="appt-purpose" placeholder="keperluan"><input class="form-control mkt-filter" id="appt-at" placeholder="YYYY-MM-DD HH:MM"><button class="btn btn-primary btn-sm" id="appt-add" type="button">Jadwalkan</button><button class="btn btn-secondary btn-sm" id="appt-reload" type="button">Muat ulang</button></div><div class="text-danger" id="appt-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Penghadap</th><th>Jadwal</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="appt-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Papan antrean</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="queue-date" placeholder="YYYY-MM-DD (kosong = hari ini)"><button class="btn btn-secondary btn-sm" id="queue-reload" type="button">Muat ulang</button></div><div class="text-danger" id="queue-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Jam</th><th>Penghadap</th><th>Keperluan</th><th>Status</th></tr></thead><tbody id="queue-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Buku tamu</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="vis-name" placeholder="nama tamu"><input class="form-control" id="vis-need" placeholder="keperluan"><input class="form-control mkt-filter" id="vis-in" placeholder="masuk YYYY-MM-DD HH:MM"><button class="btn btn-primary btn-sm" id="vis-add" type="button">Check-in</button><button class="btn btn-secondary btn-sm" id="vis-reload" type="button">Muat ulang</button></div><div class="text-danger" id="vis-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Masuk</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="vis-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pool saksi</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="wit-name" placeholder="nama saksi"><input class="form-control mkt-filter" id="wit-id" placeholder="NIK"><input class="form-control mkt-filter" id="wit-phone" placeholder="telepon"><button class="btn btn-primary btn-sm" id="wit-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="wit-reload" type="button">Muat ulang</button></div><div class="text-danger" id="wit-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>NIK</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="wit-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Verifikasi ulang identitas</h2></div><div class="card-body"><p class="text-muted">Log manual — pengecekan Dukcapil/BPN eksternal belum tersedia (BLOCKED).</p><div class="mkt-row"><input class="form-control" id="rch-subject" placeholder="subjek, mis. NIK/nama"><select class="form-control mkt-filter" id="rch-method"><option value="document">document</option><option value="physical">physical</option><option value="video">video</option></select><select class="form-control mkt-filter" id="rch-result"><option value="pass">pass</option><option value="fail">fail</option></select><input class="form-control mkt-filter" id="rch-on" placeholder="tanggal YYYY-MM-DD"><input class="form-control" id="rch-note" placeholder="catatan"><button class="btn btn-primary btn-sm" id="rch-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="rch-reload" type="button">Muat ulang</button></div><div class="text-danger" id="rch-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Subjek</th><th>Metode</th><th>Hasil</th><th>Aksi</th></tr></thead><tbody id="rch-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Jam operasional</h2></div><div class="card-body"><div class="mkt-row"><select class="form-control mkt-filter" id="hrs-day"><option value="0">Senin</option><option value="1">Selasa</option><option value="2">Rabu</option><option value="3">Kamis</option><option value="4">Jumat</option><option value="5">Sabtu</option><option value="6">Ahad</option></select><input class="form-control mkt-filter" id="hrs-open" placeholder="buka HH:MM"><input class="form-control mkt-filter" id="hrs-close" placeholder="tutup HH:MM"><button class="btn btn-primary btn-sm" id="hrs-set" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="hrs-reload" type="button">Muat ulang</button></div><div class="text-danger" id="hrs-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Hari</th><th>Jam</th><th>Aktif</th></tr></thead><tbody id="hrs-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Roster staf</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ros-name" placeholder="nama staf"><input class="form-control mkt-filter" id="ros-role" placeholder="peran"><select class="form-control mkt-filter" id="ros-shift"><option value="pagi">pagi</option><option value="siang">siang</option><option value="penuh">penuh</option></select><button class="btn btn-primary btn-sm" id="ros-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="ros-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ros-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Shift</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="ros-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Cuti &amp; pengganti</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="lev-who" placeholder="pemegang cuti"><select class="form-control mkt-filter" id="lev-kind"><option value="leave">leave</option><option value="sick">sick</option><option value="duty">duty</option></select><input class="form-control mkt-filter" id="lev-from" placeholder="dari YYYY-MM-DD"><input class="form-control mkt-filter" id="lev-to" placeholder="sampai YYYY-MM-DD"><input class="form-control" id="lev-sub" placeholder="pengganti"><button class="btn btn-primary btn-sm" id="lev-add" type="button">Ajukan</button><button class="btn btn-secondary btn-sm" id="lev-reload" type="button">Muat ulang</button></div><div class="text-danger" id="lev-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pemegang</th><th>Periode</th><th>Pengganti</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="lev-list"></tbody></table></div></div></div></div><div id="p-dok" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Dokumen</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th></tr></thead><tbody id="dl"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Unggah dokumen</h2><input class="form-control mb-2" id="dn" placeholder="nama berkas, mis. kontrak.pdf"><input class="form-control mb-2" type="file" id="df"><button class="btn btn-primary" id="doup" type="button">Unggah</button> <span class="text-danger" id="ue"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Bagikan dokumen</h2><input class="form-control mb-2" id="sh-doc" placeholder="ID dokumen"><input class="form-control mb-2" id="sh-hours" placeholder="masa berlaku (jam, default 72)"><button class="btn btn-secondary" id="sh-btn" type="button">Buat tautan</button> <span class="text-danger" id="sh-msg"></span><div id="sh-list"></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Detail &amp; riwayat versi</h2><div class="mkt-row"><input class="form-control mkt-filter" id="doc-version-id" placeholder="ID dokumen"><button class="btn btn-secondary btn-sm" id="doc-version-btn" type="button">Tampilkan</button></div><div class="text-danger" id="doc-version-msg"></div><div id="doc-version-detail"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Checklist dokumen</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="dcl-doc" placeholder="ID dokumen" inputmode="numeric"><input class="form-control" id="dcl-item" placeholder="butir pemeriksaan"><button class="btn btn-primary btn-sm" id="dcl-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="dcl-reload" type="button">Muat</button></div><div class="text-danger" id="dcl-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Butir</th><th>Selesai</th><th>Aksi</th></tr></thead><tbody id="dcl-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pustaka SOP</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="sop-code" placeholder="kode, mis. SOP-ARSIP"><input class="form-control" id="sop-title" placeholder="judul SOP"><input class="form-control" id="sop-body" placeholder="isi SOP"><button class="btn btn-primary btn-sm" id="sop-add" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="sop-reload" type="button">Muat ulang</button></div><div class="text-danger" id="sop-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Judul</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="sop-list"></tbody></table></div></div></div></div><div id="p-ocr" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Pekerjaan OCR</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Tipe</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="ocr-jobs"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Proses OCR</h2><input class="form-control mb-2" id="ocr-doc" placeholder="ID dokumen"><select class="form-control mb-2" id="ocr-type"><option value="ktp">ktp</option><option value="kk">kk</option><option value="npwp">npwp</option><option value="passport">passport</option><option value="land_certificate">land_certificate</option><option value="deed">deed</option><option value="contract">contract</option><option value="general">general</option></select><button class="btn btn-primary" id="ocr-btn" type="button">Buat job</button> <button class="btn btn-outline-secondary" id="ocr-acc-btn" type="button">Akurasi CSV</button> <span class="text-danger" id="ocr-msg"></span><p class="text-muted mt-2">Mesin ekstraksi native (tesseract) live bila terinstall di host — job processed menyimpan baris + confidence; tanpa engine 501.</p></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Detail &amp; Review</h2><div id="ocr-detail"><span class="text-muted">Pilih Lihat pada baris pekerjaan.</span></div></div></div></div><div id="p-akta" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Akta</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="al"></tbody></table></div></div><div class="text-muted mt-2" id="ae"></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Buat Akta dari Template</h2><select class="form-control mb-2" id="deed-tpl"><option value="">Pilih template…</option></select><input class="form-control mb-2" id="deed-matter" placeholder="ID perkara"><input class="form-control mb-2" id="deed-number" placeholder="nomor akta"><input class="form-control mb-2" id="deed-type" placeholder="jenis akta"><input class="form-control mb-2" id="deed-vars" placeholder="variabel JSON, mis. {&#34;hari&#34;:&#34;Senin&#34;}"><button class="btn btn-primary" id="deed-btn" type="button">Buat akta</button> <span class="text-danger" id="deed-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Repertorium (buku harian akta)</h2><div class="mkt-row"><input class="form-control mkt-filter" id="deed-rep-from" placeholder="dari YYYY-MM-DD (opsional)"><input class="form-control mkt-filter" id="deed-rep-to" placeholder="sampai YYYY-MM-DD (opsional)"><button class="btn btn-outline-secondary btn-sm" id="deed-csv" type="button">Ekspor CSV</button><span class="text-danger" id="deed-csv-msg"></span></div><div class="mkt-row mt-2"><input class="form-control mkt-filter" id="deed-cmp-from" placeholder="banding dari YYYY-MM-DD"><input class="form-control mkt-filter" id="deed-cmp-to" placeholder="banding sampai YYYY-MM-DD"><button class="btn btn-outline-secondary btn-sm" id="deed-cmp" type="button">Bandingkan periode</button></div><p class="text-muted">Nomor, jenis, status, dan tanggal pembuatan setiap akta — saring per hari atau rentang tanggal untuk buku harian.</p></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Sertifikat tanah (intake)</h2><div class="mkt-row"><input class="form-control mkt-filter" id="cert-doc" placeholder="ID dokumen sertifikat"><button class="btn btn-primary btn-sm" id="cert-btn" type="button">Buat job sertifikat</button><button class="btn btn-secondary btn-sm" id="cert-reload" type="button">Muat ulang</button></div><div class="text-danger" id="cert-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID job</th><th>Status</th></tr></thead><tbody id="cert-list"></tbody></table></div><p class="text-muted">Hasil ekstraksi direview di tab OCR &amp; Ekstraksi.</p></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Register PPAT</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ppat-number" placeholder="nomor akta, mis. 12/2026"><select class="form-control mkt-filter" id="ppat-kind"><option value="ajb">ajb</option><option value="hibah">hibah</option><option value="tukar_menukar">tukar_menukar</option><option value="apht">apht</option><option value="skmht">skmht</option><option value="sewa">sewa</option><option value="lainnya">lainnya</option></select><input class="form-control" id="ppat-seller" placeholder="penjual/pemberi"><input class="form-control" id="ppat-buyer" placeholder="pembeli/penerima"><input class="form-control" id="ppat-object" placeholder="objek, mis. SHM 001/Bogor"><button class="btn btn-primary btn-sm" id="ppat-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="ppat-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ppat-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Jenis</th><th>Para pihak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="ppat-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Buku protokol</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="pb-no" placeholder="nomor buku, mis. W1/2026"><input class="form-control mkt-filter" id="pb-year" placeholder="tahun, mis. 2026"><button class="btn btn-primary btn-sm" id="pb-add" type="button">Buka</button><button class="btn btn-secondary btn-sm" id="pb-reload" type="button">Muat ulang</button></div><div class="text-danger" id="pb-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nomor</th><th>Tahun</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="pb-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Serah terima protokol</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="ph-book" placeholder="ID buku" inputmode="numeric"><input class="form-control" id="ph-from" placeholder="dari"><input class="form-control" id="ph-to" placeholder="kepada"><input class="form-control mkt-filter" id="ph-at" placeholder="tanggal YYYY-MM-DD"><input class="form-control" id="ph-note" placeholder="catatan"><button class="btn btn-primary btn-sm" id="ph-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="ph-reload" type="button">Muat</button></div><div class="text-danger" id="ph-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Dari → Ke</th><th>Tanggal</th><th>Aksi</th></tr></thead><tbody id="ph-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Transfer protokol ke MPD</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="mpd-book" placeholder="ID buku (opsional)" inputmode="numeric"><input class="form-control" id="mpd-to" placeholder="MPD penerima"><input class="form-control mkt-filter" id="mpd-date" placeholder="tanggal YYYY-MM-DD"><input class="form-control mkt-filter" id="mpd-rcpt" placeholder="no. berita acara"><button class="btn btn-primary btn-sm" id="mpd-add" type="button">Siapkan</button><button class="btn btn-secondary btn-sm" id="mpd-reload" type="button">Muat ulang</button></div><div class="text-danger" id="mpd-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Buku</th><th>Penerima</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="mpd-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Bundel KYC penjual–pembeli</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="kyc-ppat" placeholder="ID akta PPAT" inputmode="numeric"><select class="form-control mkt-filter" id="kyc-role"><option value="seller">seller</option><option value="buyer">buyer</option><option value="witness">witness</option><option value="spouse">spouse</option><option value="heir">heir</option></select><input class="form-control" id="kyc-name" placeholder="nama pihak"><input class="form-control mkt-filter" id="kyc-id" placeholder="NIK/ID"><button class="btn btn-primary btn-sm" id="kyc-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="kyc-reload" type="button">Muat</button><span class="text-muted" id="kyc-sum"></span></div><div class="text-danger" id="kyc-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Peran</th><th>Nama</th><th>Terverifikasi</th><th>Aksi</th></tr></thead><tbody id="kyc-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Berita acara serah terima sertifikat</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="cho-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control" id="cho-to" placeholder="diserahkan kepada"><input class="form-control mkt-filter" id="cho-at" placeholder="tanggal YYYY-MM-DD"><input class="form-control mkt-filter" id="cho-rcpt" placeholder="no. berita acara"><button class="btn btn-primary btn-sm" id="cho-add" type="button">Siapkan</button><button class="btn btn-secondary btn-sm" id="cho-reload" type="button">Muat</button></div><div class="text-danger" id="cho-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kepada</th><th>Tanggal</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="cho-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">APHT / SKMHT</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="chg-ppat" placeholder="ID akta PPAT" inputmode="numeric"><select class="form-control mkt-filter" id="chg-kind"><option value="apht">apht</option><option value="skmht">skmht</option></select><input class="form-control" id="chg-cred" placeholder="kreditor"><input class="form-control mkt-filter" id="chg-ceil" placeholder="plafon minor" inputmode="numeric"><input class="form-control mkt-filter" id="chg-until" placeholder="berlaku s.d. YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="chg-add" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="chg-reload" type="button">Muat</button></div><div class="text-danger" id="chg-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Jenis</th><th>Kreditor</th><th>Plafon</th><th>Berlaku s.d.</th><th>Aksi</th></tr></thead><tbody id="chg-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Monitor kedaluwarsa SKMHT</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="chgd-days" value="30" inputmode="numeric"><button class="btn btn-secondary btn-sm" id="chgd-reload" type="button">Muat ulang</button><span class="text-muted">SKMHT berlaku 1 bulan (terdaftar) / 3 bulan (belum terdaftar).</span></div><div class="text-danger" id="chgd-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Akta</th><th>Jenis</th><th>Berlaku s.d.</th></tr></thead><tbody id="chgd-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Kesiapan akta (per jenis)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="rdy-ppat" placeholder="ID akta PPAT" inputmode="numeric"><button class="btn btn-secondary btn-sm" id="rdy-reload" type="button">Periksa</button><span class="text-muted" id="rdy-sum"></span></div><div class="text-danger" id="rdy-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Syarat</th><th>Status</th><th>Detail</th></tr></thead><tbody id="rdy-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Klapper (indeks alfabetis)</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="klapper-reload" type="button">Muat ulang</button></div><div class="text-danger" id="klapper-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Nama</th><th>Jenis</th><th>Referensi</th></tr></thead><tbody id="klapper-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Dasbor kinerja kantor</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="perf-reload" type="button">Muat ulang</button></div><div class="text-danger" id="perf-msg"></div><div id="perf-sum"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Volume akta</h2></div><div class="card-body"><div class="mkt-row"><select class="form-control mkt-filter" id="vol-group"><option value="month">per bulan</option><option value="state">per status</option></select><button class="btn btn-secondary btn-sm" id="vol-reload" type="button">Muat ulang</button></div><div class="text-danger" id="vol-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Kelompok</th><th>Jumlah</th></tr></thead><tbody id="vol-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Box penyimpanan akta</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="sbox-code" placeholder="kode box, mis. BOX-A01"><input class="form-control" id="sbox-loc" placeholder="lokasi"><input class="form-control mkt-filter" id="sbox-cap" placeholder="kapasitas" inputmode="numeric"><button class="btn btn-primary btn-sm" id="sbox-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="sbox-reload" type="button">Muat ulang</button></div><div class="text-danger" id="sbox-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Lokasi</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="sbox-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Penyegelan akhir tahun</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="seal-year" placeholder="tahun, mis. 2026"><input class="form-control" id="seal-note" placeholder="catatan"><button class="btn btn-primary btn-sm" id="seal-add" type="button">Siapkan</button><button class="btn btn-secondary btn-sm" id="seal-reload" type="button">Muat ulang</button></div><div class="text-danger" id="seal-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Tahun</th><th>Akta</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="seal-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Tabel zona (ZNT/NJOP)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="zone-code" placeholder="kode area"><input class="form-control" id="zone-name" placeholder="nama area"><input class="form-control mkt-filter" id="zone-njop" placeholder="NJOP minor" inputmode="numeric"><button class="btn btn-primary btn-sm" id="zone-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="zone-reload" type="button">Muat ulang</button></div><div class="text-danger" id="zone-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>NJOP</th><th>Aksi</th></tr></thead><tbody id="zone-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Worksheet pajak (PPh &amp; BPHTB)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="tax-value" placeholder="nilai transaksi minor" inputmode="numeric"><input class="form-control mkt-filter" id="tax-npoptkp" placeholder="NPOPTKP (default 60jt)"><button class="btn btn-primary btn-sm" id="tax-calc" type="button">Hitung</button></div><div class="text-danger" id="tax-msg"></div><div id="tax-out"></div><p class="text-muted">PPh final 2,5% x nilai bruto; BPHTB 5% x (NPOP − NPOPTKP). Asumsi terdokumentasi — bukan advis pajak.</p></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Verifikasi PBB (manual)</h2></div><div class="card-body"><p class="text-muted">Log manual — pengecekan eksternal belum tersedia.</p><div class="mkt-row"><input class="form-control mkt-filter" id="pbb-nop" placeholder="NOP"><input class="form-control mkt-filter" id="pbb-year" placeholder="tahun YYYY"><select class="form-control mkt-filter" id="pbb-res"><option value="unknown">unknown</option><option value="paid">paid</option><option value="unpaid">unpaid</option></select><input class="form-control mkt-filter" id="pbb-on" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="pbb-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="pbb-reload" type="button">Muat ulang</button></div><div class="text-danger" id="pbb-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>NOP</th><th>Tahun</th><th>Hasil</th><th>Aksi</th></tr></thead><tbody id="pbb-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pemisahan tanah (pemecahan)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="sp-src" placeholder="bidang sumber"><input class="form-control" id="sp-new" placeholder="bidang hasil, pisah koma"><input class="form-control mkt-filter" id="sp-date" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="sp-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="sp-reload" type="button">Muat ulang</button></div><div class="text-danger" id="sp-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Sumber</th><th>Hasil</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="sp-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Penggabungan tanah</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="mg-src" placeholder="bidang sumber, pisah koma"><input class="form-control mkt-filter" id="mg-res" placeholder="bidang hasil"><input class="form-control mkt-filter" id="mg-date" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="mg-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="mg-reload" type="button">Muat ulang</button></div><div class="text-danger" id="mg-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Sumber</th><th>Hasil</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="mg-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Flag sengketa tanah</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="dsp2-parcel" placeholder="referensi bidang"><select class="form-control mkt-filter" id="dsp2-kind"><option value="sengketa">sengketa</option><option value="batas">batas</option><option value="ganda">ganda</option><option value="lainnya">lainnya</option></select><input class="form-control" id="dsp2-desc" placeholder="uraian"><button class="btn btn-primary btn-sm" id="dsp2-add" type="button">Tandai</button><button class="btn btn-secondary btn-sm" id="dsp2-reload" type="button">Muat ulang</button></div><div class="text-danger" id="dsp2-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Bidang</th><th>Jenis</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="dsp2-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Verifikasi batas &amp; luas</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="bnd-parcel" placeholder="referensi bidang"><input class="form-control mkt-filter" id="bnd-decl" placeholder="luas surat (m2)" inputmode="numeric"><input class="form-control mkt-filter" id="bnd-meas" placeholder="luas ukur (m2)" inputmode="numeric"><input class="form-control mkt-filter" id="bnd-on" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="bnd-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="bnd-reload" type="button">Muat ulang</button></div><div class="text-danger" id="bnd-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Bidang</th><th>Surat vs Ukur</th><th>Hasil</th><th>Aksi</th></tr></thead><tbody id="bnd-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Persetujuan pasangan (harta bersama)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="mrt-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control" id="mrt-party" placeholder="nama pasangan pemberi persetujuan"><button class="btn btn-primary btn-sm" id="mrt-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="mrt-reload" type="button">Muat</button></div><div class="text-danger" id="mrt-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pihak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="mrt-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Persetujuan ahli waris</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="hir-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control" id="hir-party" placeholder="nama ahli waris"><button class="btn btn-primary btn-sm" id="hir-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="hir-reload" type="button">Muat</button></div><div class="text-danger" id="hir-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pihak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="hir-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Resolusi direksi korporasi</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="bod-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control" id="bod-party" placeholder="nama korporasi penjual"><button class="btn btn-primary btn-sm" id="bod-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="bod-reload" type="button">Muat</button></div><div class="text-danger" id="bod-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pihak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="bod-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Kuasa menjual</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="pow-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control" id="pow-party" placeholder="nama penerima kuasa"><button class="btn btn-primary btn-sm" id="pow-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="pow-reload" type="button">Muat</button></div><div class="text-danger" id="pow-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pihak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="pow-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Riwayat tanah</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="lhi-parcel" placeholder="referensi bidang"><select class="form-control mkt-filter" id="lhi-event"><option value="alas_hak">alas_hak</option><option value="jual_beli">jual_beli</option><option value="hibah">hibah</option><option value="waris">waris</option><option value="split">split</option><option value="merge">merge</option><option value="sengketa">sengketa</option><option value="lainnya">lainnya</option></select><input class="form-control" id="lhi-detail" placeholder="uraian peristiwa"><input class="form-control mkt-filter" id="lhi-date" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="lhi-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="lhi-reload" type="button">Muat</button></div><div class="text-danger" id="lhi-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Peristiwa</th><th>Tanggal</th><th>Uraian</th><th>Aksi</th></tr></thead><tbody id="lhi-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Serah terima unit (BAST)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="uho-unit" placeholder="referensi unit"><input class="form-control mkt-filter" id="uho-from" placeholder="diserahkan oleh"><input class="form-control mkt-filter" id="uho-to" placeholder="diterima oleh"><input class="form-control mkt-filter" id="uho-bast" placeholder="no. BAST"><input class="form-control mkt-filter" id="uho-on" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="uho-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="uho-reload" type="button">Muat ulang</button></div><div class="text-danger" id="uho-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Unit</th><th>BAST</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="uho-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Laporan bulanan PPAT</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="rpm-month" placeholder="bulan YYYY-MM"><button class="btn btn-secondary btn-sm" id="rpm-reload" type="button">Muat ulang</button><span class="text-muted" id="rpm-total"></span></div><div class="text-danger" id="rpm-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Jenis</th><th>Status</th><th>Jumlah</th></tr></thead><tbody id="rpm-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Dasbor PPAT</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="pdb-reload" type="button">Muat ulang</button></div><div class="text-danger" id="pdb-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Metrik</th><th>Nilai</th></tr></thead><tbody id="pdb-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Guardrail kepemilikan asing</h2></div><div class="card-body"><div class="mkt-row"><select class="form-control mkt-filter" id="fgd-sub"><option value="wni">wni</option><option value="wna">wna</option><option value="pma">pma</option><option value="pmnd">pmnd</option><option value="badan_wni">badan_wni</option></select><select class="form-control mkt-filter" id="fgd-right"><option value="shm">shm</option><option value="hgb">hgb</option><option value="hak_pakai">hak_pakai</option><option value="hak_guna_usaha">hak_guna_usaha</option><option value="girik">girik</option></select><button class="btn btn-secondary btn-sm" id="fgd-check" type="button">Periksa</button></div><div class="text-danger" id="fgd-msg"></div><div id="fgd-out" class="text-muted">Pilih subjek dan hak lalu Periksa.</div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Serah terima notaris–PPAT</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="nho-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control" id="nho-doc" placeholder="nama dokumen"><button class="btn btn-primary btn-sm" id="nho-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="nho-reload" type="button">Muat</button></div><div class="text-danger" id="nho-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Dokumen</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="nho-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Intake girik / tanah adat</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="grk-ref" placeholder="ref. Letter C"><input class="form-control mkt-filter" id="grk-holder" placeholder="pemegang"><input class="form-control mkt-filter" id="grk-area" placeholder="luas (m2)" inputmode="numeric"><input class="form-control mkt-filter" id="grk-vil" placeholder="desa/kelurahan"><button class="btn btn-primary btn-sm" id="grk-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="grk-reload" type="button">Muat ulang</button></div><div class="text-danger" id="grk-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Letter C</th><th>Pemegang</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="grk-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Ketentuan sewa</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="lse-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control mkt-filter" id="lse-month" placeholder="sewa/bulan" inputmode="numeric"><input class="form-control mkt-filter" id="lse-dur" placeholder="durasi (bln)" inputmode="numeric"><input class="form-control mkt-filter" id="lse-start" placeholder="mulai YYYY-MM-DD"><input class="form-control mkt-filter" id="lse-dep" placeholder="deposit" inputmode="numeric"><button class="btn btn-primary btn-sm" id="lse-save" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="lse-load" type="button">Muat</button><button class="btn btn-sm btn-outline-danger" id="lse-del" type="button">Hapus</button></div><div class="text-danger" id="lse-msg"></div><div id="lse-out" class="text-muted">Isi ID akta lalu Muat.</div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Batch bulk developer</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="dvb-dev" placeholder="nama developer"><input class="form-control mkt-filter" id="dvb-no" placeholder="no. batch"><input class="form-control mkt-filter" id="dvb-total" placeholder="total unit" inputmode="numeric"><button class="btn btn-primary btn-sm" id="dvb-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="dvb-reload" type="button">Muat ulang</button></div><div class="text-danger" id="dvb-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Developer</th><th>Batch</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="dvb-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Stok unit developer</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="dvu-batch" placeholder="ID batch (opsional)" inputmode="numeric"><input class="form-control mkt-filter" id="dvu-code" placeholder="kode unit"><select class="form-control mkt-filter" id="dvu-type"><option value="rumah">rumah</option><option value="ruko">ruko</option><option value="apartemen">apartemen</option><option value="kavling">kavling</option><option value="lainnya">lainnya</option></select><input class="form-control mkt-filter" id="dvu-price" placeholder="harga" inputmode="numeric"><button class="btn btn-primary btn-sm" id="dvu-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="dvu-reload" type="button">Muat ulang</button></div><div class="text-danger" id="dvu-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Tipe</th><th>Harga</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="dvu-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Koordinasi KPR bank</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="kpr-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control mkt-filter" id="kpr-bank" placeholder="nama bank"><input class="form-control mkt-filter" id="kpr-amt" placeholder="plafon" inputmode="numeric"><button class="btn btn-primary btn-sm" id="kpr-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="kpr-reload" type="button">Muat</button></div><div class="text-danger" id="kpr-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Bank</th><th>Plafon</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="kpr-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Register protokol</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="prg-date" placeholder="tanggal YYYY-MM-DD"><input class="form-control mkt-filter" id="prg-book" placeholder="ref. buku"><input class="form-control mkt-filter" id="prg-deed" placeholder="no. akta"><input class="form-control mkt-filter" id="prg-party" placeholder="pihak"><select class="form-control mkt-filter" id="prg-dir"><option value="in">masuk</option><option value="out">keluar</option></select><button class="btn btn-primary btn-sm" id="prg-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="prg-reload" type="button">Muat ulang</button></div><div class="text-danger" id="prg-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Tanggal</th><th>Buku</th><th>Akta</th><th>Arah</th><th>Aksi</th></tr></thead><tbody id="prg-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Draf AJB</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="ajb-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control mkt-filter" id="ajb-tpl" placeholder="nama template"><input class="form-control mkt-filter" id="ajb-price" placeholder="harga" inputmode="numeric"><button class="btn btn-secondary btn-sm" id="ajb-reload" type="button">Muat</button></div><div class="mkt-row"><textarea class="form-control" id="ajb-body" rows="3" placeholder="Isi draf AJB (min. 20 karakter)"></textarea><button class="btn btn-primary btn-sm" id="ajb-save" type="button">Simpan draf</button></div><div class="text-danger" id="ajb-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Template</th><th>Harga</th><th>Pratinjau</th><th>Aksi</th></tr></thead><tbody id="ajb-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Jejak audit PPAT</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="pat-limit" value="50" inputmode="numeric"><button class="btn btn-secondary btn-sm" id="pat-reload" type="button">Muat ulang</button><span class="text-muted">Menggabung 16 entitas properti.</span></div><div class="text-danger" id="pat-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Aksi</th><th>Entitas</th><th>Waktu</th></tr></thead><tbody id="pat-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Brankas sertifikat</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="vlt-ppat" placeholder="ID akta PPAT" inputmode="numeric"><input class="form-control mkt-filter" id="vlt-doc" placeholder="ID dokumen" inputmode="numeric"><input class="form-control mkt-filter" id="vlt-label" placeholder="label (mis. SHM asli)"><button class="btn btn-primary btn-sm" id="vlt-add" type="button">Tautkan</button><button class="btn btn-secondary btn-sm" id="vlt-reload" type="button">Muat</button></div><div class="text-danger" id="vlt-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Dokumen</th><th>Label</th><th>Aksi</th></tr></thead><tbody id="vlt-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Permohonan verifikasi BPN</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="bpnv-ref" placeholder="no. permohonan"><input class="form-control mkt-filter" id="bpnv-office" placeholder="kantor pertanahan"><button class="btn btn-primary btn-sm" id="bpnv-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="bpnv-reload" type="button">Muat ulang</button></div><div class="text-danger" id="bpnv-msg"></div><div class="text-muted">Mode manual — sinkronisasi otomatis menunggu kredensial BPN.</div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>No. permohonan</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="bpnv-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Nomor antrean BPN</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="bpnq-ref" placeholder="no. antrean"><input class="form-control mkt-filter" id="bpnq-office" placeholder="kantor pertanahan"><button class="btn btn-primary btn-sm" id="bpnq-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="bpnq-reload" type="button">Muat ulang</button></div><div class="text-danger" id="bpnq-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>No. antrean</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="bpnq-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Arsip respons BPN</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="bpnr-ref" placeholder="no. surat / berkas"><input class="form-control mkt-filter" id="bpnr-office" placeholder="kantor pertanahan"><button class="btn btn-primary btn-sm" id="bpnr-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="bpnr-reload" type="button">Muat ulang</button></div><div class="text-danger" id="bpnr-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Referensi</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="bpnr-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pustaka klausa</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="clause-name" placeholder="nama klausa"><input class="form-control mkt-filter" id="clause-cat" placeholder="kategori"><button class="btn btn-primary btn-sm" id="clause-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="clause-reload" type="button">Muat ulang</button></div><div class="text-danger" id="clause-msg"></div><div class="mkt-row"><input class="form-control" id="clause-body" placeholder="isi klausa baru (gunakan {{nama}} untuk variabel)"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Kategori</th><th>v</th><th>Aksi</th></tr></thead><tbody id="clause-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Template Baru</h2><input class="form-control mb-2" id="tpl-name" placeholder="nama template"><input class="form-control mb-2" id="tpl-body" placeholder="isi template, gunakan {{nama}} untuk variabel"><button class="btn btn-secondary" id="tpl-btn" type="button">Simpan template</button> <span class="text-danger" id="tpl-msg"></span><p class="text-muted mt-2">Tanda tangan digital (PSrE) belum dikonfigurasi — endpoint esign 501.</p></div></div></div><div id="p-keu" class="hidden"><div class="row g-2"><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Invoice per perkara</h2></div><div class="card-body"><input class="form-control mb-2" id="km" placeholder="ID perkara"><button class="btn btn-secondary mb-2" id="showinv" type="button">Tampilkan</button><div class="text-danger mb-2" id="il-msg"></div><div id="il"></div></div></div></div><div class="col-md-6"><div class="card"><div class="card-header"><h2 class="card-title">Trust transfer</h2></div><div class="card-body"><input class="form-control mb-2" id="tf" placeholder="dari rekening ID"><input class="form-control mb-2" id="tt" placeholder="ke rekening ID"><input class="form-control mb-2" id="ta" placeholder="nominal minor"><button class="btn btn-primary" id="dotrf" type="button">Transfer</button> <span class="text-danger" id="te"></span></div></div></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Buat Invoice</h2><input class="form-control mb-2" id="invm-matter" placeholder="ID perkara"><input class="form-control mb-2" id="invm-total" placeholder="total minor"><button class="btn btn-primary" id="invm-btn" type="button">Buat</button> <span class="text-danger" id="invm-msg"></span></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Ekspor Laporan</h2><button class="btn btn-outline-secondary btn-sm" id="keu-csv" type="button">Perkara CSV</button> <input class="form-control mkt-filter" id="trust-csv-acct" placeholder="ID rekening trust"><button class="btn btn-outline-secondary btn-sm" id="trust-csv-btn" type="button">Trust CSV</button> <span class="text-danger" id="keu-msg"></span><span class="text-danger" id="ar-msg"></span></div><div class="mkt-row mt-2"><strong>Piutang terbuka (AR)</strong><button class="btn btn-outline-secondary btn-sm" id="ar-csv" type="button">Ekspor AR CSV</button><button class="btn btn-outline-secondary btn-sm" id="appr-csv" type="button">Bottleneck approval</button><button class="btn btn-outline-secondary btn-sm" id="prof-csv" type="button">Profitabilitas klien</button></div><p class="text-muted">Semua invoice belum lunas beserta total, terbayar, dan sisa.</p><span class="text-danger" id="trust-csv-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Rekening trust</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Klien</th><th>Saldo</th><th>Aksi</th></tr></thead><tbody id="tl"></tbody></table></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="trust-view-id" placeholder="ID rekening"><button class="btn btn-secondary btn-sm" id="trust-view" type="button">Lihat mutasi</button></div><div class="text-danger" id="trust-view-msg"></div><div id="trust-detail"></div><div class="mkt-row mt-2"><select class="form-control mkt-filter" id="trust-entry-kind"><option value="credit">credit</option><option value="debit">debit</option></select><input class="form-control mkt-filter" id="trust-entry-matter" placeholder="ID perkara (opsional)"><input class="form-control mkt-filter" id="trust-entry-amount" placeholder="nominal minor" inputmode="numeric"><button class="btn btn-primary btn-sm" id="trust-entry-add" type="button">Catat mutasi</button></div><div class="text-danger" id="trust-entry-msg"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Daftar harga layanan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="price-code" placeholder="kode, mis. AJB-STD"><input class="form-control" id="price-name" placeholder="nama layanan"><input class="form-control mkt-filter" id="price-amount" placeholder="nominal minor" inputmode="numeric"><button class="btn btn-primary btn-sm" id="price-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="price-reload" type="button">Muat ulang</button></div><div class="text-danger" id="price-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Layanan</th><th>Nominal</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="price-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Penawaran biaya akta (quotation)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="quot-client" placeholder="nama klien"><input class="form-control mkt-filter" id="quot-kind" placeholder="jenis akta"><input class="form-control" id="quot-items" placeholder="item kode:qty, mis. AJB-STD:1,METERAI:2"><button class="btn btn-primary btn-sm" id="quot-add" type="button">Buat</button><button class="btn btn-secondary btn-sm" id="quot-reload" type="button">Muat ulang</button></div><div class="text-danger" id="quot-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Klien</th><th>Total</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="quot-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Kas kantor</h2></div><div class="card-body"><div class="mkt-row"><select class="form-control mkt-filter" id="cash-kind"><option value="in">masuk</option><option value="out">keluar</option></select><input class="form-control mkt-filter" id="cash-amount" placeholder="nominal minor" inputmode="numeric"><input class="form-control" id="cash-note" placeholder="catatan"><input class="form-control mkt-filter" id="cash-date" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="cash-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="cash-reload" type="button">Muat ulang</button><span class="text-muted" id="cash-sum"></span></div><div class="text-danger" id="cash-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Jenis</th><th>Nominal</th><th>Tanggal</th><th>Aksi</th></tr></thead><tbody id="cash-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Panjar / IOU</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="adv-holder" placeholder="pemegang panjar"><input class="form-control mkt-filter" id="adv-amount" placeholder="nominal minor" inputmode="numeric"><input class="form-control" id="adv-purpose" placeholder="keperluan"><button class="btn btn-primary btn-sm" id="adv-add" type="button">Beri</button><button class="btn btn-secondary btn-sm" id="adv-reload" type="button">Muat ulang</button></div><div class="text-danger" id="adv-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pemegang</th><th>Nominal</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="adv-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Inventaris kantor</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="inv-code" placeholder="kode, mis. METERAI-10K"><input class="form-control" id="inv-name" placeholder="nama barang"><input class="form-control mkt-filter" id="inv-qty" placeholder="stok awal" inputmode="numeric"><input class="form-control mkt-filter" id="inv-min" placeholder="stok min" inputmode="numeric"><button class="btn btn-primary btn-sm" id="inv-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="inv-reload" type="button">Muat ulang</button></div><div class="text-danger" id="inv-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Stok</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="inv-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">ATK &amp; habis pakai</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="atk-code" placeholder="kode, mis. ATK-KERTAS"><input class="form-control" id="atk-name" placeholder="nama barang"><input class="form-control mkt-filter" id="atk-qty" placeholder="stok awal" inputmode="numeric"><button class="btn btn-primary btn-sm" id="atk-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="atk-reload" type="button">Muat ulang</button></div><div class="text-danger" id="atk-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Stok</th><th>Status</th></tr></thead><tbody id="atk-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pemakaian meterai/blanko</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="use-item" placeholder="ID barang" inputmode="numeric"><input class="form-control mkt-filter" id="use-qty" placeholder="jumlah" inputmode="numeric"><input class="form-control" id="use-need" placeholder="keperluan"><input class="form-control mkt-filter" id="use-on" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="use-add" type="button">Catat pakai</button><button class="btn btn-secondary btn-sm" id="use-reload" type="button">Muat</button></div><div class="text-danger" id="use-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Jumlah</th><th>Keperluan</th><th>Tanggal</th><th>Aksi</th></tr></thead><tbody id="use-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Peta ruang arsip</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="abox-code" placeholder="kode box, mis. ARS-001"><input class="form-control" id="abox-loc" placeholder="lokasi, mis. Ruang A / Rak 3"><input class="form-control" id="abox-desc" placeholder="isi box"><button class="btn btn-primary btn-sm" id="abox-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="abox-reload" type="button">Muat ulang</button></div><div class="text-danger" id="abox-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Lokasi</th><th>Aksi</th></tr></thead><tbody id="abox-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Uang muka</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="dp-no" placeholder="no. kwitansi, mis. UM/2026/001"><input class="form-control" id="dp-client" placeholder="nama klien"><input class="form-control mkt-filter" id="dp-matter" placeholder="ID perkara (opsional)" inputmode="numeric"><input class="form-control mkt-filter" id="dp-amount" placeholder="nominal minor" inputmode="numeric"><input class="form-control mkt-filter" id="dp-date" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="dp-add" type="button">Terbitkan</button><button class="btn btn-secondary btn-sm" id="dp-reload" type="button">Muat ulang</button></div><div class="text-danger" id="dp-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kwitansi</th><th>Nominal</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="dp-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Fasilitas bank rekanan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="bank-name" placeholder="nama bank"><input class="form-control" id="bank-fac" placeholder="fasilitas, mis. KPR"><input class="form-control mkt-filter" id="bank-rate" placeholder="suku bunga, mis. 9,5%"><input class="form-control mkt-filter" id="bank-tenor" placeholder="tenor"><button class="btn btn-primary btn-sm" id="bank-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="bank-reload" type="button">Muat ulang</button></div><div class="text-danger" id="bank-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Bank</th><th>Fasilitas</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="bank-list"></tbody></table></div></div></div></div><div id="p-mkt" class="hidden"><div class="text-danger" id="mkt-err"></div><div class="card"><div class="card-header"><h2 class="card-title">Leads pipeline</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="mkt-status-filter">Filter status</label><select class="form-control mkt-filter" id="mkt-status-filter"><option value="">Semua</option><option value="new">new</option><option value="contacted">contacted</option><option value="qualified">qualified</option><option value="demo">demo</option><option value="proposal">proposal</option><option value="negotiation">negotiation</option><option value="won">won</option><option value="lost">lost</option></select><button class="btn btn-secondary btn-sm" id="mkt-reload" type="button">Muat ulang</button><button class="btn btn-outline-secondary btn-sm" id="mkt-csv" type="button">Ekspor CSV</button></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Email</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="mkt-leads"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Permintaan demo</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Email</th><th>Perusahaan</th><th>Status</th></tr></thead><tbody id="mkt-demos"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pesan kontak</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Subjek</th><th>Status</th></tr></thead><tbody id="mkt-contacts"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Umpan balik</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="fb-name" placeholder="nama (opsional)"><input class="form-control" id="fb-msg-in" placeholder="pesan umpan balik"><select class="form-control mkt-filter" id="fb-rating"><option value="0">tanpa nilai</option><option value="1">1</option><option value="2">2</option><option value="3">3</option><option value="4">4</option><option value="5">5</option></select><button class="btn btn-primary btn-sm" id="fb-add" type="button">Kirim</button><button class="btn btn-secondary btn-sm" id="fb-reload" type="button">Muat ulang</button></div><div class="text-danger" id="fb-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Pesan</th><th>Nilai</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="fb-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Newsletter</h2></div><div class="card-body"><div class="mkt-row"><span>Jumlah pelanggan: <strong id="mkt-news-count">0</strong></span><input class="form-control mkt-mail" id="mkt-sub-email" type="email" placeholder="email@kantor.id"><button class="btn btn-primary btn-sm" id="mkt-sub-btn" type="button">Uji subscribe</button><button class="btn btn-outline-secondary btn-sm" id="mkt-unsub-btn" type="button">Berhenti</button></div><div class="text-muted" id="mkt-sub-msg"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Paket harga</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Kode</th><th>Nama</th><th>Bulanan</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="mkt-plans"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">FAQ</h2></div><div class="card-body"><div class="table-responsive"><table class="table card-table"><thead><tr><th>Slug</th><th>Pertanyaan</th></tr></thead><tbody id="mkt-faqs"></tbody></table></div><div class="mkt-row"><input class="form-control" id="faq-slug" placeholder="slug"><input class="form-control" id="faq-q" placeholder="pertanyaan"><input class="form-control" id="faq-a" placeholder="jawaban"><button class="btn btn-primary btn-sm" id="mkt-faq-add" type="button">Tambah FAQ</button></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Artikel</h2></div><div class="card-body"><div class="table-responsive"><table class="table card-table"><thead><tr><th>Slug</th><th>Judul</th><th>Terbit</th><th>Aksi</th></tr></thead><tbody id="mkt-articles"></tbody></table></div><div class="mkt-row"><input class="form-control" id="art-slug" placeholder="slug"><input class="form-control" id="art-title" placeholder="judul"><input class="form-control" id="art-body" placeholder="isi markdown"><label class="form-check"><input class="form-check-input" type="checkbox" id="art-pub"><span class="form-check-label">Terbit</span></label><button class="btn btn-primary btn-sm" id="mkt-art-add" type="button">Simpan artikel</button></div></div></div></div><div id="p-audit" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Jejak Audit</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="audit-entity">Filter entitas</label><select class="form-control mkt-filter" id="audit-entity"><option value="">Semua</option><option value="matter">matter</option><option value="document">document</option><option value="deed">deed</option><option value="notification">notification</option><option value="user">user</option><option value="ppat">ppat</option><option value="report">report</option><option value="trust">trust</option><option value="ropa">ropa</option></select><button class="btn btn-secondary btn-sm" id="audit-reload" type="button">Muat ulang</button></div><div class="text-danger" id="audit-msg"></div><div class="text-muted" id="audit-note"></div></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Aktor</th><th>Aksi</th><th>Entitas</th><th>Waktu</th></tr></thead><tbody id="audit-rows"></tbody></table></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Webhook masuk</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="wh-reload" type="button">Muat ulang</button></div><div class="text-danger" id="wh-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Provider</th><th>Event</th><th>Status</th><th>Detail</th></tr></thead><tbody id="wh-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Endpoint outbound</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="whe-url" placeholder="https://mitra.id/hook"><input class="form-control mkt-filter" id="whe-secret" placeholder="secret HMAC (opsional)"><button class="btn btn-primary btn-sm" id="whe-add" type="button">Daftarkan</button><button class="btn btn-secondary btn-sm" id="whe-reload" type="button">Muat ulang</button></div><div class="text-danger" id="whe-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>URL</th><th>Event</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="whe-list"></tbody></table></div></div></div></div><div id="p-jobs" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Antrean Jobs <span class="text-muted small" id="job-summary"></span></h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Jenis</th><th>Status</th><th>Percobaan</th></tr></thead><tbody id="job-rows"></tbody></table></div></div><div class="card mt-3"><div class="card-body"><h2 class="card-title">Enqueue Job</h2><input class="form-control mb-2" id="job-kind" placeholder="jenis job, mis. notify.digest"><button class="btn btn-primary" id="job-btn" type="button">Enqueue</button> <span class="text-danger" id="job-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Kesehatan integrasi</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="int-reload" type="button">Muat ulang</button></div><div class="text-danger" id="int-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Nama</th><th>Jenis</th><th>Siap</th><th>Detail</th></tr></thead><tbody id="int-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Dead letters (gagal terkirim)</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="wh-dl-reload" type="button">Muat ulang</button></div><div class="text-danger" id="wh-dl-msg"></div></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Provider</th><th>Event</th><th>Detail</th></tr></thead><tbody id="wh-dl-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Verifikasi backup</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="bc-at" placeholder="YYYY-MM-DD HH:MM"><select class="form-control mkt-filter" id="bc-method"><option value="dump">dump</option><option value="files">files</option><option value="remote">remote</option></select><select class="form-control mkt-filter" id="bc-result"><option value="pass">pass</option><option value="fail">fail</option></select><input class="form-control" id="bc-note" placeholder="catatan"><button class="btn btn-primary btn-sm" id="bc-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="bc-reload" type="button">Muat ulang</button><span class="text-muted" id="bc-last"></span></div><div class="text-danger" id="bc-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Waktu</th><th>Metode</th><th>Hasil</th><th>Aksi</th></tr></thead><tbody id="bc-list"></tbody></table></div></div></div><div id="p-appr" class="hidden"><div class="text-danger" id="appr-err"></div><div class="row g-2"><div class="col-md-5"><div class="card"><div class="card-header"><h2 class="card-title">Buat permintaan persetujuan</h2></div><div class="card-body"><label class="form-label" for="appr-entity">Entitas</label><select class="form-control mb-2" id="appr-entity"><option value="matter">matter</option><option value="invoice">invoice</option><option value="deed">deed</option></select><input class="form-control mb-2" id="appr-entity-id" placeholder="ID entitas"><input class="form-control mb-2" id="appr-title" placeholder="judul, mis. Setujui AJB"><input class="form-control mb-2" id="appr-role1" placeholder="peran tahap 1, mis. office_admin"><input class="form-control mb-2" id="appr-role2" placeholder="peran tahap 2 (opsional)"><input class="form-control mb-2" id="appr-role3" placeholder="peran tahap 3 (opsional)"><button class="btn btn-primary" id="appr-btn" type="button">Buat</button> <span class="text-danger" id="appr-msg"></span></div></div></div><div class="col-md-7"><div class="card"><div class="card-header"><h2 class="card-title">Daftar permintaan</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="appr-filter">Filter status</label><select class="form-control mkt-filter" id="appr-filter"><option value="">Semua</option><option value="pending">pending</option><option value="needs_rework">needs_rework</option><option value="approved">approved</option><option value="rejected">rejected</option><option value="revoked">revoked</option></select><button class="btn btn-secondary btn-sm" id="appr-reload" type="button">Muat ulang</button> <button class="btn btn-outline-secondary btn-sm" id="appr-csv" type="button">Ekspor CSV</button></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Entitas</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="appr-list"></tbody></table></div><div class="mkt-row mt-2"><strong>Notifikasi approval</strong><span class="text-muted" id="appr-notif-unread"></span><label class="form-check"><input class="form-check-input" type="checkbox" id="appr-notif-unread-only"><span class="form-check-label">Hanya belum dibaca</span></label><button class="btn btn-secondary btn-sm" id="appr-notif-reload" type="button">Muat ulang</button></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Dibaca</th><th>Aksi</th></tr></thead><tbody id="appr-notif-list"></tbody></table></div><div class="mkt-row mt-2"><strong>Delegasi (pengganti cuti)</strong><input class="form-control mkt-filter" id="deleg-uid" placeholder="UID penerima kuasa"><button class="btn btn-primary btn-sm" id="deleg-add" type="button">Beri kuasa</button><button class="btn btn-secondary btn-sm" id="deleg-reload" type="button">Muat ulang</button></div><div class="text-danger" id="deleg-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pemberi</th><th>Penerima</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="deleg-list"></tbody></table></div></div></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Detail &amp; keputusan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="appr-id" placeholder="ID permintaan"><button class="btn btn-secondary btn-sm" id="appr-show" type="button">Tampilkan</button></div><div id="appr-detail"></div><div class="mkt-row"><input class="form-control mkt-filter" id="appr-step" placeholder="no. tahap"><input class="form-control mkt-filter" id="appr-comment" placeholder="komentar (opsional)"><button class="btn btn-success btn-sm" id="appr-ok" type="button">Setujui</button><button class="btn btn-danger btn-sm" id="appr-no" type="button">Tolak</button><button class="btn btn-warning btn-sm" id="appr-rw" type="button">Rework</button><button class="btn btn-outline-secondary btn-sm" id="appr-revoke" type="button">Cabut</button></div><div class="text-danger" id="appr-dmsg"></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Terlambat (overdue)</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="appr-overdue-days">Ambang hari</label><input class="form-control mkt-filter" id="appr-overdue-days" value="3" inputmode="numeric"><button class="btn btn-secondary btn-sm" id="appr-overdue-reload" type="button">Muat ulang</button></div><div class="text-danger" id="appr-overdue-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Umur (hari)</th><th>Menunggu</th></tr></thead><tbody id="appr-overdue-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Kalender persetujuan</h2></div><div class="card-body"><div class="mkt-row"><label class="form-label" for="appr-cal-month">Bulan</label><input class="form-control mkt-filter" id="appr-cal-month" placeholder="YYYY-MM (kosong = bulan ini)"><button class="btn btn-secondary btn-sm" id="appr-cal-reload" type="button">Muat ulang</button></div><div class="text-danger" id="appr-cal-msg"></div><div id="appr-cal-list"></div></div></div></div><div id="p-kepatuhan" class="hidden"><div class="text-danger" id="kep-err"></div><div class="card"><div class="card-header"><h2 class="card-title">Dasbor kepatuhan</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="posture-reload" type="button">Muat ulang</button><button class="btn btn-outline-secondary btn-sm" id="posture-csv" type="button">Ekspor CSV</button><button class="btn btn-outline-secondary btn-sm" id="regpack-csv" type="button">Paket regulator</button></div><div class="text-danger" id="posture-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Register</th><th>Total</th><th>Rincian status</th></tr></thead><tbody id="posture-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Peringatan kepatuhan</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="alerts-reload" type="button">Muat ulang</button></div><div class="text-danger" id="alerts-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Jenis</th><th>Peringatan</th></tr></thead><tbody id="alerts-list"></tbody></table></div></div></div><div class="card"><div class="card-header"><h2 class="card-title">Obligasi UU PDP</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="kep-title" placeholder="judul obligasi"><input class="form-control mkt-filter" id="kep-cat" placeholder="kategori (umum)"><input class="form-control mkt-filter" id="kep-owner" placeholder="penanggung jawab"><button class="btn btn-primary btn-sm" id="kep-add" type="button">Tambah</button><select class="form-control mkt-filter" id="kep-filter"><option value="">Semua status</option><option value="open">open</option><option value="in_progress">in_progress</option><option value="done">done</option></select><button class="btn btn-secondary btn-sm" id="kep-reload" type="button">Muat ulang</button></div><div class="text-danger" id="kep-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Kategori</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="kep-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Persetujuan (consent)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="con-subject" placeholder="subjek, mis. NIK/email"><input class="form-control" id="con-purpose" placeholder="tujuan, mis. kyc"><button class="btn btn-primary btn-sm" id="con-grant" type="button">Catat persetujuan</button><input class="form-control mkt-filter" id="con-filter" placeholder="filter subjek"><button class="btn btn-secondary btn-sm" id="con-reload" type="button">Muat ulang</button></div><div class="text-danger" id="con-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Subjek</th><th>Tujuan</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="con-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Register pemrosesan (ROPA, UU PDP)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ropa-activity" placeholder="aktivitas, mis. Pendaftaran klien"><input class="form-control" id="ropa-purpose" placeholder="tujuan, mis. Layanan notaris"><input class="form-control" id="ropa-cats" placeholder="kategori data, mis. NIK, alamat"><input class="form-control" id="ropa-basis" placeholder="dasar hukum, mis. consent"><input class="form-control" id="ropa-ret" placeholder="retensi, mis. 10 tahun"><button class="btn btn-primary btn-sm" id="ropa-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="ropa-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ropa-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Aktivitas</th><th>Tujuan</th><th>Kategori</th><th>Dasar</th><th>Retensi</th><th>Aksi</th></tr></thead><tbody id="ropa-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">DPIA (penilaian dampak)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="dpia-title" placeholder="judul asesmen"><input class="form-control" id="dpia-types" placeholder="jenis data, mis. NIK, biometrik"><select class="form-control mkt-filter" id="dpia-risk"><option value="low">low</option><option value="medium" selected>medium</option><option value="high">high</option></select><button class="btn btn-primary btn-sm" id="dpia-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="dpia-reload" type="button">Muat ulang</button></div><div class="text-danger" id="dpia-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Data</th><th>Risiko</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="dpia-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Insiden kebocoran (72 jam)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="breach-title" placeholder="judul insiden"><input class="form-control mkt-filter" id="breach-date" placeholder="diketahui YYYY-MM-DD"><input class="form-control mkt-filter" id="breach-affected" placeholder="terdampak" inputmode="numeric"><button class="btn btn-primary btn-sm" id="breach-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="breach-reload" type="button">Muat ulang</button></div><div class="text-danger" id="breach-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Diketahui</th><th>Terdampak</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="breach-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Vendor &amp; DPA</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="vendor-name" placeholder="nama vendor"><input class="form-control" id="vendor-service" placeholder="layanan, mis. cloud storage"><button class="btn btn-primary btn-sm" id="vendor-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="vendor-reload" type="button">Muat ulang</button></div><div class="text-danger" id="vendor-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Vendor</th><th>Layanan</th><th>DPA</th><th>Aksi</th></tr></thead><tbody id="vendor-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Insiden</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="inc-title" placeholder="judul insiden"><select class="form-control mkt-filter" id="inc-sev"><option value="low">low</option><option value="medium" selected>medium</option><option value="high">high</option><option value="critical">critical</option></select><button class="btn btn-primary btn-sm" id="inc-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="inc-reload" type="button">Muat ulang</button></div><div class="text-danger" id="inc-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Severitas</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="inc-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">CAPA</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="capa-title" placeholder="judul tindakan"><input class="form-control" id="capa-cause" placeholder="akar masalah"><input class="form-control" id="capa-action" placeholder="tindakan korektif"><button class="btn btn-primary btn-sm" id="capa-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="capa-reload" type="button">Muat ulang</button></div><div class="text-danger" id="capa-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Sebab</th><th>Tindakan</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="capa-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Perubahan regulasi</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="reg-title" placeholder="judul peraturan"><input class="form-control" id="reg-source" placeholder="sumber, mis. lembaran negara"><input class="form-control mkt-filter" id="reg-effective" placeholder="berlaku YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="reg-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="reg-reload" type="button">Muat ulang</button></div><div class="text-danger" id="reg-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Sumber</th><th>Berlaku</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="reg-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pelatihan kepatuhan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="trn-title" placeholder="judul pelatihan"><input class="form-control" id="trn-who" placeholder="peserta"><input class="form-control mkt-filter" id="trn-due" placeholder="tenggat YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="trn-add" type="button">Tugaskan</button><button class="btn btn-secondary btn-sm" id="trn-reload" type="button">Muat ulang</button></div><div class="text-danger" id="trn-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Peserta</th><th>Tenggat</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="trn-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Bukti penyelesaian pelatihan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="tev-trn" placeholder="ID pelatihan" inputmode="numeric"><input class="form-control" id="tev-note" placeholder="catatan bukti, mis. sertifikat No. 12"><button class="btn btn-primary btn-sm" id="tev-add" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="tev-reload" type="button">Muat</button></div><div class="text-danger" id="tev-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Catatan</th><th>Aksi</th></tr></thead><tbody id="tev-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Register risiko</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="risk-title" placeholder="judul risiko"><input class="form-control mkt-filter" id="risk-cat" placeholder="kategori"><select class="form-control mkt-filter" id="risk-like"><option value="1">K1</option><option value="2">K2</option><option value="3" selected>K3</option><option value="4">K4</option><option value="5">K5</option></select><select class="form-control mkt-filter" id="risk-imp"><option value="1">D1</option><option value="2">D2</option><option value="3" selected>D3</option><option value="4">D4</option><option value="5">D5</option></select><input class="form-control" id="risk-owner" placeholder="pemilik risiko"><button class="btn btn-primary btn-sm" id="risk-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="risk-reload" type="button">Muat ulang</button></div><div class="text-danger" id="risk-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Skor</th><th>Pemilik</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="risk-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Peta panas risiko</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="risk-hm-reload" type="button">Muat ulang</button><span class="text-muted" id="risk-hm-sum"></span></div><div class="text-danger" id="risk-hm-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>K\\D</th><th>1</th><th>2</th><th>3</th><th>4</th><th>5</th></tr></thead><tbody id="risk-hm"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Hak subjek data (DSAR)</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="dsar-subject" placeholder="subjek, mis. NIK/email"><select class="form-control mkt-filter" id="dsar-kind"><option value="access">access</option><option value="deletion">deletion</option><option value="rectification">rectification</option></select><input class="form-control" id="dsar-detail" placeholder="rincian permintaan"><input class="form-control mkt-filter" id="dsar-due" placeholder="tenggat YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="dsar-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="dsar-reload" type="button">Muat ulang</button></div><div class="text-danger" id="dsar-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Subjek</th><th>Jenis</th><th>Tenggat</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="dsar-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Deklarasi etik &amp; independensi</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ethics-who" placeholder="pendeklarasi"><input class="form-control mkt-filter" id="ethics-year" placeholder="tahun, mis. 2026"><input class="form-control" id="ethics-stmt" placeholder="pernyataan independensi"><button class="btn btn-primary btn-sm" id="ethics-add" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="ethics-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ethics-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Pendeklarasi</th><th>Tahun</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="ethics-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Register gratifikasi</h2></div><div class="card-body"><div class="mkt-row"><select class="form-control mkt-filter" id="gift-dir"><option value="received">received</option><option value="given">given</option></select><input class="form-control" id="gift-desc" placeholder="uraian hadiah/jamuan"><input class="form-control" id="gift-who" placeholder="pihak terkait"><input class="form-control mkt-filter" id="gift-at" placeholder="tanggal YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="gift-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="gift-reload" type="button">Muat ulang</button></div><div class="text-danger" id="gift-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Arah</th><th>Uraian</th><th>Tanggal</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="gift-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Temuan audit</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="af-title" placeholder="judul temuan"><input class="form-control mkt-filter" id="af-area" placeholder="area"><select class="form-control mkt-filter" id="af-sev"><option value="low">low</option><option value="medium" selected>medium</option><option value="high">high</option><option value="critical">critical</option></select><button class="btn btn-primary btn-sm" id="af-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="af-reload" type="button">Muat ulang</button></div><div class="text-danger" id="af-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Area</th><th>Severitas</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="af-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Kanal whistleblowing</h2></div><div class="card-body"><div class="mkt-row"><select class="form-control mkt-filter" id="wb-channel"><option value="web">web</option><option value="phone">phone</option><option value="box">box</option><option value="email">email</option></select><input class="form-control" id="wb-cat" placeholder="kategori"><input class="form-control" id="wb-sum" placeholder="ringkasan laporan"><input class="form-control mkt-filter" id="wb-contact" placeholder="kontak (kosong = anonim)"><button class="btn btn-primary btn-sm" id="wb-add" type="button">Catat</button><button class="btn btn-secondary btn-sm" id="wb-reload" type="button">Muat ulang</button></div><div class="text-danger" id="wb-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kanal</th><th>Ringkasan</th><th>Pelapor</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="wb-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pengecualian / waiver</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="wv-title" placeholder="judul pengecualian"><input class="form-control" id="wv-just" placeholder="justifikasi"><input class="form-control mkt-filter" id="wv-by" placeholder="pemohon"><input class="form-control mkt-filter" id="wv-exp" placeholder="berlaku s.d. YYYY-MM-DD"><button class="btn btn-primary btn-sm" id="wv-add" type="button">Ajukan</button><button class="btn btn-secondary btn-sm" id="wv-reload" type="button">Muat ulang</button></div><div class="text-danger" id="wv-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Pemohon</th><th>Berlaku s.d.</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="wv-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pustaka kebijakan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="pol-title" placeholder="judul kebijakan"><input class="form-control mkt-filter" id="pol-ver" placeholder="versi, mis. 1.0"><input class="form-control" id="pol-body" placeholder="isi kebijakan"><button class="btn btn-primary btn-sm" id="pol-add" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="pol-reload" type="button">Muat ulang</button></div><div class="text-danger" id="pol-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Versi</th><th>Status</th><th>Ack</th><th>Aksi</th></tr></thead><tbody id="pol-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Pustaka kontrol</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="ctl-code" placeholder="kode, mis. PDP-01"><input class="form-control" id="ctl-title" placeholder="judul kontrol"><input class="form-control mkt-filter" id="ctl-area" placeholder="area"><select class="form-control mkt-filter" id="ctl-freq"><option value="daily">daily</option><option value="weekly">weekly</option><option value="monthly">monthly</option><option value="quarterly" selected>quarterly</option><option value="annual">annual</option></select><button class="btn btn-primary btn-sm" id="ctl-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="ctl-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ctl-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kode</th><th>Judul</th><th>Frekuensi</th><th>Aktif</th><th>Aksi</th></tr></thead><tbody id="ctl-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Jadwal uji kontrol</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="ctdue-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ctdue-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Kode</th><th>Terakhir diuji</th><th>Jatuh tempo</th><th>Status</th></tr></thead><tbody id="ctdue-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Log uji kontrol</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="ctest-ctl" placeholder="ID kontrol" inputmode="numeric"><select class="form-control mkt-filter" id="ctest-res"><option value="pass">pass</option><option value="fail">fail</option><option value="partial">partial</option></select><input class="form-control" id="ctest-note" placeholder="catatan uji"><button class="btn btn-primary btn-sm" id="ctest-add" type="button">Catat uji</button><button class="btn btn-secondary btn-sm" id="ctest-reload" type="button">Muat</button></div><div class="text-danger" id="ctest-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Tanggal</th><th>Hasil</th><th>Catatan</th><th>Aksi</th></tr></thead><tbody id="ctest-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Rencana audit</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="aplan-title" placeholder="judul rencana"><input class="form-control mkt-filter" id="aplan-period" placeholder="periode, mis. 2026-Q1"><input class="form-control" id="aplan-scope" placeholder="ruang lingkup"><button class="btn btn-primary btn-sm" id="aplan-add" type="button">Buat</button><button class="btn btn-secondary btn-sm" id="aplan-reload" type="button">Muat ulang</button></div><div class="text-danger" id="aplan-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Periode</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="aplan-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Checklist kepatuhan perkara</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="mcl-matter" placeholder="ID perkara" inputmode="numeric"><input class="form-control" id="mcl-item" placeholder="butir checklist"><button class="btn btn-primary btn-sm" id="mcl-add" type="button">Tambah</button><button class="btn btn-secondary btn-sm" id="mcl-reload" type="button">Muat</button></div><div class="text-danger" id="mcl-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Butir</th><th>Selesai</th><th>Aksi</th></tr></thead><tbody id="mcl-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Scorecard kepatuhan</h2></div><div class="card-body"><div class="mkt-row"><button class="btn btn-secondary btn-sm" id="score-reload" type="button">Muat ulang</button><span class="text-muted" id="score-sum"></span></div><div class="text-danger" id="score-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>Area</th><th>Nilai (%)</th></tr></thead><tbody id="score-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Self-check kepatuhan</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control mkt-filter" id="sc-period" placeholder="periode, mis. 2026-Q1"><input class="form-control mkt-filter" id="sc-passed" placeholder="lolos" inputmode="numeric"><input class="form-control mkt-filter" id="sc-total" placeholder="total" inputmode="numeric"><input class="form-control" id="sc-note" placeholder="catatan"><button class="btn btn-primary btn-sm" id="sc-add" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="sc-reload" type="button">Muat ulang</button></div><div class="text-danger" id="sc-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Periode</th><th>Nilai (%)</th><th>Aksi</th></tr></thead><tbody id="sc-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Brankas bukti</h2></div><div class="card-body"><div class="mkt-row"><input class="form-control" id="ev-title" placeholder="judul bukti"><input class="form-control mkt-filter" id="ev-reg" placeholder="register, mis. risks"><input class="form-control mkt-filter" id="ev-rid" placeholder="ID entri" inputmode="numeric"><input class="form-control" id="ev-note" placeholder="catatan"><button class="btn btn-primary btn-sm" id="ev-add" type="button">Simpan</button><button class="btn btn-secondary btn-sm" id="ev-reload" type="button">Muat ulang</button></div><div class="text-danger" id="ev-msg"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Judul</th><th>Tertaut</th><th>Catatan</th><th>Aksi</th></tr></thead><tbody id="ev-list"></tbody></table></div></div></div></div><div id="p-pengaturan" class="hidden"><div class="card"><div class="card-header"><h2 class="card-title">Token API</h2></div><div class="card-body"><p class="text-muted">Buat token API untuk integrasi internal. Token penuh hanya ditampilkan sekali saat dibuat.</p><div class="set-row"><input class="form-control" id="tok-name" placeholder="nama token, mis. ci"><input class="form-control mkt-filter" id="tok-days" placeholder="hari, default 90" inputmode="numeric"><button class="btn btn-primary btn-sm" id="tok-btn" type="button">Buat token</button></div><div class="text-danger" id="tok-msg"></div><div class="tok-plain hidden" id="tok-plain"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Nama</th><th>Kedaluwarsa</th><th>Aksi</th></tr></thead><tbody id="tok-list"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Sesi aktif</h2></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Kedaluwarsa</th><th>Status</th><th>Aksi</th></tr></thead><tbody id="sess-list"></tbody></table></div><div class="card-body"><button class="btn btn-secondary btn-sm" id="sess-reload" type="button">Muat ulang</button> <span class="text-danger" id="sess-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Ubah kata sandi</h2></div><div class="card-body"><input class="form-control mb-2" id="pw-cur" type="password" autocomplete="current-password" placeholder="kata sandi saat ini"><input class="form-control mb-2" id="pw-new" type="password" autocomplete="new-password" placeholder="kata sandi baru (min 12 karakter)"><button class="btn btn-primary btn-sm" id="pw-btn" type="button">Ubah kata sandi</button> <span class="text-danger" id="pw-msg"></span></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Tetapkan peran (admin)</h2></div><div class="card-body"><p class="text-muted">Pilih pengguna satu organisasi lalu tetapkan peran. Membutuhkan hak Assign.</p><div class="set-row"><select class="form-control" id="role-uid"><option value="">Memuat pengguna…</option></select><select class="form-control" id="role-sel"><option value="super_admin">super_admin</option><option value="org_owner">org_owner</option><option value="office_admin">office_admin</option><option value="notary">notary</option><option value="ppat">ppat</option><option value="lawyer">lawyer</option><option value="legal_staff">legal_staff</option><option value="document_officer">document_officer</option><option value="finance_officer">finance_officer</option><option value="auditor">auditor</option><option value="client">client</option><option value="reviewer">reviewer</option></select><button class="btn btn-primary btn-sm" id="role-btn" type="button">Tetapkan</button></div><div class="text-danger" id="role-msg"></div><div class="mkt-row mt-2"><button class="btn btn-outline-secondary btn-sm" id="sod-btn" type="button">Cek SoD pengguna terpilih</button></div><div id="sod-list"></div><div class="table-responsive mt-2"><table class="table card-table"><thead><tr><th>Peran</th><th>Hak akses</th></tr></thead><tbody id="role-matrix"></tbody></table></div></div></div><div class="card mt-3"><div class="card-header"><h2 class="card-title">Undangan anggota</h2></div><div class="card-body"><p class="text-muted">Undang pengguna baru ke organisasi ini. Token hanya tampil sekali; berlaku 72 jam.</p><div class="mkt-row"><input class="form-control" id="inv-email" type="email" placeholder="email@kantor.id"><select class="form-control mkt-filter" id="inv-role"><option value="legal_staff">legal_staff</option><option value="lawyer">lawyer</option><option value="notary">notary</option><option value="ppat">ppat</option><option value="office_admin">office_admin</option><option value="finance_officer">finance_officer</option><option value="document_officer">document_officer</option><option value="auditor">auditor</option><option value="client">client</option></select><button class="btn btn-primary btn-sm" id="inv-add" type="button">Buat undangan</button><button class="btn btn-secondary btn-sm" id="inv-reload" type="button">Muat ulang</button></div><div class="text-danger" id="inv-msg"></div><div class="tok-plain hidden" id="inv-plain"></div><div class="table-responsive"><table class="table card-table"><thead><tr><th>ID</th><th>Email</th><th>Peran</th><th>Diterima</th><th>Aksi</th></tr></thead><tbody id="inv-list"></tbody></table></div></div></div></div><div class="mt-3"><a href="/login" class="btn btn-outline-secondary btn-sm logout">Ganti akun</a></div></div></div></div>
"#;
    html_ok(app_shell("Dasbor", body))
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
        .route("/api/v1/tasks/{tid}/assign", post(task_assign))
        .route("/api/v1/tasks/due", get(task_due_list))
        .route("/api/v1/tasks/dispatch", get(dispatch_board))
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
        .route(
            "/api/v1/reports/ocr-accuracy.csv",
            get(ocr_accuracy_csv_export),
        )
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
        .route("/api/v1/reports/approvals.csv", get(approvals_csv_export))
        .route(
            "/api/v1/reports/deadline-compliance.csv",
            get(deadline_compliance_csv),
        )
        .route(
            "/api/v1/reports/compliance-posture.csv",
            get(compliance_posture_csv),
        )
        .route("/api/v1/reports/sla-breaches.csv", get(sla_breach_csv))
        .route("/api/v1/reports/executive-summary", get(executive_summary))
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
            "/api/v1/compliance/vendors",
            post(vendor_create).get(vendor_list),
        )
        .route("/api/v1/compliance/vendors/{id}/dpa", post(vendor_set_dpa))
        .route("/api/v1/compliance/vendors/{id}", delete(vendor_delete))
        .route(
            "/api/v1/compliance/incidents",
            post(incident_create).get(incident_list),
        )
        .route(
            "/api/v1/compliance/incidents/{id}/status",
            post(incident_set_status),
        )
        .route("/api/v1/compliance/incidents/{id}", delete(incident_delete))
        .route("/api/v1/compliance/capa", post(capa_create).get(capa_list))
        .route("/api/v1/compliance/capa/{id}/status", post(capa_set_status))
        .route("/api/v1/compliance/capa/{id}", delete(capa_delete))
        .route(
            "/api/v1/compliance/regchanges",
            post(regchange_create).get(regchange_list),
        )
        .route(
            "/api/v1/compliance/regchanges/{id}/status",
            post(regchange_set_status),
        )
        .route(
            "/api/v1/compliance/regchanges/{id}",
            delete(regchange_delete),
        )
        .route(
            "/api/v1/compliance/trainings",
            post(training_create).get(training_list),
        )
        .route(
            "/api/v1/compliance/trainings/{id}/status",
            post(training_set_status),
        )
        .route("/api/v1/compliance/trainings/{id}", delete(training_delete))
        .route("/api/v1/compliance/risks", post(risk_create).get(risk_list))
        .route("/api/v1/compliance/risks/heatmap", get(risk_heatmap))
        .route(
            "/api/v1/compliance/risks/{id}/status",
            post(risk_set_status),
        )
        .route("/api/v1/compliance/risks/{id}", delete(risk_delete))
        .route("/api/v1/compliance/dsar", post(dsar_create).get(dsar_list))
        .route("/api/v1/compliance/dsar/{id}/status", post(dsar_set_status))
        .route("/api/v1/compliance/dsar/{id}", delete(dsar_delete))
        .route(
            "/api/v1/compliance/ethics",
            post(ethics_create).get(ethics_list),
        )
        .route(
            "/api/v1/compliance/ethics/{id}/status",
            post(ethics_set_status),
        )
        .route("/api/v1/compliance/ethics/{id}", delete(ethics_delete))
        .route("/api/v1/compliance/gifts", post(gift_create).get(gift_list))
        .route(
            "/api/v1/compliance/gifts/{id}/status",
            post(gift_set_status),
        )
        .route("/api/v1/compliance/gifts/{id}", delete(gift_delete))
        .route(
            "/api/v1/compliance/trainings/{id}/evidence",
            post(tevidence_add).get(tevidence_list),
        )
        .route("/api/v1/compliance/evidence/{id}", delete(tevidence_delete))
        .route(
            "/api/v1/compliance/findings",
            post(afinding_create).get(afinding_list),
        )
        .route(
            "/api/v1/compliance/findings/{id}/status",
            post(afinding_set_status),
        )
        .route("/api/v1/compliance/findings/{id}", delete(afinding_delete))
        .route(
            "/api/v1/compliance/whistle",
            post(whistle_create).get(whistle_list),
        )
        .route(
            "/api/v1/compliance/whistle/{id}/status",
            post(whistle_set_status),
        )
        .route("/api/v1/compliance/whistle/{id}", delete(whistle_delete))
        .route(
            "/api/v1/compliance/waivers",
            post(waiver_create).get(waiver_list),
        )
        .route(
            "/api/v1/compliance/waivers/{id}/status",
            post(waiver_set_status),
        )
        .route("/api/v1/compliance/waivers/{id}", delete(waiver_delete))
        .route(
            "/api/v1/compliance/policies",
            post(policy_create).get(policy_list),
        )
        .route(
            "/api/v1/compliance/policies/{id}/status",
            post(policy_set_status),
        )
        .route("/api/v1/compliance/policies/{id}", delete(policy_delete))
        .route(
            "/api/v1/compliance/policies/{id}/ack",
            post(policy_ack).get(policy_ack_list),
        )
        .route(
            "/api/v1/compliance/controls",
            post(control_create).get(control_list),
        )
        .route(
            "/api/v1/compliance/controls/{id}/active",
            post(control_set_active),
        )
        .route("/api/v1/compliance/controls/{id}", delete(control_delete))
        .route(
            "/api/v1/compliance/controls/{id}/tests",
            post(ctest_add).get(ctest_list),
        )
        .route("/api/v1/compliance/controls/due", get(controls_due))
        .route("/api/v1/compliance/ctests/{id}", delete(ctest_delete))
        .route(
            "/api/v1/compliance/aplans",
            post(aplan_create).get(aplan_list),
        )
        .route(
            "/api/v1/compliance/aplans/{id}/status",
            post(aplan_set_status),
        )
        .route("/api/v1/compliance/aplans/{id}", delete(aplan_delete))
        .route(
            "/api/v1/matters/{id}/checklist",
            post(clist_add).get(clist_list),
        )
        .route("/api/v1/checklist/{id}/toggle", post(clist_toggle))
        .route("/api/v1/checklist/{id}", delete(clist_delete))
        .route("/api/v1/compliance/scorecard", get(compliance_scorecard))
        .route(
            "/api/v1/compliance/evidence-vault",
            post(evidence_add).get(evidence_list),
        )
        .route(
            "/api/v1/compliance/evidence-vault/{id}",
            delete(evidence_delete),
        )
        .route(
            "/api/v1/reports/regulator-pack.csv",
            get(regulator_pack_export),
        )
        .route("/api/v1/protocol-books", post(pbook_create).get(pbook_list))
        .route("/api/v1/protocol-books/{id}/status", post(pbook_set_status))
        .route("/api/v1/protocol-books/{id}", delete(pbook_delete))
        .route(
            "/api/v1/protocol-books/{id}/handovers",
            post(phandover_add).get(phandover_list),
        )
        .route("/api/v1/protocol-handovers/{id}", delete(phandover_delete))
        .route("/api/v1/appointments", post(appt_create).get(appt_list))
        .route("/api/v1/appointments/queue", get(appt_queue))
        .route("/api/v1/appointments/{id}/status", post(appt_set_status))
        .route("/api/v1/appointments/{id}", delete(appt_delete))
        .route("/api/v1/prices", post(price_create).get(price_list))
        .route("/api/v1/prices/{id}/active", post(price_set_active))
        .route("/api/v1/prices/{id}", delete(price_delete))
        .route("/api/v1/quotations", post(quote_create).get(quote_list))
        .route("/api/v1/quotations/{id}/status", post(quote_set_status))
        .route("/api/v1/quotations/{id}", delete(quote_delete))
        .route("/api/v1/cashbook", post(cash_add).get(cash_list))
        .route("/api/v1/cashbook/{id}", delete(cash_delete))
        .route("/api/v1/advances", post(advance_create).get(advance_list))
        .route("/api/v1/advances/{id}/status", post(advance_set_status))
        .route("/api/v1/advances/{id}", delete(advance_delete))
        .route("/api/v1/inventory", post(stock_create).get(stock_list))
        .route("/api/v1/inventory/{id}/adjust", post(stock_adjust))
        .route("/api/v1/inventory/{id}", delete(stock_delete))
        .route(
            "/api/v1/inventory/{id}/usage",
            post(stock_use_add).get(stock_use_list),
        )
        .route("/api/v1/usage/{id}", delete(stock_use_delete))
        .route("/api/v1/archive-boxes", post(abox_create).get(abox_list))
        .route("/api/v1/archive-boxes/{id}/move", post(abox_move))
        .route("/api/v1/archive-boxes/{id}", delete(abox_delete))
        .route(
            "/api/v1/reports/office-performance",
            get(office_performance),
        )
        .route("/api/v1/reports/deed-volume", get(deed_volume))
        .route("/api/v1/down-payments", post(dp_create).get(dp_list))
        .route("/api/v1/down-payments/{id}/status", post(dp_set_status))
        .route("/api/v1/down-payments/{id}", delete(dp_delete))
        .route("/api/v1/klapper", get(klapper_list))
        .route("/api/v1/witnesses", post(witness_create).get(witness_list))
        .route("/api/v1/witnesses/{id}/active", post(witness_set_active))
        .route("/api/v1/witnesses/{id}", delete(witness_delete))
        .route("/api/v1/rechecks", post(recheck_add).get(recheck_list))
        .route("/api/v1/rechecks/{id}", delete(recheck_delete))
        .route("/api/v1/office-hours", post(hours_set).get(hours_list))
        .route("/api/v1/roster", post(roster_add).get(roster_list))
        .route("/api/v1/roster/{id}/active", post(roster_set_active))
        .route("/api/v1/roster/{id}", delete(roster_delete))
        .route("/api/v1/leaves", post(leave_request).get(leave_list))
        .route("/api/v1/leaves/{id}/status", post(leave_set_status))
        .route("/api/v1/leaves/{id}", delete(leave_delete))
        .route("/api/v1/storage-boxes", post(sbox_create).get(sbox_list))
        .route("/api/v1/storage-boxes/{id}/status", post(sbox_set_status))
        .route("/api/v1/storage-boxes/{id}", delete(sbox_delete))
        .route(
            "/api/v1/mpd-transfers",
            post(mtransfer_create).get(mtransfer_list),
        )
        .route(
            "/api/v1/mpd-transfers/{id}/status",
            post(mtransfer_set_status),
        )
        .route("/api/v1/mpd-transfers/{id}", delete(mtransfer_delete))
        .route(
            "/api/v1/documents/{id}/checklist",
            post(dclist_add).get(dclist_list),
        )
        .route("/api/v1/doc-checklist/{id}/toggle", post(dclist_toggle))
        .route("/api/v1/doc-checklist/{id}", delete(dclist_delete))
        .route(
            "/api/v1/office-incidents",
            post(oincident_create).get(oincident_list),
        )
        .route(
            "/api/v1/office-incidents/{id}/status",
            post(oincident_set_status),
        )
        .route("/api/v1/office-incidents/{id}", delete(oincident_delete))
        .route(
            "/api/v1/bank-facilities",
            post(bankf_create).get(bankf_list),
        )
        .route(
            "/api/v1/bank-facilities/{id}/active",
            post(bankf_set_active),
        )
        .route("/api/v1/bank-facilities/{id}", delete(bankf_delete))
        .route("/api/v1/feedbacks", post(feedback_add).get(feedback_list))
        .route("/api/v1/feedbacks/{id}/status", post(feedback_set_status))
        .route("/api/v1/feedbacks/{id}", delete(feedback_delete))
        .route("/api/v1/sops", post(sop_create).get(sop_list))
        .route("/api/v1/sops/{id}/status", post(sop_set_status))
        .route("/api/v1/sops/{id}", delete(sop_delete))
        .route("/api/v1/backup-checks", post(bcheck_add).get(bcheck_list))
        .route("/api/v1/backup-checks/{id}", delete(bcheck_delete))
        .route(
            "/api/v1/selfchecks",
            post(selfcheck_add).get(selfcheck_list),
        )
        .route("/api/v1/selfchecks/{id}", delete(selfcheck_delete))
        .route("/api/v1/year-seals", post(sealy_prepare).get(sealy_list))
        .route("/api/v1/year-seals/{id}/seal", post(sealy_seal))
        .route("/api/v1/year-seals/{id}", delete(sealy_delete))
        .route(
            "/api/v1/ppat/{id}/parties",
            post(pparty_add).get(pparty_list),
        )
        .route(
            "/api/v1/ppat-parties/{id}/verified",
            post(pparty_set_verified),
        )
        .route("/api/v1/ppat-parties/{id}", delete(pparty_delete))
        .route(
            "/api/v1/ppat/{id}/handovers",
            post(chandover_add).get(chandover_list),
        )
        .route(
            "/api/v1/cert-handovers/{id}/status",
            post(chandover_set_status),
        )
        .route("/api/v1/cert-handovers/{id}", delete(chandover_delete))
        .route(
            "/api/v1/ppat/{id}/charges",
            post(charge_upsert).get(charge_list),
        )
        .route("/api/v1/charges/due", get(charge_due))
        .route("/api/v1/charges/{id}", delete(charge_delete))
        .route("/api/v1/zones", post(zone_create).get(zone_list))
        .route("/api/v1/zones/{id}", delete(zone_delete))
        .route("/api/v1/worksheets/pph", get(worksheet_pph))
        .route("/api/v1/worksheets/bphtb", get(worksheet_bphtb))
        .route("/api/v1/worksheets/ppat-fee", get(worksheet_ppat_fee))
        .route("/api/v1/pbb-checks", post(pbb_add).get(pbb_list))
        .route("/api/v1/pbb-checks/{id}", delete(pbb_delete))
        .route("/api/v1/land-splits", post(lsplit_create).get(lsplit_list))
        .route("/api/v1/land-splits/{id}/status", post(lsplit_set_status))
        .route("/api/v1/land-splits/{id}", delete(lsplit_delete))
        .route("/api/v1/land-merges", post(lmerge_create).get(lmerge_list))
        .route("/api/v1/land-merges/{id}/status", post(lmerge_set_status))
        .route("/api/v1/land-merges/{id}", delete(lmerge_delete))
        .route(
            "/api/v1/land-disputes",
            post(ldispute_create).get(ldispute_list),
        )
        .route(
            "/api/v1/land-disputes/{id}/status",
            post(ldispute_set_status),
        )
        .route("/api/v1/land-disputes/{id}", delete(ldispute_delete))
        .route("/api/v1/boundary-checks", post(bndry_add).get(bndry_list))
        .route("/api/v1/boundary-checks/{id}", delete(bndry_delete))
        .route(
            "/api/v1/ppat/{id}/consents",
            post(dconsent_add).get(dconsent_list),
        )
        .route("/api/v1/consents/{id}/verify", post(dconsent_verify))
        .route("/api/v1/consents/{id}", delete(dconsent_delete))
        .route("/api/v1/land-history", post(lhist_add).get(lhist_list))
        .route("/api/v1/land-history/{id}", delete(lhist_delete))
        .route("/api/v1/unit-handovers", post(uho_add).get(uho_list))
        .route("/api/v1/unit-handovers/{id}/status", post(uho_set_status))
        .route("/api/v1/unit-handovers/{id}", delete(uho_delete))
        .route("/api/v1/reports/ppat-monthly", get(ppat_monthly_rep))
        .route("/api/v1/ppat-dashboard", get(ppat_dashboard_rep))
        .route("/api/v1/foreign-guard", get(fguard_check))
        .route(
            "/api/v1/ppat/{id}/handoffs",
            post(nhoff_add).get(nhoff_list),
        )
        .route("/api/v1/handoffs/{id}/received", post(nhoff_set_received))
        .route("/api/v1/handoffs/{id}", delete(nhoff_delete))
        .route("/api/v1/girik-intakes", post(girik_add).get(girik_list))
        .route("/api/v1/girik-intakes/{id}/status", post(girik_set_status))
        .route("/api/v1/girik-intakes/{id}", delete(girik_delete))
        .route(
            "/api/v1/ppat/{id}/lease",
            post(lease_upsert).get(lease_get).delete(lease_delete),
        )
        .route("/api/v1/dev-batches", post(dbatch_add).get(dbatch_list))
        .route("/api/v1/dev-batches/{id}/status", post(dbatch_set_status))
        .route("/api/v1/dev-batches/{id}", delete(dbatch_delete))
        .route("/api/v1/dev-units", post(dunit_add).get(dunit_list))
        .route("/api/v1/dev-units/{id}/status", post(dunit_set_status))
        .route("/api/v1/dev-units/{id}", delete(dunit_delete))
        .route("/api/v1/ppat/{id}/kpr", post(kpr_add).get(kpr_list))
        .route("/api/v1/kpr/{id}/status", post(kpr_set_status))
        .route("/api/v1/kpr/{id}", delete(kpr_delete))
        .route("/api/v1/protocol-register", post(preg_add).get(preg_list))
        .route("/api/v1/protocol-register/{id}", delete(preg_delete))
        .route(
            "/api/v1/ppat/{id}/ajb-drafts",
            post(ajbd_add).get(ajbd_list),
        )
        .route("/api/v1/ajb-drafts/{id}", delete(ajbd_delete))
        .route("/api/v1/reports/ppat-audit", get(ppat_audit_trail))
        .route("/api/v1/ppat/{id}/vault", post(clink_add).get(clink_list))
        .route("/api/v1/vault/{id}", delete(clink_delete))
        .route("/api/v1/bpn-tracks", post(bpn_add).get(bpn_list))
        .route("/api/v1/bpn-tracks/{id}/status", post(bpn_set_status))
        .route("/api/v1/bpn-tracks/{id}", delete(bpn_delete))
        .route(
            "/api/v1/reports/staff-productivity",
            get(staff_productivity),
        )
        .route("/api/v1/reports/waiting-analytics", get(waiting_analytics))
        .route("/api/v1/visitors", post(visitor_checkin).get(visitor_list))
        .route("/api/v1/visitors/{id}/checkout", post(visitor_checkout))
        .route("/api/v1/visitors/{id}", delete(visitor_delete))
        .route(
            "/api/v1/announcements",
            post(announce_create).get(announce_list),
        )
        .route(
            "/api/v1/announcements/{id}",
            post(announce_set).delete(announce_delete),
        )
        .route("/api/v1/compliance/posture", get(compliance_posture))
        .route("/api/v1/compliance/alerts", get(compliance_alerts))
        .route("/api/v1/reports/compliance.csv", get(compliance_csv_export))
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
        .route("/api/v1/ppat/{id}/readiness", get(ppat_readiness))
        .route(
            "/api/v1/delegations",
            post(delegation_set).get(delegation_list),
        )
        .route("/api/v1/roles/sod", get(sod_check))
        .route("/api/v1/orgs/invites", post(invite_create).get(invite_list))
        .route("/api/v1/orgs/invites/{id}", delete(invite_revoke))
        .route("/api/v1/integrations", get(integrations))
        .route("/api/v1/integrations/health", get(integration_health))
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
        .route("/api/v1/approvals/overdue", get(appr_overdue))
        .route("/api/v1/approvals/calendar", get(appr_calendar))
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
