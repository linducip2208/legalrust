#!/usr/bin/env python3
"""Final round 6: server endpoints (tokens, password, roles, notifications, shares, reports, audit, jobs, SSE)."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def edit(rel, old, new):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:70]}"
    p.write_text(t.replace(old, new, 1), encoding="utf-8", newline="\n")

HANDLERS = '''
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
async fn token_create(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<TokenReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let raw = format!("lr_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let hash = legalrust_document::sha256_hex(raw.as_bytes());
    let id = s.stores.users.create_token(&u.claims.sub, &b.name, &hash).await.map_err(map_store)?;
    Ok(Json(json!({"id": id, "token": raw})))
}
async fn token_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let v = s.stores.users.list_tokens(&u.claims.sub).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|(id, n)| json!({"id": id, "name": n})).collect::<Vec<_>>())))
}
async fn token_revoke(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<String>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    s.stores.users.revoke_token(&u.claims.sub, &id).await.map_err(map_store)?;
    Ok(Json(json!({"revoked": true})))
}

#[derive(Debug, Deserialize, Validate)]
struct PasswordReq {
    #[validate(length(min = 8))]
    current_password: String,
    #[validate(length(min = 8))]
    new_password: String,
}
async fn password_change(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<PasswordReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    if u.claims.sid == "api-token" { return Err(AppError::Forbidden); }
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let row = s.stores.users.get_user(&u.claims.sub).await.map_err(map_store)?.ok_or(AppError::Unauthorized)?;
    legalrust_auth::verify_password(&row.pw_hash, &b.current_password).map_err(|_| AppError::Unauthorized)?;
    let hash = legalrust_auth::hash_password(&b.new_password).map_err(|_| AppError::Internal(anyhow::anyhow!("hash")))?;
    s.stores.users.set_password(&u.claims.sub, &hash).await.map_err(map_store)?;
    Ok(Json(json!({"changed": true})))
}

#[derive(Debug, Deserialize)]
struct RoleReq {
    role: String,
}
async fn user_assign_role(State(s): State<FullState>, headers: HeaderMap, Path(uid): Path<String>, Json(b): Json<RoleReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Assign)?;
    let role = legalrust_domain::Role::parse(&b.role).ok_or_else(|| AppError::Validation("role".into()))?;
    s.stores.users.get_user(&uid).await.map_err(map_store)?.ok_or_else(|| AppError::NotFound("user".into()))?;
    s.stores.users.assign_role(&u.claims.org, &uid, role.as_str()).await.map_err(map_store)?;
    let roles = s.stores.users.user_roles(&uid).await.map_err(map_store)?;
    Ok(Json(json!({"user_id": uid, "roles": roles})))
}

async fn notify_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let v = s.stores.ops.notify_list(&u.claims.sub).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|n| json!({"id": n.id, "kind": n.kind, "title": n.title, "read": n.read})).collect::<Vec<_>>())))
}
async fn notify_read(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    s.stores.ops.notify_read(&u.claims.sub, id).await.map_err(map_store)?;
    Ok(Json(json!({"read": true})))
}

#[derive(Debug, Deserialize)]
struct ShareReq {
    hours: Option<i64>,
}
async fn share_create(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>, Json(b): Json<ShareReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Share)?;
    s.stores.core.get_doc(&u.claims.org, id).await.map_err(|_| AppError::NotFound("document".into()))?;
    let hours = b.hours.unwrap_or(72).clamp(1, 720);
    let token = uuid::Uuid::new_v4().to_string();
    s.stores.ops.share_create(&u.claims.org, id, &token, hours).await.map_err(map_store)?;
    Ok(Json(json!({"token": token, "url": format!("/api/v1/shared/{token}")})))
}
async fn share_list(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s.stores.ops.share_list(&u.claims.org, id).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|x| json!({"id": x.id, "token": x.token, "expires": x.expires, "revoked": x.revoked})).collect::<Vec<_>>())))
}
async fn share_revoke(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Share)?;
    s.stores.ops.share_revoke(&u.claims.org, id).await.map_err(map_store)?;
    Ok(Json(json!({"revoked": true})))
}
fn share_valid(expires: &str) -> bool {
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(expires, fmt) {
            return dt > chrono::Utc::now().naive_utc();
        }
    }
    if let Ok(epoch) = expires.parse::<i64>() {
        return epoch > chrono::Utc::now().timestamp();
    }
    false
}
async fn shared_get(State(s): State<FullState>, Path(token): Path<String>) -> Result<Json<Value>, AppError> {
    let sh = s.stores.ops.share_get(&token).await.map_err(map_store)?.ok_or_else(|| AppError::NotFound("share".into()))?;
    if sh.revoked || !share_valid(&sh.expires) { return Err(AppError::NotFound("share".into())); }
    let (d, vs) = s.stores.core.get_doc(&sh.org, sh.doc).await.map_err(|_| AppError::NotFound("document".into()))?;
    Ok(Json(json!({"name": d.name, "mime": d.mime, "size": d.size, "versions": vs.len()})))
}
async fn shared_content(State(s): State<FullState>, Path(token): Path<String>) -> Result<impl IntoResponse, AppError> {
    let sh = s.stores.ops.share_get(&token).await.map_err(map_store)?.ok_or_else(|| AppError::NotFound("share".into()))?;
    if sh.revoked || !share_valid(&sh.expires) { return Err(AppError::NotFound("share".into())); }
    let (d, vs) = s.stores.core.get_doc(&sh.org, sh.doc).await.map_err(|_| AppError::NotFound("document".into()))?;
    let v = vs.last().ok_or_else(|| AppError::NotFound("content".into()))?;
    let bytes = s.stores.objects.get(&v.object_key).await?;
    let mut h = HeaderMap::new();
    h.insert("content-type", d.mime.parse().unwrap_or(HeaderValue::from_static("application/octet-stream")));
    Ok((StatusCode::OK, h, bytes))
}

async fn report_matters(State(s): State<FullState>, headers: HeaderMap) -> Result<impl IntoResponse, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Export)?;
    let v = s.stores.core.list_matters(&u.claims.org).await.map_err(map_store)?;
    let mut rows = vec![vec!["id".into(), "number".into(), "title".into(), "status".into()]];
    for m in v { rows.push(vec![m.id.to_string(), m.number, m.title, m.status]); }
    Ok(([("content-type", "text/csv")], legalrust_reporting::to_csv(&rows)))
}

#[derive(Debug, Deserialize)]
struct AuditQ {
    entity: Option<String>,
    limit: Option<i64>,
}
async fn audit_list(State(s): State<FullState>, headers: HeaderMap, Query(q): Query<AuditQ>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    let allowed = u.claims.roles.iter().filter_map(|r| legalrust_domain::Role::parse(r)).any(|r| matches!(r, legalrust_domain::Role::SuperAdmin | legalrust_domain::Role::OrgOwner | legalrust_domain::Role::OfficeAdmin | legalrust_domain::Role::Auditor));
    if !allowed { return Err(AppError::Forbidden); }
    let v = s.stores.ops.audit_list(&u.claims.org, q.entity.as_deref(), q.limit.unwrap_or(100).clamp(1, 500)).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|a| json!({"id": a.id, "actor": a.actor, "action": a.action, "entity": a.entity, "entity_id": a.entity_id, "created": a.created})).collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize)]
struct JobReq {
    kind: String,
    payload: Option<String>,
}
async fn job_enqueue(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<JobReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    if b.kind.trim().is_empty() { return Err(AppError::Validation("kind".into())); }
    let id = s.stores.ops.job_enqueue(&u.claims.org, &b.kind, b.payload.as_deref().unwrap_or("{}")).await.map_err(map_store)?;
    let _ = s.stores.events.send(format!("job.enqueued:{id}"));
    Ok(Json(json!({"id": id, "status": "queued"})))
}
async fn job_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s.stores.ops.job_list(&u.claims.org).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|j| json!({"id": j.id, "kind": j.kind, "status": j.status, "attempts": j.attempts})).collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct TrustAcctReq {
    client_id: i64,
    currency: Option<String>,
}
async fn trust_acct_create(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<TrustAcctReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let a = s.stores.ops.trust_create_account(&u.claims.org, b.client_id, b.currency.as_deref().unwrap_or("IDR")).await.map_err(map_store)?;
    Ok(Json(json!({"id": a.id, "balance": a.balance})))
}
async fn trust_acct_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s.stores.ops.trust_list(&u.claims.org).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|a| json!({"id": a.id, "client_id": a.client_id, "balance": a.balance})).collect::<Vec<_>>())))
}
async fn trust_acct_get(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (a, es) = s.stores.ops.trust_get(&u.claims.org, id).await.map_err(map_store)?;
    Ok(Json(json!({"id": a.id, "balance": a.balance, "entries": es.iter().map(|e| json!({"id": e.id, "kind": e.kind, "amount": e.amount})).collect::<Vec<_>>()})))
}
#[derive(Debug, Deserialize, Validate)]
struct TrustEntryReq {
    matter_id: Option<i64>,
    kind: String,
    #[validate(range(min = 1))]
    amount_minor: i64,
}
async fn trust_entry_add(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>, Json(b): Json<TrustEntryReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    if !["receipt", "disbursement"].contains(&b.kind.as_str()) { return Err(AppError::Validation("kind".into())); }
    let e = s.stores.ops.trust_post(&u.claims.org, id, b.matter_id, &b.kind, b.amount_minor, "manual", id, &u.claims.sub).await.map_err(map_store)?;
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
async fn trust_transfer(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<TrustTransferReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let (a, x) = s.stores.ops.trust_transfer(&u.claims.org, b.from, b.to, b.amount_minor, b.matter_id, "transfer", b.from, &u.claims.sub).await.map_err(map_store)?;
    Ok(Json(json!({"out": a.id, "in": x.id})))
}

async fn sse_events(State(s): State<FullState>, headers: HeaderMap) -> Result<axum::response::Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, AppError> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio_stream::{wrappers::BroadcastStream, StreamExt};
    let _u = auth_user(&headers, &s).await?;
    let rx = s.stores.events.subscribe();
    let live = BroadcastStream::new(rx).filter_map(|r| r.ok()).map(|msg| Ok(Event::default().data(msg)));
    let seeded = tokio_stream::iter(vec![Ok(Event::default().data("hello"))]).chain(live);
    Ok(Sse::new(seeded).keep_alive(KeepAlive::default()))
}
// ---------- end Phase 11/12 final handlers ----------
'''
p = R/"apps/server/src/lib.rs"; t = p.read_text(encoding="utf-8")
anchor = "async fn not_found() -> AppError"
assert anchor in t
t = t.replace(anchor, HANDLERS + "\n" + anchor, 1)
p.write_text(t, encoding="utf-8", newline="\n")

# router additions
edit("apps/server/src/lib.rs",
'''        .route("/api/v1/webhooks/incoming/{provider}", post(webhook_in))
        .route("/api/v1/integrations", get(integrations))
        .fallback(not_found)''',
'''        .route("/api/v1/webhooks/incoming/{provider}", post(webhook_in))
        .route("/api/v1/integrations", get(integrations))
        .route("/api/v1/auth/tokens", post(token_create).get(token_list))
        .route("/api/v1/auth/tokens/{id}", delete(token_revoke))
        .route("/api/v1/auth/password", post(password_change))
        .route("/api/v1/users/{uid}/roles", post(user_assign_role))
        .route("/api/v1/notifications", get(notify_list))
        .route("/api/v1/notifications/{id}/read", post(notify_read))
        .route("/api/v1/documents/{id}/shares", post(share_create).get(share_list))
        .route("/api/v1/shares/{id}", delete(share_revoke))
        .route("/api/v1/shared/{token}", get(shared_get))
        .route("/api/v1/shared/{token}/content", get(shared_content))
        .route("/api/v1/reports/matters.csv", get(report_matters))
        .route("/api/v1/audit", get(audit_list))
        .route("/api/v1/jobs", post(job_enqueue).get(job_list))
        .route("/api/v1/trust/accounts", post(trust_acct_create).get(trust_acct_list))
        .route("/api/v1/trust/accounts/{id}", get(trust_acct_get))
        .route("/api/v1/trust/accounts/{id}/entries", post(trust_entry_add))
        .route("/api/v1/trust/transfer", post(trust_transfer))
        .route("/api/v1/events", get(sse_events))
        .fallback(not_found)''')

# imports: routing delete + deps handled below
print("F6a staged (imports fixed next)")
