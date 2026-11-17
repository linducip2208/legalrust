#!/usr/bin/env python3
"""Final push part 6: Phase 7-12 server handlers + router + state construction."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def edit(rel, old, new, count=1):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:70]}"
    p.write_text(t.replace(old, new, count), encoding="utf-8", newline="\n")

H = '''// ---------- Phase 7-12 handlers ----------
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
const DOC_TYPES: &[&str] = &["ktp", "kk", "npwp", "passport", "land_certificate", "deed", "contract", "general"];
async fn ocr_create(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<OcrJobReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    if !DOC_TYPES.contains(&b.doc_type.as_str()) { return Err(AppError::Validation("doc_type".into())); }
    s.stores.core.get_doc(&u.claims.org, b.document_id).await.map_err(|_| AppError::NotFound("document".into()))?;
    let id = s.stores.ops.ocr_create(&u.claims.org, b.document_id, &b.doc_type).await.map_err(map_store)?;
    Ok(Json(json!({"id": id, "status": "queued"})))
}
async fn ocr_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s.stores.ops.ocr_list(&u.claims.org).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|j: &OcrJobRow| json!({"id": j.id, "status": j.status, "doc_type": j.doc_type})).collect::<Vec<_>>())))
}
async fn ocr_get(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (j, fs) = s.stores.ops.ocr_get(&u.claims.org, id).await.map_err(map_store)?;
    Ok(Json(json!({"id": j.id, "status": j.status, "fields": fs.iter().map(|f| json!({"name": f.name, "value": f.value, "confidence": f.confidence})).collect::<Vec<_>>()})))
}
#[derive(Debug, Deserialize)]
struct OcrReviewReq { decision: String, note: Option<String> }
async fn ocr_review(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>, Json(b): Json<OcrReviewReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Verify)?;
    if !["approve", "reject"].contains(&b.decision.as_str()) { return Err(AppError::Validation("decision".into())); }
    s.stores.ops.ocr_review(&u.claims.org, id, &u.claims.sub, &b.decision, b.note.as_deref()).await.map_err(map_store)?;
    Ok(Json(json!({"reviewed": b.decision})))
}
async fn ocr_process(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let _ = s.stores.ops.ocr_get(&u.claims.org, id).await.map_err(map_store)?;
    Err(AppError::BlockedExternal("no OCR engine available: native libtesseract missing, cloud fallback unconfigured".into()))
}

#[derive(Debug, Deserialize, Validate)]
struct TplReq { #[validate(length(min = 2))] name: String, #[validate(length(min = 1))] body: String }
async fn tpl_create(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<TplReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let t: TemplateRow = s.stores.ops.tpl_create(&u.claims.org, &b.name, &b.body).await.map_err(map_store)?;
    Ok(Json(json!({"id": t.id, "version": t.version})))
}
async fn tpl_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s.stores.ops.tpl_list(&u.claims.org).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|t| json!({"id": t.id, "name": t.name, "version": t.version})).collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize, Validate)]
struct TplPut { #[validate(length(min = 1))] body: String }
async fn tpl_update(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>, Json(b): Json<TplPut>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Update)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let t = s.stores.ops.tpl_update(&u.claims.org, id, &b.body).await.map_err(map_store)?;
    Ok(Json(json!({"id": t.id, "version": t.version})))
}
fn substitute(body: &str, vars: &HashMap<String, String>) -> String {
    let mut out = body.to_string();
    for (k, v) in vars { out = out.replace(&format!("{{{{{k}}}}}"), v); }
    out
}
#[derive(Debug, Deserialize, Validate)]
struct DeedFromTpl { template_id: i64, matter_id: i64, #[validate(length(min = 2))] number: String, deed_type: String, variables: HashMap<String, String> }
async fn deed_from_template(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<DeedFromTpl>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let t = s.stores.ops.tpl_get(&u.claims.org, b.template_id).await.map_err(map_store)?;
    s.stores.core.get_matter(&u.claims.org, b.matter_id).await.map_err(|_| AppError::NotFound("matter".into()))?;
    let rendered = substitute(&t.body, &b.variables);
    let d: DeedRow = s.stores.ops.deed_create(&u.claims.org, b.matter_id, &b.number, &b.deed_type, &rendered).await.map_err(map_store)?;
    Ok(Json(json!({"id": d.id, "state": d.state})))
}
async fn deed_list(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s.stores.ops.deed_list(&u.claims.org).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|d| json!({"id": d.id, "number": d.number, "state": d.state})).collect::<Vec<_>>())))
}
#[derive(Debug, Deserialize)]
struct DeedTransition { to: String }
async fn deed_transition(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>, Json(b): Json<DeedTransition>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Approve)?;
    let cur = s.stores.ops.deed_list(&u.claims.org).await.map_err(map_store)?.into_iter().find(|d| d.id == id).ok_or_else(|| AppError::NotFound("deed".into()))?;
    let from = DeedState::parse(&cur.state).ok_or_else(|| AppError::Internal(anyhow::anyhow!("bad state")))?;
    let to = DeedState::parse(&b.to).ok_or_else(|| AppError::Validation("to".into()))?;
    transition(from, to).map_err(|_| AppError::Conflict("illegal transition".into()))?;
    let d = s.stores.ops.deed_set_state(&u.claims.org, id, b.to.as_str()).await.map_err(map_store)?;
    Ok(Json(json!({"id": d.id, "state": d.state})))
}

#[derive(Debug, Deserialize, Validate)]
struct ChatMsg { role: String, content: String }
#[derive(Debug, Deserialize, Validate)]
struct ChatReq { #[validate(length(min = 1, max = 200))] system: Option<String>, #[validate] messages: Vec<ChatMsg> }
async fn ai_chat(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<ChatReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    if b.messages.is_empty() || b.messages.iter().any(|m| m.content.trim().is_empty()) { return Err(AppError::Validation("messages".into())); }
    let chars: usize = b.messages.iter().map(|m| m.content.len()).sum::<usize>() + b.system.as_deref().map_or(0, str::len);
    let est = (chars / 4).max(1) as i64;
    let conv = s.stores.ops.conv_create(&u.claims.org, "chat").await.map_err(map_store)?;
    for m in &b.messages { s.stores.ops.msg_add(conv, &m.role, &m.content).await.map_err(map_store)?; }
    s.stores.ops.usage_add(&u.claims.org, est, 0).await.map_err(map_store)?;
    Err(AppError::BlockedExternal(format!("no AI provider configured; estimated input tokens: {est} (logged, zero output)")))
}
async fn ai_usage(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (i, o) = s.stores.ops.usage_sum(&u.claims.org).await.map_err(map_store)?;
    Ok(Json(json!({"input_tokens": i, "output_tokens": o})))
}

#[derive(Debug, Deserialize)]
struct RagIndexReq { document_id: i64 }
async fn rag_index(State(s): State<FullState>, headers: HeaderMap, Json(b): Json<RagIndexReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    let (d, vs) = s.stores.core.get_doc(&u.claims.org, b.document_id).await.map_err(|_| AppError::NotFound("document".into()))?;
    if !d.mime.starts_with("text/") { return Err(AppError::Validation("binary parsing not supported".into())); }
    let v = vs.last().ok_or_else(|| AppError::Validation("no content".into()))?;
    let bytes = s.stores.objects.get(&v.object_key).await?;
    let text = String::from_utf8(bytes).map_err(|_| AppError::Validation("not utf8".into()))?;
    let chunks = legalrust_rag::chunk_text(&text, 800, 100);
    s.stores.ops.rag_put(b.document_id, &chunks).await.map_err(map_store)?;
    Ok(Json(json!({"chunks": chunks.len()})))
}
#[derive(Debug, Deserialize)]
struct RagSearchQ { q: String, limit: Option<i64> }
async fn rag_search(State(s): State<FullState>, headers: HeaderMap, Query(q): Query<RagSearchQ>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    if q.q.trim().is_empty() { return Err(AppError::Validation("q".into())); }
    let hits = s.stores.ops.rag_search(&u.claims.org, &q.q, q.limit.unwrap_or(10).clamp(1, 50)).await.map_err(map_store)?;
    Ok(Json(json!(hits.iter().map(|h| json!({"doc_id": h.doc_id, "idx": h.idx, "text": h.text})).collect::<Vec<_>>())))
}

#[derive(Debug, Deserialize, Validate)]
struct InvReq { #[validate(range(min = 1))] total_minor: i64, currency: Option<String> }
async fn inv_create(State(s): State<FullState>, headers: HeaderMap, Path(mid): Path<i64>, Json(b): Json<InvReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    s.stores.core.get_matter(&u.claims.org, mid).await.map_err(|_| AppError::NotFound("matter".into()))?;
    let number = format!("INV-{}", &uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
    let v: InvoiceRow = s.stores.ops.inv_create(&u.claims.org, mid, &number, b.total_minor, b.currency.as_deref().unwrap_or("IDR")).await.map_err(map_store)?;
    Ok(Json(json!({"id": v.id, "number": v.number})))
}
async fn inv_list(State(s): State<FullState>, headers: HeaderMap, Path(mid): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let v = s.stores.ops.inv_list(&u.claims.org, mid).await.map_err(map_store)?;
    Ok(Json(json!(v.iter().map(|i| json!({"id": i.id, "number": i.number, "total": i.total})).collect::<Vec<_>>())))
}
async fn inv_get(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let (v, ps) = s.stores.ops.inv_get(&u.claims.org, id).await.map_err(map_store)?;
    let paid: i64 = ps.iter().map(|p| p.amount).sum();
    Ok(Json(json!({"id": v.id, "total": v.total, "paid": paid, "balance": v.total - paid, "payments": ps.iter().map(|p| json!({"id": p.id, "amount": p.amount})).collect::<Vec<_>>()})))
}
#[derive(Debug, Deserialize, Validate)]
struct PayReq { #[validate(range(min = 1))] amount_minor: i64, method: String }
async fn pay_add(State(s): State<FullState>, headers: HeaderMap, Path(id): Path<i64>, Json(b): Json<PayReq>) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::Create)?;
    b.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let p = s.stores.ops.pay_add(&u.claims.org, id, b.amount_minor, &b.method).await.map_err(map_store)?;
    Ok(Json(json!({"id": p.id, "amount": p.amount})))
}

async fn webhook_in(State(s): State<FullState>, headers: HeaderMap, Path(provider): Path<String>, body: axum::body::Bytes) -> Result<Json<Value>, AppError> {
    if s.stores.integration_secret.is_empty() { return Err(AppError::BlockedExternal("integration secret not configured".into())); }
    let sig = headers.get("x-signature").ok_or(AppError::Unauthorized)?.to_str().map_err(|_| AppError::Unauthorized)?;
    let ts = headers.get("x-ts").ok_or(AppError::Unauthorized)?.to_str().map_err(|_| AppError::Unauthorized)?;
    legalrust_integrations::verify_webhook(&s.stores.integration_secret, &body, ts, sig).map_err(|_| AppError::Unauthorized)?;
    audit(&s.pool, AuditEvent { tenant_id: "ext".into(), actor_id: None, action: format!("webhook.{provider}.received"), entity: "webhook".into(), entity_id: 0, ip: None }).await;
    Ok(Json(json!({"accepted": true, "provider": provider})))
}
async fn integrations(State(s): State<FullState>, headers: HeaderMap) -> Result<Json<Value>, AppError> {
    let u = auth_user(&headers, &s).await?;
    authorize(&u, Permission::View)?;
    let configured = !s.stores.integration_secret.is_empty();
    Ok(Json(json!(s.stores.integrations.iter().map(|n| json!({"name": n, "configured": configured})).collect::<Vec<_>>())))
}
// ---------- end Phase 7-12 handlers ----------
'''
p = R/"apps/server/src/lib.rs"; t = p.read_text(encoding="utf-8")
anchor = "async fn not_found() -> AppError"
assert anchor in t
t = t.replace(anchor, H + "\n" + anchor, 1)
p.write_text(t, encoding="utf-8", newline="\n")
print("P6a OK")
