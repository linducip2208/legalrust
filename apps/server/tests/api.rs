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
async fn mk_invoice(app: &axum::Router, tok: &str, mid: i64, total: i64) -> i64 {
    use tower::ServiceExt;
    let res = app
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/invoices"),
            &format!(r#"{{"total_minor":{total}}}"#),
            Some(tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["id"].as_i64().unwrap()
}
#[tokio::test]
async fn ocr_process_requires_content() {
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
    // No bytes were ever uploaded for this document: honest 422, and the
    // job stays unprocessed (engine presence must not matter here).
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ocr/jobs/{jid}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["status"]
        .as_str()
        .unwrap()
        .contains("reviewed"));
}
/// Live-gated end-to-end: real PNG bytes through the native engine.
/// Hosts with tesseract installed get 200 + persisted fields; hosts
/// without it (CI) get the honest 501. Either way no text is fabricated.
#[tokio::test]
async fn ocr_process_image_live_or_blocked() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ocrlive@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content: &[u8] = include_bytes!("../static/hero-office.png");
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"kantor.png","mime":"image/png","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
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
        .oneshot(post(
            "/api/v1/ocr/jobs",
            &format!(r#"{{"document_id":{did},"doc_type":"general"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let jid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ocr/jobs/{jid}/process"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    match res.status() {
        StatusCode::NOT_IMPLEMENTED => {} // no tesseract on this host (CI): honest 501
        StatusCode::OK => {
            let v = body_json(res).await;
            assert_eq!(v["status"].as_str().unwrap(), "processed");
            let n = v["lines"].as_u64().unwrap();
            assert_eq!(v["fields"].as_array().unwrap().len() as u64, n);
            let res = a
                .clone()
                .oneshot(get(&format!("/api/v1/ocr/jobs/{jid}"), &tok))
                .await
                .unwrap();
            let g = body_json(res).await;
            assert_eq!(g["status"].as_str().unwrap(), "processed");
            assert_eq!(g["fields"].as_array().unwrap().len() as u64, n);
        }
        other => panic!("unexpected process status: {other}"),
    }
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
static AI_ENV_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn ai_env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    AI_ENV_GUARD.lock().await
}

#[tokio::test]
async fn ai_chat_blocked_with_estimate_and_usage() {
    use tower::ServiceExt;
    let _env = ai_env_lock().await;
    std::env::set_var("AI_BASE_URL", "http://127.0.0.1:1");
    std::env::remove_var("AI_API_KEY");
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
    std::env::remove_var("AI_BASE_URL");
}

#[tokio::test]
async fn ai_chat_live_against_stub() {
    use std::io::{Read, Write};
    use tower::ServiceExt;
    let _env = ai_env_lock().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ai stub");
    let addr = listener.local_addr().expect("ai stub addr");
    let stub = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().expect("ai stub accept");
        let mut buf = vec![0u8; 65536];
        let n = s.read(&mut buf).expect("stub read");
        assert!(String::from_utf8_lossy(&buf[..n]).contains("stub-model"));
        let body = br#"{"id":"chatcmpl-x","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":"ringkasan: akta jual beli"},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":7,"total_tokens":19}}"#;
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        s.write_all(head.as_bytes()).expect("stub head");
        s.write_all(body).expect("stub body");
    });
    std::env::set_var("AI_BASE_URL", format!("http://{addr}"));
    std::env::set_var("AI_MODEL", "stub-model");
    std::env::remove_var("AI_API_KEY");
    let a = app();
    let tok = reg(&a, "aistub@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ai/chat",
            r#"{"messages":[{"role":"user","content":"ringkas dokumen ini"}]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(v["reply"].as_str().unwrap().contains("ringkasan"));
    assert_eq!(v["model"].as_str().unwrap(), "stub-model");
    assert_eq!(v["output_tokens"].as_i64().unwrap(), 7);
    stub.join().expect("stub served");
    let res = a
        .clone()
        .oneshot(get("/api/v1/ai/usage", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["output_tokens"].as_i64().unwrap() > 0);
    std::env::remove_var("AI_BASE_URL");
    std::env::remove_var("AI_MODEL");
}

/// Live-gated: if a local Ollama gateway answers with the expected model,
/// one small prompt runs for real. Otherwise passes vacuously (CI-safe).
#[tokio::test]
async fn ai_chat_live_ollama_gated() {
    use tower::ServiceExt;
    let _env = ai_env_lock().await;
    if std::net::TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().expect("addr"),
        std::time::Duration::from_secs(1),
    )
    .is_err()
    {
        return;
    }
    std::env::set_var("AI_BASE_URL", "http://127.0.0.1:11434/v1");
    std::env::set_var("AI_MODEL", "qwen2.5:1.5b");
    std::env::remove_var("AI_API_KEY");
    std::env::set_var("AI_TIMEOUT_SECS", "180");
    let a = app();
    let tok = reg(&a, "aiollama@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ai/chat",
            r#"{"messages":[{"role":"user","content":"jawab hanya: 2+2"}]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    // Model may be absent even when Ollama runs: 501 then, real reply otherwise.
    match res.status() {
        StatusCode::OK => {
            assert!(!body_json(res).await["reply"]
                .as_str()
                .unwrap()
                .trim()
                .is_empty());
        }
        StatusCode::NOT_IMPLEMENTED => {}
        other => panic!("unexpected ai status: {other}"),
    }
    std::env::remove_var("AI_BASE_URL");
    std::env::remove_var("AI_MODEL");
    std::env::remove_var("AI_TIMEOUT_SECS");
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
    let _env = tripay_env_lock().await;
    for v in [
        "TRIPAY_API_KEY",
        "TRIPAY_PRIVATE_KEY",
        "TRIPAY_MERCHANT_CODE",
        "TRIPAY_BASE_URL",
    ] {
        std::env::remove_var(v);
    }
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
            r#"{"channel":"QRIS","customer_name":"Budi Santoso"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
    let j = body_json(res).await;
    assert!(j["error"]["message"]
        .as_str()
        .unwrap()
        .contains("tripay not configured"));
}

/// Tripay end-to-end against a local stub: checkout 200, PAID callback posts
/// the payment exactly once, bad signature is 401. Serialized with the 501
/// test via an async mutex because both mutate `TRIPAY_*` env vars.
static TRIPAY_ENV_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn tripay_env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    TRIPAY_ENV_GUARD.lock().await
}

#[tokio::test]
async fn tripay_gateway_and_callback_against_stub() {
    use std::io::{Read, Write};
    use tower::ServiceExt;
    let _env = tripay_env_lock().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind tripay stub");
    let addr = listener.local_addr().expect("stub addr");
    let stub = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().expect("tripay stub accept");
        let mut buf = vec![0u8; 65536];
        let n = s.read(&mut buf).expect("stub read");
        let text = String::from_utf8_lossy(&buf[..n]).into_owned();
        assert!(text.contains("QRIS"), "stub saw channel");
        assert!(text.contains("LRINV"), "stub saw merchant_ref");
        let body = br#"{"success":true,"message":"ok","data":{"reference":"T-STUB-1","amount":250000,"checkout_url":"https://tripay.co.id/checkout/T-STUB-1"}}"#;
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        s.write_all(head.as_bytes()).expect("stub head");
        s.write_all(body).expect("stub body");
    });
    std::env::set_var("TRIPAY_API_KEY", "stub-api-key");
    std::env::set_var("TRIPAY_PRIVATE_KEY", "stub-private-key");
    std::env::set_var("TRIPAY_MERCHANT_CODE", "STUBCODE");
    std::env::set_var("TRIPAY_MODE", "sandbox");
    std::env::set_var("TRIPAY_BASE_URL", format!("http://{addr}"));
    let a = app();
    let tok = reg(&a, "tripay@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/invoices"),
            r#"{"total_minor":250000}"#,
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
            r#"{"channel":"qris","customer_name":"Budi Santoso","customer_email":"budi@x.id","expired_hours":24}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["reference"].as_str().unwrap(), "T-STUB-1");
    assert!(v["checkout_url"].as_str().unwrap().contains("T-STUB-1"));
    assert_eq!(v["amount"].as_i64().unwrap(), 250000);
    let merchant_ref = v["merchant_ref"].as_str().unwrap().to_string();
    stub.join().expect("stub served");
    // PAID callback posts the payment.
    let cb = format!(
        r#"{{"reference":"T-STUB-1","merchant_ref":"{merchant_ref}","status":"PAID","amount_received":250000}}"#
    );
    let sig = {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"stub-private-key").expect("hmac key");
        mac.update(cb.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/webhooks/incoming/tripay")
                .method("POST")
                .header("x-callback-signature", sig)
                .header("content-type", "application/json")
                .body(Body::from(cb.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(v["accepted"].as_bool().unwrap());
    assert_eq!(v["amount"].as_i64().unwrap(), 250000);
    let res = a
        .clone()
        .oneshot(get("/api/v1/webhooks/deliveries?provider=tripay", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["provider"] == "tripay" && r["status"] == "accepted"));
    // Status filter trims to matching rows only.
    let res = a
        .clone()
        .oneshot(get("/api/v1/webhooks/deliveries?status=rejected", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["status"] == "rejected"));
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/invoices/{iid}"), &tok))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["paid"].as_i64().unwrap(), 250000);
    assert_eq!(v["balance"].as_i64().unwrap(), 0);
    // Repeat callback is a harmless duplicate; bad signature is 401.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/webhooks/incoming/tripay")
                .method("POST")
                .header("x-callback-signature", {
                    use hmac::{Hmac, Mac};
                    use sha2::Sha256;
                    let mut mac =
                        Hmac::<Sha256>::new_from_slice(b"stub-private-key").expect("hmac key");
                    mac.update(cb.as_bytes());
                    hex::encode(mac.finalize().into_bytes())
                })
                .header("content-type", "application/json")
                .body(Body::from(cb))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(body_json(res).await["duplicate"].as_bool().unwrap());
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/webhooks/incoming/tripay")
                .method("POST")
                .header("x-callback-signature", "deadbeef")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reference":"T-X"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    for v in [
        "TRIPAY_API_KEY",
        "TRIPAY_PRIVATE_KEY",
        "TRIPAY_MERCHANT_CODE",
        "TRIPAY_MODE",
        "TRIPAY_BASE_URL",
    ] {
        std::env::remove_var(v);
    }
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
async fn notifications_kind_filter_and_read() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "notifkind@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    // Approval create emits a kind=approval notification.
    let _ = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?kind=approval", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(!v.as_array().unwrap().is_empty());
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["kind"] == "approval"));
    let nid = v[0]["id"].as_i64().unwrap();
    // Other kinds excluded; overlong kind rejected.
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?kind=deed", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
    let long = "k".repeat(65);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/notifications?kind={long}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Mark-read flips the read flag.
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?kind=approval", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"].as_i64() == Some(nid) && n["read"] == true));
}

#[tokio::test]
async fn notification_prefs_opt_out_suppresses() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "notifpref@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    // Disable approval notes: creating an approval emits nothing for me.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/notifications/prefs",
            r#"{"kind":"approval","enabled":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _ = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?kind=approval", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
    // Suppression is audit-traced.
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=notification", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "notify.approval.suppressed"));
    // Re-enable: the next approval notifies again.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/notifications/prefs",
            r#"{"kind":"approval","enabled":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _ = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?kind=approval", &tok))
        .await
        .unwrap();
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
    // Guards: empty/overlong kind rejected.
    for body in [
        r#"{"kind":"","enabled":false}"#.to_string(),
        format!(r#"{{"kind":"{}","enabled":false}}"#, "k".repeat(65)),
    ] {
        let res = a
            .clone()
            .oneshot(post("/api/v1/notifications/prefs", &body, Some(&tok)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn notification_flood_guard_caps_per_user() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "notifflood@x.id").await;
    // 25 rapid matter creations: at most 20 notify, rest rate-limited.
    for _ in 0..25 {
        let _ = mk_matter(&a, &tok).await;
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?kind=matter", &tok))
        .await
        .unwrap();
    let n = body_json(res).await.as_array().unwrap().len();
    assert!(n <= 20, "flood guard failed: {n} matter notifs");
    assert!(n >= 15, "guard too aggressive: only {n} matter notifs");
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=notification", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "notify.matter.rate_limited"));
}

#[tokio::test]
async fn task_due_set_list_and_calendar_feed() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "taskdue@x.id").await;
    let tok_b = reg(&a, "taskdueB@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/tasks"),
            r#"{"title":"Berkas AJB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    // Bad format rejected; valid datetime sticks.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}/due"),
            r#"{"due":"tomorrow"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}/due"),
            r#"{"due":"2026-12-01 09:30:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["due"], "2026-12-01 09:30:00");
    // List carries due; due feed orders ascending and isolates tenants.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/tasks"), &tok))
        .await
        .unwrap();
    assert_eq!(
        body_json(res).await.as_array().unwrap()[0]["due"],
        "2026-12-01 09:30:00"
    );
    let res = a
        .clone()
        .oneshot(get("/api/v1/tasks/due?limit=10", &tok))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"].as_i64() == Some(tid) && t["due"] == "2026-12-01 09:30:00"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/tasks/due?limit=10", &tok_b))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
    // Unknown task 404; clearing works.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/tasks/999999999/due",
            r#"{"due":null}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}/due"),
            r#"{"due":null}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["due"].is_null());
}

#[tokio::test]
async fn notifications_unread_only_and_limit() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "notifunread@x.id").await;
    let tok_b = reg(&a, "notifunreadB@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // All seeded notifs start unread; other users see none.
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?unread_only=true", &tok))
        .await
        .unwrap();
    let v = body_json(res).await;
    let all_unread = v.as_array().unwrap();
    assert!(all_unread.len() >= 2);
    assert!(all_unread.iter().all(|n| n["read"] == false));
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?unread_only=true", &tok_b))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
    // Mark one read: unread shrinks, full list keeps it flagged.
    let nid = all_unread[0]["id"].as_i64().unwrap();
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?unread_only=true", &tok))
        .await
        .unwrap();
    let now_unread = body_json(res).await;
    assert_eq!(now_unread.as_array().unwrap().len(), all_unread.len() - 1);
    // Limit pages server-side; out-of-range clamps instead of erroring.
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?limit=1", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?limit=0", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // Bad bool rejected at the query layer (Axum: 400); missing token unauthorized.
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications?unread_only=yes", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/notifications?unread_only=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Notification audit trail: send + read events queryable per tenant.
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=notification", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let trail = body_json(res).await;
    let acts: Vec<&str> = trail
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["action"].as_str())
        .collect();
    assert!(
        acts.iter()
            .any(|x| x.starts_with("notify.") && x.ends_with(".sent")),
        "missing notify.*.sent in {acts:?}"
    );
    assert!(
        acts.contains(&"notification.read"),
        "missing notification.read in {acts:?}"
    );
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
async fn mkt_newsletter_unsubscribe_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    // Unknown email cannot be unsubscribed; bad email rejected.
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/unsubscribe",
            r#"{"email":"gone@x.id"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/unsubscribe",
            r#"{"email":"bukan-email"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Subscribe -> unsubscribe -> gone -> resubscribe works.
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/subscribe",
            r#"{"email":"cycle@x.id"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/unsubscribe",
            r#"{"email":"cycle@x.id"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["subscribed"], false);
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/unsubscribe",
            r#"{"email":"cycle@x.id"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(pub_post(
            "/api/v1/public/newsletter/subscribe",
            r#"{"email":"cycle@x.id"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
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
    // Finance audit trail: transfer + report exports queryable per entity.
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=trust", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "trust.transfer"));
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
    assert!(t.contains("TERSEDIA"));
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
    assert!(t.contains("31/32"), "no honest 31/32 status");
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
async fn feature_scores_full_catalog_transparent() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "skor@x.id").await;
    // Unauthenticated callers cannot see the catalog.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/features/scores")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Full catalog: 1000 features, honest summary math.
    let res = a
        .clone()
        .oneshot(get("/api/v1/features/scores", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert_eq!(j["summary"]["features"].as_u64().unwrap(), 1000);
    assert_eq!(j["total"].as_u64().unwrap(), 1000);
    assert_eq!(j["items"].as_array().unwrap().len(), 50);
    let by = j["summary"]["by_status"].as_object().unwrap();
    let sum: u64 = by.values().map(|v| v.as_u64().unwrap()).sum();
    assert_eq!(sum, 1000);
    assert!(j["summary"]["implemented"].as_u64().unwrap() >= 1);
    assert!(j["summary"]["implemented"].as_u64().unwrap() < 1000);
    // Domain + status filters narrow; unknown values rejected.
    let res = a
        .clone()
        .oneshot(get("/api/v1/features/scores?domain=D11", &tok))
        .await
        .unwrap();
    let j = body_json(res).await;
    assert!(!j["items"].as_array().unwrap().is_empty());
    assert!(j["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x["domain"] == "D11"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/features/scores?status=implemented", &tok))
        .await
        .unwrap();
    let j = body_json(res).await;
    assert!(j["total"].as_u64().unwrap() >= 1);
    assert!(j["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x["status"] == "IMPLEMENTED"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/features/scores?q=ropa", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == "F-D11-006"));
    // Oversized page clamps to 100; bad domain/status rejected.
    let res = a
        .clone()
        .oneshot(get("/api/v1/features/scores?per_page=200", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["items"].as_array().unwrap().len(), 100);
    for uri in [
        "/api/v1/features/scores?status=bogus",
        "/api/v1/features/scores?domain=XX",
    ] {
        let res = a.clone().oneshot(get(uri, &tok)).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "uri was {uri}"
        );
    }
}

#[tokio::test]
async fn search_pagination_totals_and_types() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "searchpage@x.id").await;
    for n in ["Qpg Alpha Satu", "Qpg Alpha Dua", "Qpg Alpha Tiga"] {
        mk_search_client(&a, &tok, n).await;
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=qpg&per_page=2&page=1", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert_eq!(j["clients"].as_array().unwrap().len(), 2);
    assert_eq!(j["totals"]["clients"].as_i64().unwrap(), 3);
    assert_eq!(j["page"].as_i64().unwrap(), 1);
    assert_eq!(j["per_page"].as_i64().unwrap(), 2);
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=qpg&per_page=2&page=2", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["clients"].as_array().unwrap().len(), 1);
    // Type filter narrows groups; unknown type rejected.
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=qpg&types=clients", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(!j["clients"].as_array().unwrap().is_empty());
    assert!(j["matters"].as_array().unwrap().is_empty());
    assert!(j["documents"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=qpg&types=bogus", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rag_search_isolates_tenants() {
    use tower::ServiceExt;
    let a = app();
    let tok_a = reg(&a, "ragisoA@x.id").await;
    let tok_b = reg(&a, "ragisoB@x.id").await;
    let mid = mk_matter(&a, &tok_a).await;
    let content = b"Zzqrag rahasia isolasi tenant sertifikat unik.";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"r.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok_a))).await.unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    let req = Request::builder()
        .uri(format!("/api/v1/documents/{did}/content"))
        .method("PUT")
        .header(
            "authorization",
            format!("Bearer {tok_a}")
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
            Some(&tok_a),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Same org sees the hit with its doc id; foreign org sees nothing.
    let res = a
        .clone()
        .oneshot(get("/api/v1/rag/search?q=zzqrag", &tok_a))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hits = body_json(res).await;
    assert!(hits
        .as_array()
        .unwrap()
        .iter()
        .any(|h| h["doc_id"].as_i64() == Some(did)));
    let res = a
        .clone()
        .oneshot(get("/api/v1/rag/search?q=zzqrag", &tok_b))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.as_array().unwrap().is_empty());
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
        "mkt-unsub-btn",
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
        "/api/v1/tasks/due",
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
        "mkt-unsub-btn",
        "mkt-faq-add",
        "mkt-art-add",
        "leads.csv",
    ] {
        assert!(js.contains(needle), "site.js missing {needle}");
    }
    assert!(!js.contains("onclick"), "inline handler in site.js");
}

#[tokio::test]
async fn dash_app_has_sidebar_nav_groups() {
    let (_, _, t) = page_get(&app(), "/app").await;
    assert!(t.contains("id=\"sidebar\""), "missing sidebar");
    // One link per tab: no duplicate data-tab may share the active state.
    for s in [
        "menu-q",
        "Cari menu",
        "Dashboard",
        "Verifikasi Identitas",
        "Klien &amp; Perkara",
        "Keuangan",
        "Dokumen",
        "Akta",
        "Persetujuan",
        "Marketing",
        "Pengaturan",
        "Audit",
        "Jobs",
        "Kepatuhan",
    ] {
        assert!(t.contains(s), "missing sidebar item {s}");
    }
    let sb_start = t.find("id=\"sidebar\"").unwrap();
    let sb = &t[sb_start..t[sb_start..].find("</aside>").unwrap() + sb_start];
    for tab in [
        "ringkas",
        "ocr",
        "klien",
        "keu",
        "dok",
        "akta",
        "appr",
        "mkt",
        "pengaturan",
        "audit",
        "jobs",
        "kepatuhan",
    ] {
        let n = sb.matches(&format!("data-tab=\"{tab}\"")).count();
        assert_eq!(n, 1, "sidebar data-tab={tab} appears {n}x, want exactly 1");
    }
    for id in [
        "p-ringkas",
        "p-klien",
        "p-dok",
        "p-ocr",
        "p-akta",
        "p-keu",
        "p-mkt",
    ] {
        assert!(t.contains(&format!("id=\"{id}\"")), "missing section #{id}");
    }
    assert!(t.contains("id=\"orgsel\""), "missing org switcher");
    assert!(t.contains("id=\"bell\""), "missing bell");
    assert!(t.contains("id=\"avatar\""), "missing avatar");
    assert!(t.contains("id=\"orgname\""), "missing org name");
}

#[tokio::test]
async fn dash_app_has_kpi_labels_and_panels() {
    let (_, _, t) = page_get(&app(), "/app").await;
    for s in [
        "Klien Aktif",
        "Perkara Berjalan",
        "Tugas Terbuka",
        "Dokumen",
        "OCR Menunggu Review",
        "Akta Dalam Proses",
        "Invoice Outstanding",
        "Saldo Trust",
        "live",
    ] {
        assert!(t.contains(s), "missing KPI {s}");
    }
    for id in [
        "k1",
        "k2",
        "k3",
        "k4",
        "k5",
        "k6",
        "k7",
        "k8",
        "funnel",
        "chart",
        "cal",
        "al-q",
        "ae-q",
        "dl5",
        "ocr-recent",
        "fin-sum",
        "m-all",
        "cl",
        "task-matter",
        "task-list",
        "ocr-jobs",
        "ocr-detail",
        "fin-sum",
    ] {
        assert!(t.contains(&format!("id=\"{id}\"")), "missing #{id}");
    }
    for id in [
        "qa-client",
        "qa-matter",
        "qa-upload",
        "qa-ocr",
        "qa-deed",
        "qa-invoice",
    ] {
        assert!(
            t.contains(&format!("id=\"{id}\"")),
            "missing quick action #{id}"
        );
    }
    for id in [
        "cli-name",
        "cli-kind",
        "cli-btn",
        "mat-title",
        "mat-client",
        "mat-btn",
        "task-title",
        "task-btn",
        "task-due",
        "ocr-doc",
        "ocr-type",
        "ocr-btn",
        "ocr-acc-btn",
        "deed-tpl",
        "deed-matter",
        "deed-number",
        "deed-type",
        "deed-btn",
        "tpl-name",
        "tpl-body",
        "tpl-btn",
        "invm-matter",
        "invm-total",
        "invm-btn",
        "sh-doc",
        "sh-btn",
        "doc-version-id",
        "doc-version-btn",
        "doc-version-detail",
        "cert-doc",
        "cert-btn",
        "cert-reload",
        "cert-msg",
        "cert-list",
        "clause-name",
        "clause-cat",
        "clause-body",
        "clause-add",
        "clause-reload",
        "clause-list",
        "ppat-number",
        "ppat-kind",
        "ppat-add",
        "ppat-reload",
        "ppat-list",
    ] {
        assert!(t.contains(&format!("id=\"{id}\"")), "missing form #{id}");
    }
}

#[tokio::test]
async fn dash_flow_client_create() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dashcli@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"PT Dash","kind":"corporate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j["id"].as_i64().unwrap() > 0);
    assert_eq!(j["display_name"].as_str().unwrap(), "PT Dash");
}

#[tokio::test]
async fn dash_flow_matter_create() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dashmat@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"Klien Dash","kind":"individual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"Perkara Dash","client_id":{cid}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j["id"].as_i64().unwrap() > 0);
    assert!(j["number"].as_str().unwrap().starts_with("M-"));
}

#[tokio::test]
async fn dash_flow_task_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dashtask@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/tasks"),
            r#"{"title":"Siapkan berkas"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/tasks"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // open -> done is illegal
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
    // open -> in_progress -> done is legal
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
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}"),
            r#"{"status":"done"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "done");
}

#[tokio::test]
async fn dash_flow_ocr_job_create() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dashocr@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"NIK 12345";
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
    let j = body_json(res).await;
    let jid = j["id"].as_i64().unwrap();
    assert_eq!(j["status"].as_str().unwrap(), "queued");
    let res = a
        .clone()
        .oneshot(get("/api/v1/ocr/jobs", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|j| j["id"].as_i64() == Some(jid)));
}

#[tokio::test]
async fn dash_flow_deed_from_template() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dashdeed@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/deed-templates",
            r#"{"name":"AJB Dash","body":"Pada hari ini {{hari}}."}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post("/api/v1/deeds/from-template", &format!(r#"{{"template_id":{tid},"matter_id":{mid},"number":"D-DASH-1","deed_type":"ajb","variables":{{"hari":"Senin"}}}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    let did = j["id"].as_i64().unwrap();
    assert_eq!(j["state"].as_str().unwrap(), "draft");
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
async fn ppat_register_crud_and_guards() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ppat@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // RBAC: auditor cannot create.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"1/2026","deed_kind":"ajb","seller":"A","buyer":"B","object":"SHM 1"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Validation: bad kind rejected at the store layer.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"1/2026","deed_kind":"gadai"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"1/2026","deed_kind":"ajb","seller":"A","buyer":"B","object":"SHM 1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    // Duplicate number rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"1/2026","deed_kind":"hibah"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // List shows draft; skip-ahead refused; walk the chain.
    let res = a.clone().oneshot(get("/api/v1/ppat", &tok)).await.unwrap();
    let v = body_json(res).await;
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(pid) && r["status"] == "draft"));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/status"),
            r#"{"status":"registered"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["signed", "registered"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/ppat/{pid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "transition to {st}");
    }
    // Auditor cannot move status either (no Update).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/status"),
            r#"{"status":"draft"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Checklist: full entry complete; minimal entry lists gaps; unknown 404.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/checklist"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert_eq!(j["complete"], true);
    assert!(j["missing"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"9/2026","deed_kind":"ajb"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let pid2 = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid2}/checklist"), &tok))
        .await
        .unwrap();
    let j = body_json(res).await;
    assert_eq!(j["complete"], false);
    assert_eq!(j["missing"].as_array().unwrap().len(), 3);
    let res = a
        .clone()
        .oneshot(get("/api/v1/ppat/999999999/checklist", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Audit filter surfaces the new ppat trail.
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=ppat", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "ppat.created"));
}

#[tokio::test]
async fn clause_library_versions_and_guards() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clause@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // RBAC: auditor cannot create.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/deed-clauses",
            r#"{"name":"K1","body":"isi"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Validation: blank body rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/deed-clauses",
            r#"{"name":"K1","body":""}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/deed-clauses",
            r#"{"name":"Klausul AJB","category":"ajb","body":"Para pihak {{nama}} sepakat."}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["version"], 1);
    let cid = body_json(
        a.clone()
            .oneshot(post(
                "/api/v1/deed-clauses",
                r#"{"name":"K2","body":"x"}"#,
                Some(&tok),
            ))
            .await
            .unwrap(),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    // Update bumps version; list shows it; delete removes.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/deed-clauses/{cid}"))
                .method("PUT")
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::from(r#"{"body":"revisi"}"#.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["version"], 2);
    let res = a
        .clone()
        .oneshot(get("/api/v1/deed-clauses", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"].as_i64() == Some(cid) && c["version"] == 2));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/deed-clauses/{cid}"))
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
        .oneshot(get("/api/v1/deed-clauses", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["id"].as_i64() != Some(cid)));
}

#[tokio::test]
async fn land_certificate_intake_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cert@x.id").await;
    let did = mk_search_doc(&a, &tok, "shm-001.jpg").await;
    // Unknown document rejected; valid intake queues a land_certificate job.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ocr/jobs",
            r#"{"document_id":999999999,"doc_type":"land_certificate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ocr/jobs",
            &format!(r#"{{"document_id":{did},"doc_type":"land_certificate"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let jid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/ocr/jobs", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|j| j["id"].as_i64() == Some(jid) && j["doc_type"] == "land_certificate"));
}

#[tokio::test]
async fn ocr_accuracy_report_empty_and_audited() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ocracc@x.id").await;
    // Fresh org: header only, still audit-trailed.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/ocr-accuracy.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()["content-type"]
        .to_str()
        .unwrap()
        .contains("text/csv"));
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(text.trim(), "doc_type,jobs,fields,avg_confidence");
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=report", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "report.ocr_exported"));
}

#[tokio::test]
async fn document_version_history_lists_versions() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docver@x.id").await;
    // Create doc without content: no versions yet.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            r#"{"name":"v.txt","mime":"text/plain","size":4,"sha":"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["versions"]
        .as_array()
        .unwrap()
        .is_empty());
    // Upload content ("test" sha256 above) -> version 1 recorded.
    let req = Request::builder()
        .uri(format!("/api/v1/documents/{did}/content"))
        .method("PUT")
        .header(
            "authorization",
            format!("Bearer {tok}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .body(Body::from(b"test".to_vec()))
        .unwrap();
    let res = a.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["version"], 1);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}"), &tok))
        .await
        .unwrap();
    let vs = body_json(res).await["versions"].clone();
    assert_eq!(vs.as_array().unwrap().len(), 1);
    assert_eq!(vs[0]["v"], 1);
    // Unknown doc is a clean 404.
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/999999999", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dash_flow_invoice_create() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dashinv@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/invoices"),
            r#"{"total_minor":2500000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/invoices/{iid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert_eq!(j["total"].as_i64().unwrap(), 2500000);
    assert_eq!(j["balance"].as_i64().unwrap(), 2500000);
}

#[tokio::test]
async fn invoice_list_shows_paid_balance_and_records_payment() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "invpay@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let iid = mk_invoice(&a, &tok, mid, 1000000).await;
    // List carries paid/balance for the Keuangan tab pay form.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/invoices"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list = body_json(res).await;
    let row = &list.as_array().unwrap()[0];
    assert_eq!(row["id"].as_i64().unwrap(), iid);
    assert_eq!(row["paid"].as_i64().unwrap(), 0);
    assert_eq!(row["balance"].as_i64().unwrap(), 1000000);
    // Record partial payment; list reflects it; overpay rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/invoices/{iid}/payments"),
            r#"{"amount_minor":250000,"method":"cash"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/invoices"), &tok))
        .await
        .unwrap();
    let list = body_json(res).await;
    let row = &list.as_array().unwrap()[0];
    assert_eq!(row["paid"].as_i64().unwrap(), 250000);
    assert_eq!(row["balance"].as_i64().unwrap(), 750000);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/invoices/{iid}/payments"),
            r#"{"amount_minor":800000,"method":"transfer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Receipt carries totals + payment lines + audited view.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/invoices/{iid}/receipt"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let r = body_json(res).await;
    assert_eq!(r["total"].as_i64().unwrap(), 1000000);
    assert_eq!(r["paid"].as_i64().unwrap(), 250000);
    assert_eq!(r["balance"].as_i64().unwrap(), 750000);
    assert_eq!(r["status"], "belum_lunas");
    assert_eq!(r["payments"].as_array().unwrap().len(), 1);
    assert!(r["issued_at"].as_str().unwrap().len() >= 10);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=report", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "report.receipt_viewed"));
    // Unknown invoice is a clean 404.
    let res = a
        .clone()
        .oneshot(get("/api/v1/invoices/999999999/receipt", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dash_search_still_green() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dashsearch@x.id").await;
    let cid = mk_search_client(&a, &tok, "Dashsearch Client").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            r#"{"title":"Dashsearch Perkara"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let did = mk_search_doc(&a, &tok, "dashsearch-doc.txt").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/search?q=dashsearch", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j["clients"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"].as_i64() == Some(cid)));
    assert!(j["matters"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["id"].as_i64() == Some(mid)));
    assert!(j["documents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"].as_i64() == Some(did)));
}

#[tokio::test]
async fn ui5_tokens_crud_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ui5tok@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/tokens",
            r#"{"name":"ci-token"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    let plain = j["token"].as_str().unwrap().to_string();
    let tid = j["id"].as_str().unwrap().to_string();
    assert!(plain.starts_with("lr_"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/auth/tokens", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list = body_json(res).await;
    assert!(list.as_array().unwrap().iter().any(|t| t["id"] == tid));
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/auth/tokens/{tid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/auth/tokens", &tok))
        .await
        .unwrap();
    assert!(!body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"] == tid));
}

#[tokio::test]
async fn ui5_password_change_then_login_with_new() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ui5pw@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/password",
            r#"{"current_password":"password1-2-3","new_password":"password9-9-9"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/login",
            r#"{"email":"ui5pw@x.id","password":"password9-9-9"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/login",
            r#"{"email":"ui5pw@x.id","password":"password1-2-3"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ui5_role_assign_ok() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ui5role@x.id").await;
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
            r#"{"role":"auditor"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["roles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == "auditor"));
}

#[tokio::test]
async fn ui5_role_assign_invalid_422() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ui5badrole@x.id").await;
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
            r#"{"role":"presiden"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn token_expiry_set_and_validated() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "tokexp@x.id").await;
    // Explicit 30-day expiry lands ~30 days out.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/tokens",
            r#"{"name":"ci30","expires_in_days":30}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let exp = body_json(res).await["expires_at"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(exp.len() >= 10);
    // Out-of-range windows rejected; default is 90 days.
    for bad in [0, -5, 400] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/auth/tokens",
                &format!(r#"{{"name":"bad{bad}","expires_in_days":{bad}}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/tokens",
            r#"{"name":"nodefault"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert!(body_json(res).await["expires_at"].as_str().is_some());
    let res = a
        .clone()
        .oneshot(get("/api/v1/auth/tokens", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["name"] == "ci30" && t["expires_at"].as_str() == Some(exp.as_str())));
}

#[tokio::test]
async fn session_console_lists_and_revokes_others() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "sesscon@x.id").await;
    let tok_b = reg(&a, "sessconB@x.id").await;
    // Second login = second session.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/login",
            r#"{"email":"sesscon@x.id","password":"password1-2-3"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/auth/sessions", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let all = v.as_array().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all.iter().filter(|s| s["current"] == true).count(), 1);
    assert!(all.iter().all(|s| s["revoked"] == false));
    let other = all.iter().find(|s| s["current"] != true).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let current = all.iter().find(|s| s["current"] == true).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Revoke the other session; self-revoke refused; foreign sid hidden.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/auth/sessions/{other}/revoke"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/auth/sessions/{current}/revoke"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/auth/sessions/{other}/revoke"),
            "{}",
            Some(&tok_b),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/auth/sessions", &tok))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["id"] == other && s["revoked"] == true));
}

#[tokio::test]
async fn sod_check_flags_risky_pairs() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "sodown@x.id").await;
    // Owner holds broad permissions: self-approval-style pairs conflict.
    let res = a
        .clone()
        .oneshot(get("/api/v1/roles/sod", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(!v["permissions"].as_array().unwrap().is_empty());
    let pairs: Vec<String> = v["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["pair"][0].as_str().unwrap().to_string() + "+" + c["pair"][1].as_str().unwrap())
        .collect();
    assert!(
        pairs.contains(&"Assign+Download".to_string()),
        "pairs: {pairs:?}"
    );
    assert!(
        pairs.contains(&"Sign+Verify".to_string()),
        "pairs: {pairs:?}"
    );
    assert!(
        pairs.contains(&"Create+Approve".to_string()),
        "pairs: {pairs:?}"
    );
    // Auditor lacks Assign: forbidden. Unknown user: 404.
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(get("/api/v1/roles/sod", &auditor))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(get("/api/v1/roles/sod?user=nobody", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn role_matrix_lists_least_privilege_sets() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "rolemat@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/roles/matrix", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let all = v.as_array().unwrap();
    assert_eq!(all.len(), 12);
    let owner = all.iter().find(|r| r["role"] == "org_owner").unwrap();
    assert_eq!(owner["permissions"].as_array().unwrap().len(), 12);
    let auditor = all.iter().find(|r| r["role"] == "auditor").unwrap();
    let ap = auditor["permissions"].as_array().unwrap();
    assert!(ap.iter().any(|p| p == "View"));
    assert!(!ap.iter().any(|p| p == "Assign"));
    assert!(!ap.iter().any(|p| p == "Approve"));
    // Unauthenticated callers get nothing.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/roles/matrix")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn public_pages_all_render_200() {
    use tower::ServiceExt;
    let a = app();
    for uri in [
        "/",
        "/fitur",
        "/fitur/manajemen-perkara",
        "/fitur/manajemen-dokumen",
        "/fitur/ocr",
        "/fitur/ai-legal-assistant",
        "/fitur/generasi-akta",
        "/fitur/tanda-tangan",
        "/solusi",
        "/solusi/notaris",
        "/solusi/ppat",
        "/solusi/firma-hukum",
        "/solusi/legal-corporate",
        "/harga",
        "/tentang",
        "/artikel",
        "/kontak",
        "/demo",
        "/faq",
        "/kebijakan-privasi",
        "/syarat-ketentuan",
        "/login",
        "/sitemap.xml",
        "/robots.txt",
        "/app",
    ] {
        let res = a
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "page {uri} must render");
    }
}

#[tokio::test]
async fn delegation_guards_and_audit() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "deleg@x.id").await;
    let tok_b = reg(&a, "delegB@x.id").await;
    let other_uid = body_json(
        a.clone()
            .oneshot(get("/api/v1/orgs/me", &tok_b))
            .await
            .unwrap(),
    )
    .await["user_id"]
        .as_str()
        .unwrap()
        .to_string();
    // Cross-org delegate invisible; self-delegation conflicts.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/delegations",
            &format!(r#"{{"delegate_uid":"{other_uid}"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let my_uid = body_json(
        a.clone()
            .oneshot(get("/api/v1/orgs/me", &tok))
            .await
            .unwrap(),
    )
    .await["user_id"]
        .as_str()
        .unwrap()
        .to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/delegations",
            &format!(r#"{{"delegate_uid":"{my_uid}"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Unknown uid 404; auditor cannot delegate.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/delegations",
            r#"{"delegate_uid":"no-such-user"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/delegations",
            &format!(r#"{{"delegate_uid":"{my_uid}"}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Empty list for fresh org.
    let res = a
        .clone()
        .oneshot(get("/api/v1/delegations", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn invite_accept_joins_org_and_enables_delegation() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "inviter@x.id").await;
    // Owner invites a lawyer; non-Assign callers cannot.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/orgs/invites",
            r#"{"email":"member@x.id","role":"lawyer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let inv_token = body_json(res).await["token"].as_str().unwrap().to_string();
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/orgs/invites",
            r#"{"email":"other@x.id","role":"lawyer"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Invite list shows pending; wrong email is rejected at redeem.
    let res = a
        .clone()
        .oneshot(get("/api/v1/orgs/invites", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/register",
            &format!(r#"{{"org_name":"Acme","email":"wrong@x.id","name":"Wa","password":"password1-2-3","invite_token":"{inv_token}"}}"#),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Correct email joins the same org with the invited role.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/register",
            &format!(r#"{{"org_name":"Acme","email":"member@x.id","name":"Anggota","password":"password1-2-3","invite_token":"{inv_token}"}}"#),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    let member_tok = j["token"].as_str().unwrap().to_string();
    let member_uid = j["user_id"].as_str().unwrap().to_string();
    let org_a = body_json(
        a.clone()
            .oneshot(get("/api/v1/orgs/me", &tok))
            .await
            .unwrap(),
    )
    .await["org_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(j["org_id"].as_str().unwrap(), org_a);
    // Member visible in the org directory now.
    let res = a.clone().oneshot(get("/api/v1/users", &tok)).await.unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u["id"].as_str() == Some(member_uid.as_str())));
    // Reuse of the consumed token is rejected (already accepted).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/register",
            &format!(r#"{{"org_name":"Acme","email":"member@x.id","name":"Dup","password":"password1-2-3","invite_token":"{inv_token}"}}"#),
            None,
        ))
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::OK);
    // End-to-end delegation: member (lawyer, no Approve) cannot decide a
    // finance step; after owner delegates, the same decision succeeds.
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"finance_officer"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&member_tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/delegations",
            &format!(r#"{{"delegate_uid":"{member_uid}"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&member_tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn users_list_scoped_and_cross_org_assign_blocked() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "usrlist@x.id").await;
    let tok_b = reg(&a, "usrlistB@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Directory lists own org only, with roles attached.
    let res = a.clone().oneshot(get("/api/v1/users", &tok)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let mine = v
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "usrlist@x.id")
        .expect("self listed");
    assert!(mine["roles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == "org_owner"));
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .all(|u| u["email"] != "usrlistB@x.id"));
    // No-token and under-privileged callers are rejected.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(get("/api/v1/users", &auditor))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Cross-tenant role grant is a 404 (no org leak signal).
    let other_uid = body_json(
        a.clone()
            .oneshot(get("/api/v1/users", &tok_b))
            .await
            .unwrap(),
    )
    .await[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/users/{other_uid}/roles"),
            r#"{"role":"lawyer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Same-org grant works and is audit-trailed.
    let my_uid = mine["id"].as_str().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/users/{my_uid}/roles"),
            r#"{"role":"lawyer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=user", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "user.role_assigned:lawyer"));
}

#[tokio::test]
async fn ui5_clients_csv_export() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ui5csv@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"PT UI5","kind":"corporate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients.csv", &tok))
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
    assert_eq!(lines.next().unwrap(), "id,display_name,kind");
    let first = lines.next().expect("one client row");
    assert!(first.contains("PT UI5"), "row was {first}");
    assert!(first.contains("corporate"), "row was {first}");
}

#[tokio::test]
async fn ui5_app_pengaturan_ids_and_faq_lupa_link() {
    let a = app();
    let (st, _, t) = page_get(&a, "/app").await;
    assert_eq!(st, StatusCode::OK);
    for needle in [
        "p-pengaturan",
        "tok-name",
        "tok-btn",
        "tok-list",
        "tok-days",
        "sod-btn",
        "sod-list",
        "role-matrix",
        "sess-list",
        "sess-reload",
        "sess-msg",
        "tok-plain",
        "pw-cur",
        "pw-new",
        "pw-btn",
        "role-uid",
        "role-sel",
        "role-btn",
        "role-msg",
        "role-matrix",
        "inv-email",
        "inv-role",
        "inv-add",
        "inv-reload",
        "inv-list",
        "inv-plain",
        "inv-msg",
        "cli-csv",
        "bell-n",
        "bell-list",
        "Pengaturan",
        "Ekspor CSV",
    ] {
        assert!(t.contains(needle), "/app missing {needle}");
    }
    assert!(
        t.contains("data-tab=\"pengaturan\""),
        "missing pengaturan tab link"
    );
    let (st, _, t) = page_get(&a, "/login").await;
    assert_eq!(st, StatusCode::OK);
    assert!(t.contains("Lupa kata sandi?"), "login missing Lupa link");
    assert!(t.contains("href=\"/faq\""), "Lupa link must point to /faq");
    let (st, _, _) = page_get(&a, "/faq").await;
    assert_eq!(st, StatusCode::OK);
}

async fn mk_crm_client(app: &axum::Router, tok: &str) -> i64 {
    use tower::ServiceExt;
    let res = app
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"CRM Client","kind":"individual"}"#,
            Some(tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["id"].as_i64().unwrap()
}

#[tokio::test]
async fn crm_meeting_validation_422() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "crmv@x.id").await;
    let cid = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cid}/meetings"),
            r#"{"title":"ab","happened_at":"2026-11-18 10:00:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cid}/meetings"),
            r#"{"title":"Kickoff call","happened_at":"18-11-2026"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn crm_meeting_flow_ok() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "crmok@x.id").await;
    let cid = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cid}/meetings"),
            r#"{"title":"Kickoff call","notes":"scope notes","happened_at":"2026-11-18 10:00:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cid}/meetings"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list = body_json(res).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"].as_str().unwrap(), "Kickoff call");
    let res = a
        .clone()
        .oneshot(get("/api/v1/meetings?limit=10", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let recent = body_json(res).await;
    assert!(recent
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["id"].as_i64() == Some(mid)));
}

#[tokio::test]
async fn crm_meeting_cross_org_404() {
    use tower::ServiceExt;
    let a = app();
    let tok_a = reg(&a, "crma@x.id").await;
    let tok_b = reg(&a, "crmb@x.id").await;
    let cid = mk_crm_client(&a, &tok_a).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cid}/meetings"),
            r#"{"title":"Foreign attempt","happened_at":"2026-11-18 10:00:00"}"#,
            Some(&tok_b),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cid}/meetings"), &tok_b))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cid}/meetings"),
            r#"{"title":"Own meeting","happened_at":"2026-11-18 10:00:00"}"#,
            Some(&tok_a),
        ))
        .await
        .unwrap();
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/meetings/{mid}"), &tok_b))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/meetings", &tok_b))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn crm_tag_dup_409() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "crmt@x.id").await;
    let res = a
        .clone()
        .oneshot(post("/api/v1/tags", r#"{"name":"vip"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post("/api/v1/tags", r#"{"name":"vip"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a.clone().oneshot(get("/api/v1/tags", &tok)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn crm_tag_attach_idempotent() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "crmat@x.id").await;
    let cid = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post("/api/v1/tags", r#"{"name":"priority"}"#, Some(&tok)))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    for _ in 0..2 {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/clients/{cid}/tags"),
                &format!(r#"{{"tag_id":{tid}}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cid}/tags"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tags = body_json(res).await;
    assert_eq!(tags.as_array().unwrap().len(), 1);
    assert_eq!(tags[0]["name"].as_str().unwrap(), "priority");
}

#[tokio::test]
async fn crm_tag_detach() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "crmdt@x.id").await;
    let cid = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post("/api/v1/tags", r#"{"name":"temp"}"#, Some(&tok)))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cid}/tags"),
            &format!(r#"{{"tag_id":{tid}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/clients/{cid}/tags/{tid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/clients/{cid}/tags/{tid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cid}/tags"), &tok))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn crm_meeting_delete() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "crmdel@x.id").await;
    let cid = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cid}/meetings"),
            r#"{"title":"To remove","happened_at":"2026-11-18 10:00:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/meetings/{mid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(del_auth(&format!("/api/v1/meetings/{mid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cid}/meetings"), &tok))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn crm_client_tags_cross_org_404() {
    use tower::ServiceExt;
    let a = app();
    let tok_a = reg(&a, "crmca@x.id").await;
    let tok_b = reg(&a, "crmcb@x.id").await;
    let cid = mk_crm_client(&a, &tok_a).await;
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cid}/tags"), &tok_b))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn crm_app_contains_crm_ids() {
    let (st, _, t) = page_get(&app(), "/app").await;
    assert_eq!(st, StatusCode::OK);
    for needle in [
        "crm-meet-list",
        "crm-meet-btn",
        "crm-tag-list",
        "crm-tag-btn",
        "crm-ct-list",
        "crm-recent",
    ] {
        assert!(t.contains(needle), "/app missing {needle}");
    }
}

// ----- app UI batch: audit viewer, jobs queue, CSV exports, bell, palette -----
fn lawyer_token_from(owner_tok: &str) -> String {
    // Re-issue the same live session with a lawyer-only role set.
    // assign_role() only appends, so forging via the known dev secret is the
    // only way to exercise the audit 403 path in-process.
    let secret = b"test-secret-0123456789-0123456789ab";
    let c: legalrust_auth::Claims =
        legalrust_auth::validate_token(secret, owner_tok).expect("decode owner token");
    legalrust_auth::issue_token(
        secret,
        &legalrust_auth::Claims {
            sub: c.sub,
            org: c.org,
            roles: vec!["lawyer".into()],
            sid: c.sid,
            exp: c.exp,
        },
    )
    .expect("reissue lawyer token")
}

#[tokio::test]
async fn uib_app_has_audit_jobs_ui() {
    let (st, _, t) = page_get(&app(), "/app").await;
    assert_eq!(st, StatusCode::OK);
    for id in [
        "p-audit",
        "p-jobs",
        "audit-entity",
        "audit-rows",
        "audit-reload",
        "audit-msg",
        "audit-note",
        "job-rows",
        "job-kind",
        "job-btn",
        "job-msg",
        "rep-matters",
        "rep-deadlines",
        "rep-posture",
        "rep-exec",
        "rep-msg",
        "keu-csv",
        "keu-msg",
        "bell",
        "bell-n",
        "bell-list",
    ] {
        assert!(t.contains(&format!("id=\"{id}\"")), "/app missing #{id}");
    }
    assert!(t.contains("data-tab=\"audit\""), "missing audit tab link");
    assert!(t.contains("data-tab=\"jobs\""), "missing jobs tab link");
    assert!(t.contains(">Audit<"), "missing Audit nav label");
    assert!(t.contains(">Jobs<"), "missing Jobs nav label");
    assert!(
        t.contains("Operasional"),
        "missing Operasional sidebar group"
    );
    assert!(t.contains("Jejak Audit"), "missing audit section title");
    assert!(t.contains("Antrean Jobs"), "missing jobs section title");
    assert!(t.contains("Ekspor Laporan"), "missing export card");
    assert!(!t.contains("onclick"), "inline handler in /app");
    assert!(!t.contains("style=\""), "inline style in /app");
    assert!(!t.contains("<style"), "style tag in /app");
}

#[tokio::test]
async fn uib_site_js_wires_batch_widgets() {
    let (_, _, js) = page_get(&app(), "/static/site.js").await;
    for needle in [
        "audit",
        "jobs",
        "loadAudit",
        "loadJobs",
        "audit-rows",
        "job-rows",
        "audit-entity",
        "job-kind",
        "rep-matters",
        "keu-csv",
        "/api/v1/audit",
        "/api/v1/jobs",
        "/api/v1/reports/matters.csv",
        "/api/v1/reports/matters-csv",
        "ctrlKey",
    ] {
        assert!(js.contains(needle), "site.js missing {needle}");
    }
    assert!(!js.contains("onclick"), "inline handler in site.js");
}

#[tokio::test]
async fn uib_audit_200_shape_admin() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "uibaudit@x.id").await;
    let _ = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=matter", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let arr = v.as_array().unwrap();
    assert!(!arr.is_empty(), "expected audit rows after matter create");
    let first = &arr[0];
    for key in ["id", "actor", "action", "entity", "created"] {
        assert!(first.get(key).is_some(), "audit row missing {key}");
    }
}

#[tokio::test]
async fn uib_audit_401_without_token() {
    use tower::ServiceExt;
    let res = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn uib_audit_403_for_lawyer_role() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "uiblaw@x.id").await;
    let lawyer = lawyer_token_from(&tok);
    // forged token still rides the live session
    let res = a
        .clone()
        .oneshot(get("/api/v1/orgs/me", &lawyer))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit", &lawyer))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // owner token keeps access
    let res = a.clone().oneshot(get("/api/v1/audit", &tok)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn uib_jobs_enqueue_list_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "uibjobs@x.id").await;
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
    let j = body_json(res).await;
    assert!(j["id"].as_i64().unwrap() > 0);
    assert_eq!(j["status"].as_str().unwrap(), "queued");
    let res = a.clone().oneshot(get("/api/v1/jobs", &tok)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let arr = body_json(res).await;
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["kind"].as_str().unwrap(), "notify.digest");
    assert_eq!(arr[0]["status"].as_str().unwrap(), "queued");
    assert!(arr[0].get("attempts").is_some(), "job missing attempts");
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn uib_csv_exports_content_type() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "uibcsv@x.id").await;
    let _ = mk_matter(&a, &tok).await;
    for uri in ["/api/v1/reports/matters.csv", "/api/v1/reports/matters-csv"] {
        let res = a.clone().oneshot(get(uri, &tok)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{uri}");
        let ctype = res
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(ctype.contains("text/csv"), "{uri} content-type was {ctype}");
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(text.lines().next().unwrap(), "id,number,title,status");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients.csv", &tok))
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
    assert!(ctype.contains("text/csv"), "clients.csv type was {ctype}");
}

#[tokio::test]
async fn uib_notifications_bell_shape() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "uibbell@x.id").await;
    let _ = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let arr = body_json(res).await;
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 1, "matter create must notify");
    let n = &arr[0];
    for key in ["id", "kind", "title", "read"] {
        assert!(n.get(key).is_some(), "notification missing {key}");
    }
    assert_eq!(n["read"].as_bool(), Some(false));
    let unread = arr.iter().filter(|x| x["read"] == false).count();
    assert_eq!(unread, 1);
    let nid = n["id"].as_i64().unwrap();
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/notifications", &tok))
        .await
        .unwrap();
    let arr2 = body_json(res).await;
    assert_eq!(arr2.as_array().unwrap()[0]["read"].as_bool(), Some(true));
}

#[tokio::test]
async fn uib_trust_has_no_csv_endpoint() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "uibtrust@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/trust/accounts.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ----- approvals engine -----
fn forged_role_token(owner_tok: &str, roles: Vec<&str>) -> String {
    let secret = b"test-secret-0123456789-0123456789ab";
    let c: legalrust_auth::Claims =
        legalrust_auth::validate_token(secret, owner_tok).expect("decode owner token");
    legalrust_auth::issue_token(
        secret,
        &legalrust_auth::Claims {
            sub: c.sub,
            org: c.org,
            roles: roles.into_iter().map(str::to_string).collect(),
            sid: c.sid,
            exp: c.exp,
        },
    )
    .expect("reissue role token")
}

async fn mk_approval(app: &axum::Router, tok: &str, mid: i64, steps: &str) -> i64 {
    use tower::ServiceExt;
    let res = app
        .clone()
        .oneshot(post(
            "/api/v1/approvals",
            &format!(
                r#"{{"entity":"matter","entity_id":{mid},"title":"Setujui AJB","steps":{steps}}}"#
            ),
            Some(tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["status"].as_str().unwrap(), "pending");
    v["id"].as_i64().unwrap()
}

#[tokio::test]
async fn approvals_sequential_flow() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "appr@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(
        &a,
        &tok,
        mid,
        r#"[{"step_no":1,"role":"office_admin"},{"step_no":2,"role":"finance_officer"}]"#,
    )
    .await;
    // Out-of-order decision is rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":2,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Step 1 approve (owner bypasses role match), step 2 approve closes.
    for (no, done) in [(1, "pending"), (2, "approved")] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/approvals/{aid}/decisions"),
                &format!(r#"{{"step_no":{no},"decision":"approve"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let v = body_json(res).await;
        assert_eq!(v["status"].as_str().unwrap(), "approved");
        assert_eq!(v["request_status"].as_str().unwrap(), done);
    }
    // Re-deciding a closed step and revoking a closed request both fail.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":2,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/revoke"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // List filter + history evidence.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals?status=approved", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64().unwrap() == aid));
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid}"), &tok))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["steps"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn approvals_rework_revoke_roundtrip() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "appr2@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"rework","comment":"lengkapi lampiran"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["request_status"].as_str().unwrap(),
        "needs_rework"
    );
    // Rework resets the step to pending; a fresh approve closes it.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(
        body_json(res).await["request_status"].as_str().unwrap(),
        "approved"
    );
    // Revoke path on a fresh request.
    let aid2 = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid2}/revoke"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid2}"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["status"].as_str().unwrap(), "revoked");
}

#[tokio::test]
async fn webhook_deliveries_logged_and_listed() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "whlog@x.id").await;
    let body = br#"{"event":"signed","deed":"D-1"}"#;
    let sig = legalrust_integrations::sign_webhook(b"test-integration", body, "t9");
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/webhooks/incoming/psre")
                .method("POST")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .header("x-signature", sig)
                .header("x-ts", "t9")
                .header("content-type", "application/json")
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/webhooks/deliveries", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Tenant-scoped list: the HMAC-shared-secret receipt has unknown org,
    // so only shape is asserted here (attributed tripay rows are covered
    // in tripay_gateway_and_callback_against_stub).
    let res = a
        .clone()
        .oneshot(get("/api/v1/webhooks/deliveries", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.is_array());
    let res = a
        .clone()
        .oneshot(get("/api/v1/webhooks/deliveries?provider=tripay", &tok))
        .await
        .unwrap();
    assert_eq!(
        body_json(res).await.as_array().unwrap().len(),
        0,
        "tripay filter must be empty here"
    );
    // Bad signature is rejected (401); delivery log keeps the rejection.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/webhooks/incoming/psre")
                .method("POST")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .header("x-signature", "bad")
                .header("x-ts", "t9")
                .header("content-type", "application/json")
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approvals_validation_and_rbac() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "appr3@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let notary = forged_role_token(&tok, vec!["notary"]);
    // Lawyer (no Create) cannot compose; unknown entity/matter/steps rejected.
    for (payload, want) in [
        (
            format!(r#"{{"entity":"matter","entity_id":{mid},"title":"T","steps":[]}}"#),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!(
                r#"{{"entity":"deed","entity_id":{mid},"title":"T","steps":[{{"step_no":1,"role":"office_admin"}}]}}"#
            ),
            StatusCode::NOT_FOUND,
        ),
        (
            format!(
                r#"{{"entity":"alien","entity_id":{mid},"title":"T","steps":[{{"step_no":1,"role":"office_admin"}}]}}"#
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!(
                r#"{{"entity":"matter","entity_id":{mid},"title":"T","steps":[{{"step_no":2,"role":"office_admin"}}]}}"#
            ),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let res = a
            .clone()
            .oneshot(post("/api/v1/approvals", &payload, Some(&tok)))
            .await
            .unwrap();
        assert_eq!(res.status(), want, "payload was {payload}");
    }
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals",
            &format!(r#"{{"entity":"matter","entity_id":{mid},"title":"T","steps":[{{"step_no":1,"role":"office_admin"}}]}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Lawyer (no Approve) cannot decide; notary cannot decide a finance step.
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"finance_officer"}]"#).await;
    for (t, want) in [
        (&lawyer, StatusCode::FORBIDDEN),
        (&notary, StatusCode::FORBIDDEN),
    ] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/approvals/{aid}/decisions"),
                r#"{"step_no":1,"decision":"approve"}"#,
                Some(t),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), want);
    }
    // Notary CAN decide a notary step.
    let aid2 = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"notary"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid2}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&notary),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Unknown list filter rejected.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals?status=bogus", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn compliance_obligation_and_consent_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "pdp@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Obligation compose + validation.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/obligations",
            r#"{"title":"","category":"retention"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/obligations",
            r#"{"title":"ROPA review Q4","category":"ropa","owner":"dpo@x.id"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Lawyer holds Create: may compose, but cannot change status (no Update).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/obligations",
            r#"{"title":"DPIA vendor X","category":"dpia"}"#,
            Some(&lawyer),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let loid = body_json(res).await["id"].as_i64().unwrap();
    // Auditor lacks Update: cannot transition status.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/obligations/{loid}/status"),
            r#"{"status":"in_progress"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/obligations",
            r#"{"title":"ROPA review Q4","category":"ropa","owner":"dpo@x.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let oid = body_json(res).await["id"].as_i64().unwrap();
    // open -> in_progress -> done; done is terminal.
    for (st, want) in [("in_progress", StatusCode::OK), ("done", StatusCode::OK)] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/obligations/{oid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), want, "transition to {st}");
    }
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/obligations/{oid}/status"),
            r#"{"status":"open"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/obligations?status=done", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64().unwrap() == oid));
    // Consent grant -> duplicate 409 -> withdraw -> re-grant ok.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/consents",
            r#"{"subject":"nik-3174","purpose":"kyc"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/consents",
            r#"{"subject":"nik-3174","purpose":"kyc"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/consents/{cid}/withdraw"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/consents/{cid}/withdraw"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/consents",
            r#"{"subject":"nik-3174","purpose":"kyc"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/consents?subject=nik-3174", &tok))
        .await
        .unwrap();
    let list = body_json(res).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert!(list
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["status"] == "withdrawn"));
}

#[tokio::test]
async fn webhook_endpoints_crud_and_test_send() {
    use std::io::{Read, Write};
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "whout@x.id").await;
    // Validation: non-http(s) URL rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/webhooks/endpoints",
            r#"{"url":"ftp://x/y"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Local stub asserting the signed test ping (synchronous in-request send).
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind wh stub");
    let addr = listener.local_addr().expect("stub addr");
    let stub = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().expect("wh stub accept");
        let mut buf = vec![0u8; 65536];
        let n = s.read(&mut buf).expect("stub read");
        let text = String::from_utf8_lossy(&buf[..n]).into_owned();
        assert!(text.contains("x-webhook-signature"), "missing sig header");
        let body = br#"{"ok":true}"#;
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        s.write_all(head.as_bytes()).expect("stub head");
        s.write_all(body).expect("stub body");
    });
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/webhooks/endpoints",
            &format!(r#"{{"url":"http://{addr}/hook","secret":"topsecret","events":"*"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let eid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/webhooks/endpoints", &tok))
        .await
        .unwrap();
    let list = body_json(res).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert!(list[0].get("secret").is_none(), "secret must not leak");
    // Test-send delivers signed ping; delivery logged as outbound/accepted.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/webhooks/endpoints/{eid}/test"),
            r#"{"event":"ping"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    stub.join().expect("stub served");
    let res = a
        .clone()
        .oneshot(get("/api/v1/webhooks/deliveries?provider=outbound", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["status"] == "accepted" && r["event"] == "ping"));
    // Delete is Approve-gated (lawyer lacks it); owner deletes.
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let res = a
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/webhooks/endpoints/{eid}"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {lawyer}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/webhooks/endpoints/{eid}"))
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
}

#[tokio::test]
async fn webhook_auto_dispatch_on_notify() {
    use std::io::{Read, Write};
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "whout@x.id").await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind wh stub");
    let addr = listener.local_addr().expect("stub addr");
    // No join() on the stub thread: a blocking join inside a current-thread
    // test runtime starves the background dispatch task. The polling loop
    // below bounds the wait; the thread exits on its own.
    let _stub = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().expect("wh stub accept");
        let mut buf = vec![0u8; 65536];
        let n = s.read(&mut buf).expect("stub read");
        let text = String::from_utf8_lossy(&buf[..n]).into_owned();
        assert!(
            text.contains("x-webhook-signature"),
            "dispatch must be signed"
        );
        assert!(
            text.contains("notify:matter") || text.contains("matter"),
            "wrong event"
        );
        let body = br#"{"ok":true}"#;
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        s.write_all(head.as_bytes()).expect("stub head");
        s.write_all(body).expect("stub body");
    });
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/webhooks/endpoints",
            &format!(r#"{{"url":"http://{addr}/hook","secret":"s3cr3t","events":"matter"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Creating a matter fires notify_user(kind=matter) -> background dispatch.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            r#"{"title":"Dispatch me"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // No join here (see stub comment): poll for the background delivery.
    // Delivery log eventually shows the accepted outbound attempt.
    let mut found = false;
    for _ in 0..50 {
        let res = a
            .clone()
            .oneshot(get("/api/v1/webhooks/deliveries?provider=outbound", &tok))
            .await
            .unwrap();
        if body_json(res)
            .await
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["status"] == "accepted" && r["event"] == "matter")
        {
            found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(found, "outbound delivery not logged");
}

#[tokio::test]
async fn webhook_failed_send_queues_retry() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "whretry@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/webhooks/endpoints",
            r#"{"url":"http://127.0.0.1:1/hook","events":"matter"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            r#"{"title":"Retry me"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // The background task fails fast (refused) and must leave a retry job.
    let mut queued = false;
    for _ in 0..50 {
        let res = a.clone().oneshot(get("/api/v1/jobs", &tok)).await.unwrap();
        if body_json(res)
            .await
            .as_array()
            .unwrap()
            .iter()
            .any(|j| j["kind"] == "webhook.dispatch" && j["status"] == "queued")
        {
            queued = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(queued, "retry job not queued");
}

#[tokio::test]
async fn vendor_register_and_dpa_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "vendor@x.id").await;
    let tok_b = reg(&a, "vendorB@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // RBAC: auditor cannot create.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/vendors",
            r#"{"name":"Cloud ID"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/vendors",
            r#"{"name":"Cloud ID","service":"object storage"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let vid = body_json(res).await["id"].as_i64().unwrap();
    // Tenant isolation on read.
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/vendors", &tok_b))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
    // DPA workflow draft -> review -> signed; bad value rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/vendors/{vid}/dpa"),
            r#"{"dpa":"bogus"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["review", "signed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/vendors/{vid}/dpa"),
                &format!(r#"{{"dpa":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "dpa to {st}");
    }
    // Auditor cannot delete; owner deletes; twice -> 404.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/vendors/{vid}"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {auditor}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    for want in [StatusCode::OK, StatusCode::NOT_FOUND] {
        let res = a
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/compliance/vendors/{vid}"))
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
        assert_eq!(res.status(), want);
    }
}

#[tokio::test]
async fn dpia_breach_lifecycle_and_guards() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dpiabreach@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // RBAC: auditor cannot create either.
    for uri in ["/api/v1/compliance/dpia", "/api/v1/compliance/breach"] {
        let res = a
            .clone()
            .oneshot(post(uri, r#"{"title":"X"}"#, Some(&auditor)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "uri was {uri}");
    }
    // Bad risk rejected; valid DPIA walks draft -> review -> approved.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/dpia",
            r#"{"title":"Dx","risk_level":"extreme"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/dpia",
            r#"{"title":"DPIA vendor","data_types":"nik","risk_level":"high"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/dpia/{did}/status"),
            r#"{"status":"approved"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["review", "approved"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/dpia/{did}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "dpia to {st}");
    }
    // Breach: negative affected rejected; walks to closed.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/breach",
            r#"{"title":"Bx","affected":-1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/breach",
            r#"{"title":"Kebocoran email","discovered":"2026-11-18","affected":120}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["assessing", "notified", "closed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/breach/{bid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "breach to {st}");
    }
    // Lists are tenant-scoped and show final states.
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/dpia", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(did) && r["status"] == "approved"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/breach", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(bid) && r["status"] == "closed"));
}

#[tokio::test]
async fn compliance_ro_pa_crud_and_guards() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ropax@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Validation: activity too short rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/ropa",
            r#"{"activity":"X","purpose":"Y"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // RBAC: auditor cannot create (payload valid so the Create guard fires first).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/ropa",
            r#"{"activity":"ROPA Pemasaran","purpose":"Pemasaran langsung"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/ropa",
            r#"{"activity":"ROPA Pemasaran","purpose":"Pemasaran langsung","data_categories":"nama,email","retention":"2 tahun"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/ropa", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64().unwrap() == rid));
    // Delete is Delete-gated: auditor forbidden, owner ok; twice -> 404.
    let res = a
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/compliance/ropa/{rid}"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {auditor}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/compliance/ropa/{rid}"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/compliance/ropa/{rid}"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn trust_entry_add_and_detail_lists_mutations() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "trustmut@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"Trust Mut","kind":"individual"}"#,
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
    let aid = body_json(res).await["id"].as_i64().unwrap();
    // Credit posts; overdraft debit rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/trust/accounts/{aid}/entries"),
            r#"{"kind":"credit","amount_minor":500000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/trust/accounts/{aid}/entries"),
            r#"{"kind":"debit","amount_minor":600000}"#,
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
    let j = body_json(res).await;
    assert_eq!(j["balance"].as_i64().unwrap(), 500000);
    let es = j["entries"].as_array().unwrap();
    assert_eq!(es.len(), 1);
    assert_eq!(es[0]["kind"], "credit");
    assert_eq!(es[0]["amount"].as_i64().unwrap(), 500000);
    // Foreign account hidden.
    let res = a
        .clone()
        .oneshot(get("/api/v1/trust/accounts/999999999", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn trust_csv_export_returns_ledger() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "trustcsv@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"Trust Co","kind":"corporate"}"#,
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
            r#"{"kind":"credit","amount_minor":750000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/reports/trust-ledger.csv?account_id={aid}"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()["content-type"]
        .to_str()
        .unwrap()
        .contains("text/csv"));
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "id,matter_id,amount_cents,memo");
    let row = lines.next().expect("one entry row");
    assert!(row.contains("750000"), "row was {row}");
    assert!(row.contains("credit"), "row was {row}");
    // Export is audit-trailed under entity=report.
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=report", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "report.trust_csv_exported"));
}

#[tokio::test]
async fn executive_summary_aggregates_modules() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "execsum@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let _ = mk_invoice(&a, &tok, mid, 500000).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/executive-summary", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let j = body_json(res).await;
    assert!(j["matters"].as_i64().unwrap() >= 1);
    assert!(j["open_invoices"].as_i64().unwrap() >= 1);
    assert_eq!(j["open_balance"].as_i64().unwrap(), 500000);
    assert!(j.get("clients").is_some());
    assert!(j.get("deeds").is_some());
    assert!(j.get("pending_approvals").is_some());
    assert!(j.get("tasks_overdue").is_some());
    // Unauthorized without token.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/executive-summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reports_deadline_compliance_and_posture() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "repcmp@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    // One overdue open task, one future open task, one done overdue task.
    for (title, due) in [
        ("Overdue terbuka", Some("2001-01-01 09:00:00")),
        ("Mendatang", Some("2031-01-01 09:00:00")),
        ("Selesai lampau", Some("2001-01-01 09:00:00")),
    ] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/matters/{mid}/tasks"),
                &format!(r#"{{"title":"{title}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        let tid = body_json(res).await["id"].as_i64().unwrap();
        if let Some(d) = due {
            let res = a
                .clone()
                .oneshot(post(
                    &format!("/api/v1/tasks/{tid}/due"),
                    &format!(r#"{{"due":"{d}"}}"#),
                    Some(&tok),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }
        if title == "Selesai lampau" {
            for st in ["in_progress", "done"] {
                let res = a
                    .clone()
                    .oneshot(post(
                        &format!("/api/v1/tasks/{tid}"),
                        &format!(r#"{{"status":"{st}"}}"#),
                        Some(&tok),
                    ))
                    .await
                    .unwrap();
                assert_eq!(res.status(), StatusCode::OK, "task to {st}");
            }
        }
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/deadline-compliance.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        text.starts_with("matter_id,number,title,total_tasks,open_tasks,overdue_tasks\n"),
        "was {text}"
    );
    let row = text
        .lines()
        .find(|l| l.contains("M1") || l.contains(&mid.to_string()))
        .expect("matter row");
    assert!(row.ends_with(",3,2,1"), "row was {row}");
    // Posture snapshot reflects obligations created in this org.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/obligations",
            r#"{"title":"Retensi","category":"retention"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/compliance-posture.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(text.starts_with("domain,metric,value\n"), "was {text}");
    assert!(text.contains("obligation,open,1"), "was {text}");
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=report", &tok))
        .await
        .unwrap();
    let acts: Vec<String> = body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["action"].as_str().map(str::to_string))
        .collect();
    assert!(acts.contains(&"report.deadlines_exported".to_string()));
    assert!(acts.contains(&"report.compliance_exported".to_string()));
}

#[tokio::test]
async fn reports_bottleneck_and_profitability() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "reps@x.id").await;
    // Client + matter + open invoice.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"Rep Co","kind":"corporate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"M1","client_id":{cid}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let _ = mk_invoice(&a, &tok, mid, 800000).await;
    // Pending approval stuck at step 1.
    let aid = mk_approval(
        &a,
        &tok,
        mid,
        r#"[{"step_no":1,"role":"office_admin"},{"step_no":2,"role":"finance_officer"}]"#,
    )
    .await;
    // Bottleneck CSV names the blocking step.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals-bottleneck.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        text.starts_with("id,title,entity,stuck_step,stuck_role,pending_steps\n"),
        "was {text}"
    );
    assert!(
        text.contains(&format!("{aid}")) && text.contains(",1,office_admin,2"),
        "was {text}"
    );
    // Profitability CSV aggregates the client row.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/profitability.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        text.starts_with("client_id,client_name,matters,open_balance\n"),
        "was {text}"
    );
    assert!(
        text.contains(&format!("{cid},Rep Co,1,800000")),
        "was {text}"
    );
    // Both exports are audit-trailed.
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=report", &tok))
        .await
        .unwrap();
    let acts: Vec<String> = body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["action"].as_str().map(str::to_string))
        .collect();
    assert!(
        acts.contains(&"report.approvals_exported".to_string()),
        "missing approvals trail"
    );
    assert!(
        acts.contains(&"report.profitability_exported".to_string()),
        "missing profitability trail"
    );
}

#[tokio::test]
async fn ar_aging_csv_lists_only_open_invoices() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "araging@x.id").await;
    let m1 = mk_matter(&a, &tok).await;
    let m2 = mk_matter(&a, &tok).await;
    let i1 = mk_invoice(&a, &tok, m1, 1000000).await;
    let i2 = mk_invoice(&a, &tok, m2, 500000).await;
    // Partial pay on i1; overpay rejected; full pay on i2 closes it.
    for (iid, amount, want) in [
        (i1, 400000, StatusCode::OK),
        (i1, 700000, StatusCode::CONFLICT),
        (i2, 500000, StatusCode::OK),
    ] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/invoices/{iid}/payments"),
                &format!(r#"{{"amount_minor":{amount},"method":"transfer"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), want);
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/ar-aging.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()["content-type"]
        .to_str()
        .unwrap()
        .contains("text/csv"));
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let mut lines = text.lines();
    assert_eq!(
        lines.next().unwrap(),
        "id,number,matter_id,total_minor,paid_minor,balance_minor"
    );
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), 1, "only the partially-paid invoice stays open");
    assert!(rows[0].contains("1000000"), "row was {}", rows[0]);
    assert!(rows[0].contains("400000"), "row was {}", rows[0]);
    assert!(rows[0].contains("600000"), "row was {}", rows[0]);
    let _ = i1;
    let _ = i2;
}

#[tokio::test]
async fn deeds_csv_repertorium_export() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "deedcsv@x.id").await;
    let did = mk_deed(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/deeds.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()["content-type"]
        .to_str()
        .unwrap()
        .contains("text/csv"));
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "id,number,deed_type,state,created");
    let row = lines.next().expect("one deed row");
    assert!(row.contains("D-9"), "row was {row}");
    // Today's date filters to the deed; an ancient date yields header only.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/reports/deeds.csv?date={today}"),
            &tok,
        ))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert!(String::from_utf8(bytes.to_vec()).unwrap().contains("D-9"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/deeds.csv?date=2000-01-01", &tok))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert!(!String::from_utf8(bytes.to_vec()).unwrap().contains("D-9"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/deeds.csv?date=bogus", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Period: today..today contains the deed; ancient..ancient does not.
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/reports/deeds.csv?from={today}&to={today}"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert!(String::from_utf8(bytes.to_vec()).unwrap().contains("D-9"));
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/reports/deeds.csv?from=2000-01-01&to=2000-01-31",
            &tok,
        ))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert!(!String::from_utf8(bytes.to_vec()).unwrap().contains("D-9"));
    // Guards: bad shape and inverted range rejected.
    for uri in [
        "/api/v1/reports/deeds.csv?from=bogus".to_string(),
        format!("/api/v1/reports/deeds.csv?from={today}&to=2000-01-01"),
    ] {
        let res = a.clone().oneshot(get(&uri, &tok)).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "uri was {uri}"
        );
    }
    // Comparative summary: today vs ancient past (1 vs 0, delta 1).
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/reports/deeds.csv?compare=1&from={today}&to={today}&cmp_from=2000-01-01&cmp_to=2000-01-31"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        text.starts_with("metric,current,previous,delta\n"),
        "was {text}"
    );
    assert!(text.contains("count,1,0,1"), "was {text}");
    // Compare guards: missing/inverted/bad compare range rejected.
    for uri in [
        format!("/api/v1/reports/deeds.csv?compare=1&from={today}&to={today}"),
        format!("/api/v1/reports/deeds.csv?compare=1&from={today}&to={today}&cmp_from=2000-02-01&cmp_to=2000-01-01"),
        format!("/api/v1/reports/deeds.csv?compare=1&from={today}&to={today}&cmp_from=bogus&cmp_to=2000-01-31"),
    ] {
        let res = a.clone().oneshot(get(&uri, &tok)).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "uri was {uri}"
        );
    }
    let _ = did;
}

#[tokio::test]
async fn uib_app_has_approvals_ui() {
    let (st, _, t) = page_get(&app(), "/app").await;
    assert_eq!(st, StatusCode::OK);
    for needle in [
        "p-appr",
        "appr-entity",
        "appr-entity-id",
        "appr-title",
        "appr-role1",
        "appr-btn",
        "appr-msg",
        "appr-filter",
        "appr-reload",
        "appr-list",
        "appr-id",
        "appr-show",
        "appr-detail",
        "appr-step",
        "appr-comment",
        "appr-ok",
        "appr-no",
        "appr-rw",
        "appr-revoke",
        "appr-dmsg",
        "appr-err",
        "appr-notif-list",
        "appr-notif-reload",
        "appr-notif-unread",
        "appr-notif-unread-only",
        "deleg-uid",
        "deleg-add",
        "deleg-reload",
        "deleg-list",
        "job-summary",
        "trust-csv-acct",
        "trust-csv-btn",
        "trust-csv-msg",
        "trust-view-id",
        "trust-view",
        "trust-detail",
        "trust-entry-kind",
        "trust-entry-add",
        "ar-csv",
        "ar-msg",
        "appr-csv",
        "prof-csv",
        "Piutang terbuka",
        "il-msg",
        "wh-list",
        "wh-reload",
        "wh-msg",
        "whe-url",
        "whe-secret",
        "whe-add",
        "whe-reload",
        "whe-msg",
        "whe-list",
        "deed-rep-from",
        "deed-rep-to",
        "deed-cmp-from",
        "deed-cmp-to",
        "deed-cmp",
        "deed-csv",
        "deed-csv-msg",
        "wh-dl-list",
        "wh-dl-reload",
        "wh-dl-msg",
        "p-kepatuhan",
        "kep-title",
        "kep-add",
        "kep-list",
        "con-subject",
        "con-grant",
        "con-list",
        "ropa-activity",
        "ropa-purpose",
        "ropa-add",
        "ropa-reload",
        "ropa-list",
        "dpia-title",
        "dpia-add",
        "dpia-reload",
        "dpia-list",
        "breach-title",
        "breach-add",
        "breach-reload",
        "breach-list",
        "vendor-name",
        "vendor-service",
        "vendor-add",
        "vendor-reload",
        "vendor-list",
        "ROPA",
        "Kepatuhan",
        "Persetujuan",
    ] {
        assert!(t.contains(needle), "/app missing {needle}");
    }
    assert!(t.contains("data-tab=\"appr\""), "missing appr tab link");
}
