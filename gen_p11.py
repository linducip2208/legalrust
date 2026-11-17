#!/usr/bin/env python3
"""Final push part 8: extended API tests (OCR, deeds, RAG, finance, webhooks, rate-limit, AI-blocked)."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
T = '''
async fn reg(app: &axum::Router, email: &str) -> String {
    use tower::ServiceExt;
    let res = app.clone().oneshot(post("/api/v1/auth/register", &format!(r#"{{"org_name":"Acme","email":"{email}","name":"Ayu","password":"password1"}}"#), None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["token"].as_str().unwrap().to_string()
}
async fn mk_matter(app: &axum::Router, tok: &str) -> i64 {
    use tower::ServiceExt;
    let res = app.clone().oneshot(post("/api/v1/matters", r#"{"title":"M1"}"#, Some(tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["id"].as_i64().unwrap()
}
#[tokio::test]
async fn ocr_flow_review_and_blocked_process() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ocr@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"John Doe 123";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"ktp.png","mime":"image/png","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post("/api/v1/ocr/jobs", &format!(r#"{{"document_id":{did},"doc_type":"ktp"}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let jid = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post(&format!("/api/v1/ocr/jobs/{jid}/review"), r#"{"decision":"approve"}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a.clone().oneshot(get(&format!("/api/v1/ocr/jobs/{jid}"), &tok)).await.unwrap();
    assert!(body_json(res).await["status"].as_str().unwrap().contains("reviewed"));
    let res = a.clone().oneshot(post(&format!("/api/v1/ocr/jobs/{jid}/process"), "{}", Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
}
#[tokio::test]
async fn deed_flow_from_template_with_guards() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "deed@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a.clone().oneshot(post("/api/v1/deed-templates", r#"{"name":"AJB","body":"Pada hari ini {{hari}} hadapan {{notaris}}."}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post("/api/v1/deeds/from-template", &format!(r#"{{"template_id":{tid},"matter_id":{mid},"number":"D-1","deed_type":"ajb","variables":{{"hari":"Senin","notaris":"N"}}}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    // illegal skip draft->signing
    let res = a.clone().oneshot(post(&format!("/api/v1/deeds/{did}/transition"), r#"{"to":"signing"}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // legal step
    let res = a.clone().oneshot(post(&format!("/api/v1/deeds/{did}/transition"), r#"{"to":"collection"}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
#[tokio::test]
async fn rag_index_search_and_invoice_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "rag@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"Akta jual beli tanah seluas seratus meter di Jakarta. Obligasi penjual menyerahkan sertifikat.";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"n.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    let req = Request::builder().uri(format!("/api/v1/documents/{did}/content")).method("PUT")
        .header("authorization", format!("Bearer {tok}").parse::<axum::http::HeaderValue>().unwrap())
        .body(Body::from(content.to_vec())).unwrap();
    assert_eq!(a.clone().oneshot(req).await.unwrap().status(), StatusCode::OK);
    let res = a.clone().oneshot(post("/api/v1/rag/index", &format!(r#"{{"document_id":{did}}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a.clone().oneshot(get("/api/v1/rag/search?q=sertifikat", &tok)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.as_array().unwrap().len() >= 1);
    // finance
    let res = a.clone().oneshot(post(&format!("/api/v1/matters/{mid}/invoices"), r#"{"total_minor":1000000}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iid = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post(&format!("/api/v1/invoices/{iid}/payments"), r#"{"amount_minor":400000,"method":"transfer"}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a.clone().oneshot(post(&format!("/api/v1/invoices/{iid}/payments"), r#"{"amount_minor":700000,"method":"transfer"}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a.clone().oneshot(get(&format!("/api/v1/invoices/{iid}"), &tok)).await.unwrap();
    let j = body_json(res).await;
    assert_eq!(j["paid"].as_i64().unwrap(), 400000);
    assert_eq!(j["balance"].as_i64().unwrap(), 600000);
}
#[tokio::test]
async fn ai_chat_blocked_with_estimate_and_usage() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ai@x.id").await;
    let res = a.clone().oneshot(post("/api/v1/ai/chat", r#"{"messages":[{"role":"user","content":"ringkas dokumen ini"}]}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
    let res = a.clone().oneshot(get("/api/v1/ai/usage", &tok)).await.unwrap();
    assert!(body_json(res).await["input_tokens"].as_i64().unwrap() > 0);
}
#[tokio::test]
async fn webhook_verify_and_rate_limit() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "wh@x.id").await;
    let body = br#"{"event":"signed"}"#;
    let sig = legalrust_integrations::sign_webhook(b"test-integration", body, "t1");
    let req = Request::builder().uri("/api/v1/webhooks/incoming/psre").method("POST")
        .header("authorization", format!("Bearer {tok}").parse::<axum::http::HeaderValue>().unwrap())
        .header("x-signature", sig).header("x-ts", "t1").body(Body::from(body.to_vec())).unwrap();
    assert_eq!(a.clone().oneshot(req).await.unwrap().status(), StatusCode::OK);
    let bad = Request::builder().uri("/api/v1/webhooks/incoming/psre").method("POST")
        .header("authorization", format!("Bearer {tok}").parse::<axum::http::HeaderValue>().unwrap())
        .header("x-signature", "bad").header("x-ts", "t1").body(Body::from(body.to_vec())).unwrap();
    assert_eq!(a.clone().oneshot(bad).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    // rate limit: fresh app, 21 registers -> last one 429
    let b = app();
    for i in 0..20 {
        let res = b.clone().oneshot(post("/api/v1/auth/register", &format!(r#"{{"org_name":"R{i}","email":"r{i}@x.id","name":"R","password":"password1"}}"#), None)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = b.clone().oneshot(post("/api/v1/auth/register", r#"{"org_name":"R21","email":"r21@x.id","name":"R","password":"password1"}"#, None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}
'''
p = R/"apps/server/tests/api.rs"; t = p.read_text(encoding="utf-8")
t = t + "\n" + T.strip() + "\n"
p.write_text(t, encoding="utf-8", newline="\n")
print("P8 OK")
