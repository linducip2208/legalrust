use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
fn app() -> axum::Router {
    legalrust_server::build_router(legalrust_server::dev_state())
}
async fn body_json(res: axum::response::Response) -> serde_json::Value {
    let b = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&b).unwrap()
}
fn post(uri: &str, json: &str, tok: Option<&str>) -> Request<Body> {
    let mut r = Request::builder()
        .uri(uri)
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(json.to_string()))
        .unwrap();
    if let Some(t) = tok {
        r.headers_mut()
            .insert("authorization", format!("Bearer {t}").parse().unwrap());
    }
    r
}
fn get(uri: &str, tok: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(
            "authorization",
            format!("Bearer {tok}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .body(Body::empty())
        .unwrap()
}
#[tokio::test]
async fn healthz_ok() {
    let res = app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
#[tokio::test]
async fn full_flow_memory() {
    let a = app();
    // register
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/register",
            r#"{"org_name":"Acme","email":"a@x.id","name":"Ayu","password":"password1"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "register");
    let j = body_json(res).await;
    let tok = j["token"].as_str().unwrap().to_string();
    // me
    let res = a
        .clone()
        .oneshot(get("/api/v1/orgs/me", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // client + matter + task lifecycle
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"PT Maju","kind":"corporate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"Akta Jual Beli","client_id":{cid}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/tasks"),
            r#"{"title":"Kumpulkan KTP"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    // illegal transition rejected
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}"),
            r#"{"status":"done"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // legal transition
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}"),
            r#"{"status":"in_progress"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // document metadata + content with sha pinning
    let content = b"fake-deed-bytes";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"akta.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}/content"))
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::from(content.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // logout revokes
    let res = a
        .clone()
        .oneshot(post("/api/v1/auth/logout", "{}", Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a.oneshot(get("/api/v1/orgs/me", &tok)).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]
async fn no_token_is_401() {
    let res = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/orgs/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

async fn reg(app: &axum::Router, email: &str) -> String {
    use tower::ServiceExt;
    let res = app
        .clone()
        .oneshot(post(
            "/api/v1/auth/register",
            &format!(
                r#"{{"org_name":"Acme","email":"{email}","name":"Ayu","password":"password1"}}"#
            ),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["token"].as_str().unwrap().to_string()
}
async fn mk_matter(app: &axum::Router, tok: &str) -> i64 {
    use tower::ServiceExt;
    let res = app
        .clone()
        .oneshot(post("/api/v1/matters", r#"{"title":"M1"}"#, Some(tok)))
        .await
        .unwrap();
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
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ocr/jobs",
            &format!(r#"{{"document_id":{did},"doc_type":"ktp"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let jid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ocr/jobs/{jid}/review"),
            r#"{"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ocr/jobs/{jid}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["status"]
        .as_str()
        .unwrap()
        .contains("reviewed"));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ocr/jobs/{jid}/process"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
}
#[tokio::test]
async fn deed_flow_from_template_with_guards() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "deed@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/deed-templates",
            r#"{"name":"AJB","body":"Pada hari ini {{hari}} hadapan {{notaris}}."}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post("/api/v1/deeds/from-template", &format!(r#"{{"template_id":{tid},"matter_id":{mid},"number":"D-1","deed_type":"ajb","variables":{{"hari":"Senin","notaris":"N"}}}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    // illegal skip draft->signing
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/deeds/{did}/transition"),
            r#"{"to":"signing"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // legal step
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/deeds/{did}/transition"),
            r#"{"to":"collection"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
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
    let req = Request::builder()
        .uri(format!("/api/v1/documents/{did}/content"))
        .method("PUT")
        .header(
            "authorization",
            format!("Bearer {tok}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .body(Body::from(content.to_vec()))
        .unwrap();
    assert_eq!(
        a.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/rag/index",
            &format!(r#"{{"document_id":{did}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/rag/search?q=sertifikat", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
    // finance
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/invoices"),
            r#"{"total_minor":1000000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/invoices/{iid}/payments"),
            r#"{"amount_minor":400000,"method":"transfer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/invoices/{iid}/payments"),
            r#"{"amount_minor":700000,"method":"transfer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/invoices/{iid}"), &tok))
        .await
        .unwrap();
    let j = body_json(res).await;
    assert_eq!(j["paid"].as_i64().unwrap(), 400000);
    assert_eq!(j["balance"].as_i64().unwrap(), 600000);
}
#[tokio::test]
async fn ai_chat_blocked_with_estimate_and_usage() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ai@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ai/chat",
            r#"{"messages":[{"role":"user","content":"ringkas dokumen ini"}]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
    let res = a
        .clone()
        .oneshot(get("/api/v1/ai/usage", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["input_tokens"].as_i64().unwrap() > 0);
}
#[tokio::test]
async fn webhook_verify_and_rate_limit() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "wh@x.id").await;
    let body = br#"{"event":"signed"}"#;
    let sig = legalrust_integrations::sign_webhook(b"test-integration", body, "t1");
    let req = Request::builder()
        .uri("/api/v1/webhooks/incoming/psre")
        .method("POST")
        .header(
            "authorization",
            format!("Bearer {tok}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .header("x-signature", sig)
        .header("x-ts", "t1")
        .body(Body::from(body.to_vec()))
        .unwrap();
    assert_eq!(
        a.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );
    let bad = Request::builder()
        .uri("/api/v1/webhooks/incoming/psre")
        .method("POST")
        .header(
            "authorization",
            format!("Bearer {tok}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .header("x-signature", "bad")
        .header("x-ts", "t1")
        .body(Body::from(body.to_vec()))
        .unwrap();
    assert_eq!(
        a.clone().oneshot(bad).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    // rate limit: fresh app, 21 registers -> last one 429
    let b = app();
    for i in 0..20 {
        let res = b
            .clone()
            .oneshot(post(
                "/api/v1/auth/register",
                &format!(
                    r#"{{"org_name":"R{i}","email":"r{i}@x.id","name":"Ray","password":"password1"}}"#
                ),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = b
        .clone()
        .oneshot(post(
            "/api/v1/auth/register",
            r#"{"org_name":"R21","email":"r21@x.id","name":"Ray","password":"password1"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}
