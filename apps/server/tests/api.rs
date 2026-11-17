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
async fn body_text(res: axum::response::Response) -> String {
    String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
#[tokio::test]
async fn public_pages_serve_landing_login_app() {
    let a = app();
    let res = a
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
    let b = body_text(res).await;
    assert!(b.contains("LegalRust") && b.contains("/login"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let b = body_text(res).await;
    assert!(b.contains("/static/site.js") && b.contains("Masuk"));
    let res = a
        .clone()
        .oneshot(Request::builder().uri("/app").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let b = body_text(res).await;
    assert!(b.contains("/static/site.js"));
    assert!(b.contains("Dokumen"));
    assert!(b.contains("/static/site.css"));
    assert!(b.contains("Keuangan"));
    assert!(b.contains("Ringkasan") && b.contains("Akta"));
    for (u, needle) in [
        ("/static/site.css", ".hero"),
        ("/static/site.js", "lr_token"),
    ] {
        let res = a
            .clone()
            .oneshot(Request::builder().uri(u).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{u}");
        assert!(body_text(res).await.contains(needle), "{u}");
    }
}
#[tokio::test]
async fn full_flow_memory() {
    let a = app();
    // register
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/register",
            r#"{"org_name":"Acme","email":"a@x.id","name":"Ayu","password":"password1-2-3"}"#,
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
                r#"{{"org_name":"Acme","email":"{email}","name":"Ayu","password":"password1-2-3"}}"#
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
                    r#"{{"org_name":"R{i}","email":"r{i}@x.id","name":"Ray","password":"password1-2-3"}}"#
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
            r#"{"org_name":"R21","email":"r21@x.id","name":"Ray","password":"password1-2-3"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}

async fn mk_deed(app: &axum::Router, tok: &str) -> i64 {
    use tower::ServiceExt;
    let mid = mk_matter(app, tok).await;
    let res = app
        .clone()
        .oneshot(post(
            "/api/v1/deed-templates",
            r#"{"name":"AJB","body":"Pada hari ini {{hari}}."}"#,
            Some(tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = app.clone().oneshot(post("/api/v1/deeds/from-template", &format!(r#"{{"template_id":{tid},"matter_id":{mid},"number":"D-9","deed_type":"ajb","variables":{{"hari":"Senin"}}}}"#), Some(tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["id"].as_i64().unwrap()
}

#[tokio::test]
async fn esign_stub_returns_501() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "esign@x.id").await;
    let did = mk_deed(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/deeds/{did}/esign"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
    let j = body_json(res).await;
    assert_eq!(j["error"]["code"].as_str().unwrap(), "EXTERNAL_BLOCKED");
    assert!(j["error"]["message"]
        .as_str()
        .unwrap()
        .contains("PSrE provider not configured"));
}

#[tokio::test]
async fn deed_detail_returns_200() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "deedget@x.id").await;
    let did = mk_deed(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/deeds/{did}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert_eq!(j["id"].as_i64().unwrap(), did);
    assert_eq!(j["number"].as_str().unwrap(), "D-9");
    assert_eq!(j["state"].as_str().unwrap(), "draft");
    let res = a
        .clone()
        .oneshot(get("/api/v1/deeds/999999", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pay_gateway_stub_returns_501() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "paygw@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/invoices"),
            r#"{"total_minor":500000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/invoices/{iid}/pay-gateway"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
    let j = body_json(res).await;
    assert!(j["error"]["message"]
        .as_str()
        .unwrap()
        .contains("billing gateway not configured"));
}

#[tokio::test]
async fn matters_csv_export_returns_csv() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "csv@x.id").await;
    let _ = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/matters-csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ctype = res
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ctype.contains("text/csv"), "content-type was {ctype}");
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "id,number,title,status");
    let first = lines.next().expect("one matter row");
    assert!(first.contains("M-"), "row was {first}");
    assert!(first.contains("M1"), "row was {first}");
}

#[tokio::test]
async fn login_lockout_after_5_bad_attempts() {
    use tower::ServiceExt;
    let a = app();
    let email = "locked@x.id";
    let _ = reg(&a, email).await;
    let mut statuses = Vec::new();
    for _ in 0..6 {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/auth/login",
                &format!(r#"{{"email":"{email}","password":"wrongpass1234"}}"#),
                None,
            ))
            .await
            .unwrap();
        statuses.push(res.status());
    }
    assert_eq!(statuses[0], StatusCode::UNAUTHORIZED);
    assert_eq!(statuses[3], StatusCode::UNAUTHORIZED);
    assert_eq!(statuses[4], StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(statuses[5], StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn security_headers_present() {
    use tower::ServiceExt;
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
    let h = res.headers();
    assert_eq!(
        h.get("content-security-policy").unwrap(),
        "default-src 'self'"
    );
    assert!(h.contains_key("strict-transport-security"));
    assert!(h.contains_key("x-request-id"));
    assert_eq!(h.get("x-content-type-options").unwrap(), "nosniff");
}

#[tokio::test]
async fn ocr_supported_lists_doc_types() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "supported@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/ocr/supported", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    let arr = j.as_array().unwrap();
    assert!(arr.iter().any(|v| v.as_str() == Some("ktp")));
    assert!(arr.iter().any(|v| v.as_str() == Some("general")));
    // unauthenticated access is rejected (auth required)
    let res = a
        .oneshot(
            Request::builder()
                .uri("/api/v1/ocr/supported")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_route_is_404() {
    use tower::ServiceExt;
    let res = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/nope-missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn tokens_password_roles_notifications() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "sec@x.id").await;
    // api token
    let res = a
        .clone()
        .oneshot(post("/api/v1/auth/tokens", r#"{"name":"ci"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    let plain = j["token"].as_str().unwrap().to_string();
    let tid = j["id"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(get("/api/v1/orgs/me", &plain))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // revoke -> dead
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/auth/tokens/{tid}"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/orgs/me", &plain))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // password change
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/password",
            r#"{"current_password":"password1-2-3","new_password":"password2-2-3"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/login",
            r#"{"email":"sec@x.id","password":"password2-2-3"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // roles: assign lawyer to self (org_owner assigning)
    let me = body_json(
        a.clone()
            .oneshot(get("/api/v1/orgs/me", &tok))
            .await
            .unwrap(),
    )
    .await;
    let uid = me["user_id"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/users/{uid}/roles"),
            r#"{"role":"lawyer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["roles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == "lawyer"));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/users/{uid}/roles"),
            r#"{"role":"nope"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // notifications from matter creation
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications", &tok))
        .await
        .unwrap();
    let notifs = body_json(res).await;
    assert!(notifs.as_array().unwrap().is_empty());
    let _ = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications", &tok))
        .await
        .unwrap();
    let notifs = body_json(res).await;
    assert_eq!(notifs.as_array().unwrap().len(), 1);
    let nid = notifs[0]["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/notifications/{nid}/read"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn shares_reports_audit_jobs_trust_sse() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ops@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    // document + share link
    let content = b"public report content for sharing";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"r.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
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
            &format!("/api/v1/documents/{did}/shares"),
            r#"{"hours":24}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let share_tok = body_json(res).await["token"].as_str().unwrap().to_string();
    // public access without auth
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/shared/{share_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/shared/{share_tok}/content"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(&body[..], content);
    // reports + audit
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/matters.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=matter", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
    // jobs
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/jobs",
            r#"{"kind":"notify.digest"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a.clone().oneshot(get("/api/v1/jobs", &tok)).await.unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // trust ledger
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"Trust Client","kind":"individual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/trust/accounts",
            &format!(r#"{{"client_id":{cid}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let aid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/trust/accounts/{aid}/entries"),
            r#"{"kind":"credit","amount_minor":1000000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/trust/accounts/{aid}/entries"),
            r#"{"kind":"debit","amount_minor":400000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/trust/accounts/{aid}/entries"),
            r#"{"kind":"debit","amount_minor":700000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/trust/accounts/{aid}"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["balance"].as_i64().unwrap(), 600000);
    // SSE first event
    let res = a
        .clone()
        .oneshot(get("/api/v1/events", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()["content-type"], "text/event-stream");
}

fn pub_post(uri: &str, json: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(json.to_string()))
        .unwrap()
}
fn pub_get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}
fn put_auth(uri: &str, json: &str, tok: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("PUT")
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {tok}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .body(Body::from(json.to_string()))
        .unwrap()
}
fn del_auth(uri: &str, tok: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("DELETE")
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
async fn mkt_public_contact_ok() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/contact",
            r#"{"name":"Budi","email":"budi@x.id","subject":"Harga","message":"Minta penawaran paket notaris"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j["id"].as_i64().unwrap() > 0);
    assert!(j["lead_id"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn mkt_public_contact_invalid_email_422() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/contact",
            r#"{"name":"Budi","email":"not-an-email","subject":"Hi","message":"hello world"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn mkt_newsletter_subscribe_then_dup_409() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/subscribe",
            r#"{"email":"news@x.id"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/subscribe",
            r#"{"email":"news@x.id"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn mkt_public_demo_ok() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/demo-requests",
            r#"{"name":"Sari","email":"sari@x.id","company":"PT Maju","preferred_at":"2026-12-01"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j["id"].as_i64().unwrap() > 0);
    assert!(j["lead_id"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn mkt_public_sales_ok() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/sales-inquiries",
            r#"{"name":"Tono","email":"tono@x.id","company":"PT Mundur","message":"Butuh integrasi API dokumen"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "new");
}

#[tokio::test]
async fn mkt_public_pricing_and_faq_lists() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mktcms@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/pricing",
            r#"{"code":"pro","name":"Pro","monthly_minor":99000,"annual_minor":990000,"features":["akta","ocr"],"active":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/faqs",
            r#"{"slug":"harga","question":"Berapa harga?","answer":"Mulai gratis.","sort":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/pricing"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j.as_array().unwrap().iter().any(|p| p["code"] == "pro"));
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/faqs"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j.as_array().unwrap().iter().any(|f| f["slug"] == "harga"));
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/features"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn mkt_admin_unauthorized_401() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/admin/leads"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mkt_admin_lead_pipeline_ok() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mktpipe@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/leads",
            r#"{"name":"Prospek","email":"p@x.id","company":"PT Calon","source":"manual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let lid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/admin/leads/{lid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "new");
    let res = a
        .clone()
        .oneshot(put_auth(
            &format!("/api/v1/admin/leads/{lid}"),
            r#"{"status":"contacted"}"#,
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/admin/leads?status=contacted&limit=10", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["id"].as_i64() == Some(lid)));
}

#[tokio::test]
async fn mkt_admin_lead_illegal_transition_409() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mktbad@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/leads",
            r#"{"name":"Skip","email":"skip@x.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let lid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(put_auth(
            &format!("/api/v1/admin/leads/{lid}"),
            r#"{"status":"qualified"}"#,
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn mkt_admin_assign_note_followup_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mktflow@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/leads",
            r#"{"name":"Follow","email":"f@x.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let lid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/admin/leads/{lid}/assign"),
            r#"{"assignee":"staff-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["assigned_to"].as_str().unwrap(),
        "staff-1"
    );
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/admin/leads/{lid}/notes"),
            r#"{"note":"called twice"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/admin/leads/{lid}/follow-ups"),
            r#"{"due_at":"2026-12-02 10:00:00","note":"call back"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/admin/leads/{lid}/follow-ups"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn mkt_admin_delete_soft_parks_lost() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mktdel@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/leads",
            r#"{"name":"Gone","email":"gone@x.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let lid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/admin/leads/{lid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "lost");
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/admin/leads/{lid}"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "lost");
}

#[tokio::test]
async fn mkt_public_article_404_and_published_flow() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/articles/nope-missing"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let tok = reg(&a, "mktart@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/articles",
            r##"{"slug":"panduan-akta","title":"Panduan Akta","body_md":"# Isi","published":true}"##,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/admin/articles",
            r#"{"slug":"draft-x","title":"Draft","body_md":"wip"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/articles/panduan-akta"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["title"].as_str().unwrap(),
        "Panduan Akta"
    );
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/articles?published=1"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["slug"] == "panduan-akta"));
}

#[tokio::test]
async fn mkt_admin_contacts_demos_lists() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/contact",
            r#"{"name":"List","email":"list@x.id","subject":"Tanya","message":"apakah ada demo produk?"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/demo-requests",
            r#"{"name":"List","email":"list@x.id","company":"PT List"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tok = reg(&a, "mktlist@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/admin/contacts", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get("/api/v1/admin/demo-requests", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn trust_transfer_json_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "trx@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"C1","kind":"individual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let c1 = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"C2","kind":"individual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let c2 = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/trust/accounts",
            &format!(r#"{{"client_id":{c1}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let aid1 = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/trust/accounts",
            &format!(r#"{{"client_id":{c2}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let aid2 = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/trust/accounts/{aid1}/entries"),
            r#"{"kind":"credit","amount_minor":1000000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/trust/transfer",
            &format!(r#"{{"from":{aid1},"to":{aid2},"amount_minor":400000}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/trust/accounts/{aid1}"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["balance"].as_i64().unwrap(), 600000);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/trust/accounts/{aid2}"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["balance"].as_i64().unwrap(), 400000);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/trust/transfer",
            &format!(r#"{{"from":{aid1},"to":{aid2},"amount_minor":700000}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

// ----- public marketing website (page_shell routes) -----
async fn page_get(a: &axum::Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
    use tower::ServiceExt;
    let res = a.clone().oneshot(pub_get(uri)).await.unwrap();
    let st = res.status();
    let h = res.headers().clone();
    let t = body_text(res).await;
    (st, h, t)
}

#[tokio::test]
async fn pub_site_core_routes_200_with_headlines() {
    let a = app();
    for (uri, needle) in [
        ("/fitur", "Fitur Lengkap LegalRust"),
        ("/fitur/manajemen-perkara", "Manajemen Perkara Terintegrasi"),
        ("/fitur/manajemen-dokumen", "Manajemen Dokumen Berversi"),
        ("/fitur/ocr", "OCR dan Verifikasi Dokumen"),
        ("/fitur/ai-legal-assistant", "Asisten Legal AI"),
        ("/fitur/generasi-akta", "Generasi Akta dari Template"),
        ("/fitur/tanda-tangan", "Tanda Tangan Digital"),
        ("/solusi", "Solusi untuk Setiap Praktik Hukum"),
        ("/solusi/notaris", "Software Notaris"),
        ("/solusi/ppat", "Software PPAT"),
        ("/solusi/firma-hukum", "Firma Hukum"),
        ("/solusi/legal-corporate", "Legal Corporate"),
        ("/tentang", "Tentang LegalRust"),
        ("/kontak", "Hubungi Kami"),
        ("/demo", "Minta Demo"),
        ("/artikel", "Artikel dan Panduan"),
        ("/kebijakan-privasi", "Kebijakan Privasi"),
        ("/syarat-ketentuan", "Syarat dan Ketentuan"),
    ] {
        let (st, h, t) = page_get(&a, uri).await;
        assert_eq!(st, StatusCode::OK, "{uri}");
        assert_eq!(
            h.get("content-type").unwrap(),
            "text/html; charset=utf-8",
            "{uri}"
        );
        assert!(t.contains(needle), "{uri} missing {needle}");
        assert!(
            t.contains("https://legalrust.test"),
            "{uri} missing canonical"
        );
        assert!(t.contains("/harga"), "{uri} missing nav");
    }
}

#[tokio::test]
async fn pub_site_fitur_ocr_badge_and_endpoints() {
    let (st, _, t) = page_get(&app(), "/fitur/ocr").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("BUTUH KONFIGURASI"));
    assert!(t.contains("/api/v1/ocr/jobs"));
    assert!(t.contains("501"));
}

#[tokio::test]
async fn pub_site_fitur_ai_badge() {
    let (st, _, t) = page_get(&app(), "/fitur/ai-legal-assistant").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("BUTUH KONFIGURASI"));
    assert!(t.contains("/api/v1/ai/chat"));
}

#[tokio::test]
async fn pub_site_fitur_ttd_badge() {
    let (st, _, t) = page_get(&app(), "/fitur/tanda-tangan").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("BUTUH KONFIGURASI"));
    assert!(t.contains("/api/v1/deeds/{id}/esign"));
}

#[tokio::test]
async fn pub_site_fitur_akta_limited() {
    let (st, _, t) = page_get(&app(), "/fitur/generasi-akta").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("TERSEDIA TERBATAS"));
    assert!(t.contains("/api/v1/deeds/from-template"));
}

#[tokio::test]
async fn pub_site_harga_empty_db_fallback() {
    let (st, _, t) = page_get(&app(), "/harga").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("Harga LegalRust"));
    assert!(t.contains("Free Trial"));
    assert!(t.contains("Enterprise"));
}

#[tokio::test]
async fn pub_site_harga_renders_plans() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "harga@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/pricing",
            r#"{"code":"pro","name":"Professional","monthly_minor":99000,"annual_minor":990000,"features":["akta","ocr"],"active":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let (st, _, t) = page_get(&a, "/harga").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("Professional"));
    assert!(t.contains("Rp"));
}

#[tokio::test]
async fn pub_site_artikel_unknown_404() {
    let (st, _, t) = page_get(&app(), "/artikel/xxx-unknown").await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert!(t.contains("Tidak Ditemukan"));
}

#[tokio::test]
async fn pub_site_artikel_published_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "artpub@x.id").await;
    for (slug, title, md, pubd) in [
        ("panduan-web", "Panduan Web", "# Isi web", true),
        ("draf-web", "Draf Web", "wip", false),
    ] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/admin/articles",
                &format!(
                    r#"{{"slug":"{slug}","title":"{title}","body_md":"{md}","published":{pubd}}}"#
                ),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let (st, _, t) = page_get(&a, "/artikel").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("Panduan Web"));
    assert!(!t.contains("Draf Web"));
    let (st, _, t) = page_get(&a, "/artikel/panduan-web").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("Panduan Web"));
    let (st, _, _) = page_get(&a, "/artikel/draf-web").await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pub_site_faq_has_details() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "faqweb@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/faqs",
            r#"{"slug":"web","question":"Apakah data tersegregasi?","answer":"Ya, per organisasi.","sort":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let (st, _, t) = page_get(&a, "/faq").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("Pertanyaan Umum"));
    assert!(t.contains("<details>"));
    assert!(t.contains("Apakah data tersegregasi?"));
}

#[tokio::test]
async fn pub_site_sitemap_has_harga() {
    let (st, h, t) = page_get(&app(), "/sitemap.xml").await;
    assert_eq!(st, StatusCode::OK);
    assert!(h
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/xml"));
    assert!(t.contains("/harga"));
    assert!(t.contains("https://legalrust.test/fitur/ocr"));
}

#[tokio::test]
async fn pub_site_robots_disallows_app_and_api() {
    let (st, _, t) = page_get(&app(), "/robots.txt").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("Disallow: /app"));
    assert!(t.contains("Disallow: /api/"));
    assert!(t.contains("Sitemap: https://legalrust.test/sitemap.xml"));
}

#[tokio::test]
async fn pub_site_csp_header_on_root() {
    let (st, h, _) = page_get(&app(), "/").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        h.get("content-security-policy").unwrap(),
        "default-src 'self'"
    );
    assert!(h.contains_key("x-request-id"));
}

#[tokio::test]
async fn pub_site_no_inline_handlers_or_styles() {
    let a = app();
    for uri in ["/", "/fitur/ocr", "/kontak", "/faq", "/harga"] {
        let (st, _, t) = page_get(&a, uri).await;
        assert_eq!(st, StatusCode::OK, "{uri}");
        assert!(!t.contains("onclick"), "{uri} has onclick");
        assert!(!t.contains("style=\""), "{uri} has inline style");
        assert!(!t.contains("<style"), "{uri} has style tag");
    }
}

#[tokio::test]
async fn pub_site_kontak_form_posts_urlencoded() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/public/contact")
                .method("POST")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "name=Budi&email=budi%40x.id&subject=Harga&message=minta+penawaran+paket+notaris",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["id"].as_i64().unwrap() > 0);
    let (_, _, t) = page_get(&a, "/kontak").await;
    assert!(t.contains("id=\"kontak-form\""));
    assert!(t.contains("action=\"/api/v1/public/contact\""));
}

#[tokio::test]
async fn pub_site_demo_form_posts_urlencoded() {
    use tower::ServiceExt;
    let a = app();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/public/demo-requests")
                .method("POST")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "name=Sari&email=sari%40x.id&company=PT+Maju&preferred_at=2026-12-01",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let (_, _, t) = page_get(&a, "/demo").await;
    assert!(t.contains("id=\"demo-form\""));
    assert!(t.contains("action=\"/api/v1/public/demo-requests\""));
}

#[tokio::test]
async fn landing_has_all_section_anchors() {
    let (_, _, t) = page_get(&app(), "/").await;
    for id in [
        "hero",
        "dasbor",
        "kepercayaan",
        "fitur",
        "perkara",
        "dokumen",
        "ocr",
        "akta",
        "keuangan",
        "keamanan",
        "pelaporan",
        "alur",
        "solusi",
        "status",
        "faq",
        "cta",
    ] {
        assert!(t.contains(&format!("id=\"{id}\"")), "missing section #{id}");
    }
}

#[tokio::test]
async fn landing_has_4_status_labels() {
    let (_, _, t) = page_get(&app(), "/").await;
    for s in ["TERSEDIA", "TERSEDIA TERBATAS", "BUTUH KONFIGURASI"] {
        assert!(t.contains(s), "missing badge {s}");
    }
    for s in [
        "AVAILABLE",
        "LIMITED",
        "CONFIGURATION_REQUIRED",
        "COMING NEXT",
    ] {
        assert!(t.contains(s), "missing legend {s}");
    }
}

#[tokio::test]
async fn landing_has_6_images_with_alt() {
    let (_, _, t) = page_get(&app(), "/").await;
    let imgs = t.matches("<img").count();
    let alts = t.matches("alt=\"").count();
    assert!(imgs >= 5, "only {imgs} images");
    assert!(alts >= imgs, "images without alt");
    assert!(t.contains("<video"), "hero video missing");
    assert!(t.contains("hero-loop.mp4"), "hero video src missing");
    assert!(t.contains("aria-label"), "video without accessible label");
    for src in [
        "/static/hero-office.png",
        "/static/office-interior.png",
        "/static/doc-scan.png",
        "/static/ocr-scan.png",
        "/static/signature.png",
        "/static/courthouse.png",
        "/static/security.png",
        "/static/ai-assist.png",
    ] {
        assert!(t.contains(src), "missing image {src}");
    }
    assert!(
        t.contains("poster=\"/static/hero-office.png\""),
        "hero poster missing"
    );
    assert!(t.contains("loading=\"lazy\""), "no lazy images");
}

#[tokio::test]
async fn landing_faq_details_and_roadmap() {
    let (_, _, t) = page_get(&app(), "/").await;
    assert!(t.contains("id=\"faq\""));
    assert!(t.contains("<details>"), "no faq accordions");
    assert!(t.contains("/faq"), "no /faq link");
    assert!(t.contains("28/32"), "no honest 28/32 status");
    assert!(t.contains("counsel"), "no counsel roadmap item");
    assert!(
        t.contains("production-ready"),
        "no production-ready disclaimer"
    );
    assert!(t.contains("501"), "no honest 501 note");
}

async fn mk_search_client(app: &axum::Router, tok: &str, name: &str) -> i64 {
    use tower::ServiceExt;
    let res = app
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            &format!(r#"{{"display_name":"{name}","kind":"individual"}}"#),
            Some(tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["id"].as_i64().unwrap()
}

async fn mk_search_doc(app: &axum::Router, tok: &str, name: &str) -> i64 {
    use tower::ServiceExt;
    let content = b"search fixture bytes";
    let sha = legalrust_document::sha256_hex(content);
    let res = app
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"{name}","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["id"].as_i64().unwrap()
}

#[tokio::test]
async fn search_requires_auth_401() {
    use tower::ServiceExt;
    let res = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=abcd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn search_rejects_short_query_422() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "search422@x.id").await;
    for uri in ["/api/v1/search?q=x", "/api/v1/search"] {
        let res = a.clone().oneshot(get(uri, &tok)).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY, "{uri}");
    }
}

#[tokio::test]
async fn search_empty_result_shape() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "searchempty@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=zzz-no-such-thing", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert_eq!(j["clients"].as_array().unwrap().len(), 0);
    assert_eq!(j["matters"].as_array().unwrap().len(), 0);
    assert_eq!(j["documents"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn search_finds_clients_matters_docs() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "searchhit@x.id").await;
    let cid = mk_search_client(&a, &tok, "Zxqsearch Client").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            r#"{"title":"Zxqsearch Akta Tanah"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let did = mk_search_doc(&a, &tok, "zxqsearch-kontrak.txt").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=zxqsearch", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    let cs = j["clients"].as_array().unwrap();
    let ms = j["matters"].as_array().unwrap();
    let ds = j["documents"].as_array().unwrap();
    assert!(cs.iter().any(|c| c["id"].as_i64() == Some(cid)));
    assert!(cs
        .iter()
        .any(|c| c["name"].as_str() == Some("Zxqsearch Client")));
    assert!(ms.iter().any(|m| m["id"].as_i64() == Some(mid)));
    assert!(ds.iter().any(|d| d["id"].as_i64() == Some(did)));
    // case-insensitive + oversized limit clamps instead of erroring
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=ZXQSEARCH&limit=200", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await["clients"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn search_isolates_tenants() {
    use tower::ServiceExt;
    let a = app();
    let tok_a = reg(&a, "searchisoA@x.id").await;
    let tok_b = reg(&a, "searchisoB@x.id").await;
    let cid = mk_search_client(&a, &tok_a, "Zzqiso Private Client").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=zzqiso", &tok_b))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j["clients"].as_array().unwrap().is_empty());
    assert!(j["matters"].as_array().unwrap().is_empty());
    assert!(j["documents"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=zzqiso", &tok_a))
        .await
        .unwrap();
    let j = body_json(res).await;
    assert!(j["clients"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"].as_i64() == Some(cid)));
}

#[tokio::test]
async fn app_page_has_search_box() {
    let (_, _, t) = page_get(&app(), "/app").await;
    assert!(t.contains("id=\"q\""), "missing search input");
    assert!(t.contains("id=\"qres\""), "missing results dropdown");
}

#[tokio::test]
async fn mkt_app_has_marketing_tab_and_admin_refs() {
    let a = app();
    let (_, _, t) = page_get(&a, "/app").await;
    assert!(t.contains("Marketing"), "missing Marketing nav tab");
    assert!(t.contains("id=\"p-mkt\""), "missing p-mkt section");
    assert!(t.contains("class=\"hidden\""), "p-mkt must start hidden");
    for id in [
        "mkt-status-filter",
        "mkt-leads",
        "mkt-csv",
        "mkt-demos",
        "mkt-contacts",
        "mkt-news-count",
        "mkt-sub-btn",
        "mkt-plans",
        "mkt-faqs",
        "mkt-faq-add",
        "mkt-articles",
        "mkt-art-add",
    ] {
        assert!(t.contains(&format!("id=\"{id}\"")), "missing #{id}");
    }
    let (_, _, js) = page_get(&a, "/static/site.js").await;
    for ep in [
        "/api/v1/admin/leads",
        "/api/v1/admin/demo-requests",
        "/api/v1/admin/contacts",
        "/api/v1/admin/pricing",
        "/api/v1/admin/faqs",
        "/api/v1/admin/articles",
    ] {
        assert!(js.contains(ep), "site.js missing {ep}");
    }
}

#[tokio::test]
async fn mkt_admin_lead_full_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mktfull@x.id").await;
    // create
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/leads",
            r#"{"name":"Full Flow","email":"full@x.id","company":"PT Flow","source":"manual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let lid = body_json(res).await["id"].as_i64().unwrap();
    // assign
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/admin/leads/{lid}/assign"),
            r#"{"assignee":"staff-9"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["assigned_to"].as_str().unwrap(),
        "staff-9"
    );
    // status advance new -> contacted
    let res = a
        .clone()
        .oneshot(put_auth(
            &format!("/api/v1/admin/leads/{lid}"),
            r#"{"status":"contacted"}"#,
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["status"].as_str().unwrap(),
        "contacted"
    );
    // note
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/admin/leads/{lid}/notes"),
            r#"{"note":"hubungi ulang senin"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // follow-up add + list
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/admin/leads/{lid}/follow-ups"),
            r#"{"due_at":"2026-12-02 10:00:00","note":"call back"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/admin/leads/{lid}/follow-ups"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // soft delete parks as lost
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/admin/leads/{lid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "lost");
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/admin/leads/{lid}"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "lost");
}

#[tokio::test]
async fn mkt_admin_routes_require_auth_401() {
    use tower::ServiceExt;
    let a = app();
    for (m, uri, body) in [
        ("GET", "/api/v1/admin/leads", None),
        ("GET", "/api/v1/admin/leads/1", None),
        ("PUT", "/api/v1/admin/leads/1", Some(r#"{"status":"lost"}"#)),
        ("DELETE", "/api/v1/admin/leads/1", None),
        (
            "POST",
            "/api/v1/admin/leads/1/notes",
            Some(r#"{"note":"x"}"#),
        ),
        (
            "POST",
            "/api/v1/admin/leads/1/assign",
            Some(r#"{"assignee":"s"}"#),
        ),
        ("GET", "/api/v1/admin/leads/1/follow-ups", None),
        ("GET", "/api/v1/admin/contacts", None),
        ("GET", "/api/v1/admin/demo-requests", None),
        (
            "POST",
            "/api/v1/admin/pricing",
            Some(r#"{"code":"x","name":"X","monthly_minor":1,"annual_minor":1}"#),
        ),
        (
            "POST",
            "/api/v1/admin/faqs",
            Some(r#"{"slug":"x","question":"q","answer":"a"}"#),
        ),
        (
            "POST",
            "/api/v1/admin/articles",
            Some(r#"{"slug":"x","title":"t","body_md":"b"}"#),
        ),
    ] {
        let req = Request::builder()
            .uri(uri)
            .method(m)
            .header("content-type", "application/json")
            .body(Body::from(body.unwrap_or("").to_string()))
            .unwrap();
        let res = a.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{m} {uri}");
    }
}

#[tokio::test]
async fn mkt_admin_pricing_toggle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mkttoggle@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/pricing",
            r#"{"code":"team","name":"Team","monthly_minor":199000,"annual_minor":1990000,"features":["akta"],"active":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["active"].as_bool(), Some(true));
    // toggle off with same code
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/pricing",
            r#"{"code":"team","name":"Team","monthly_minor":199000,"annual_minor":1990000,"features":["akta"],"active":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["active"].as_bool(), Some(false));
    // inactive plan hidden from public list
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/pricing"))
        .await
        .unwrap();
    assert!(!body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["code"] == "team"));
}

#[tokio::test]
async fn mkt_admin_article_publish_toggle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mktpub@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/articles",
            r##"{"slug":"toggle-art","title":"Toggle Art","body_md":"# Isi","published":false}"##,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["published"].as_bool(), Some(false));
    // draft invisible publicly
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/articles/toggle-art"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // publish with same slug
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/articles",
            r##"{"slug":"toggle-art","title":"Toggle Art","body_md":"# Isi","published":true}"##,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["published"].as_bool(), Some(true));
    let res = a
        .clone()
        .oneshot(pub_get("/api/v1/public/articles/toggle-art"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn mkt_app_page_csp_safe_and_js_wires_marketing() {
    let (_, _, t) = page_get(&app(), "/app").await;
    assert!(!t.contains("onclick"), "inline handler in /app");
    assert!(!t.contains("style=\""), "inline style in /app");
    assert!(!t.contains("<style"), "style tag in /app");
    let (_, _, js) = page_get(&app(), "/static/site.js").await;
    for needle in [
        "mktRefresh",
        "mkt-leads",
        "mkt-status-filter",
        "mkt-csv",
        "mkt-adv",
        "mkt-note-btn",
        "mkt-plan-toggle",
        "mkt-art-toggle",
        "mkt-sub-btn",
        "mkt-faq-add",
        "mkt-art-add",
        "leads.csv",
    ] {
        assert!(js.contains(needle), "site.js missing {needle}");
    }
    assert!(!js.contains("onclick"), "inline handler in site.js");
}
