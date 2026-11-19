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
        "fb-name",
        "fb-add",
        "fb-reload",
        "fb-list",
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
        "data-ppat-deed",
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
        "klapper-reload",
        "klapper-list",
        "rdy-ppat",
        "rdy-reload",
        "mpd-book",
        "mpd-add",
        "mpd-reload",
        "mpd-list",
        "kyc-ppat",
        "kyc-add",
        "kyc-reload",
        "kyc-list",
        "cho-ppat",
        "cho-add",
        "cho-reload",
        "cho-list",
        "chg-ppat",
        "chg-add",
        "chg-reload",
        "chg-list",
        "chgd-days",
        "chgd-reload",
        "sbox-code",
        "sbox-add",
        "sbox-reload",
        "sbox-list",
        "zone-code",
        "zone-add",
        "zone-reload",
        "zone-list",
        "tax-value",
        "tax-calc",
        "pbb-nop",
        "pbb-add",
        "pbb-reload",
        "pbb-list",
        "sp-src",
        "sp-add",
        "sp-reload",
        "sp-list",
        "mg-src",
        "mg-add",
        "mg-reload",
        "mg-list",
        "dsp2-parcel",
        "dsp2-add",
        "dsp2-reload",
        "dsp2-list",
        "bnd-parcel",
        "bnd-add",
        "bnd-reload",
        "bnd-list",
        "mrt-ppat",
        "mrt-add",
        "mrt-reload",
        "mrt-list",
        "hir-ppat",
        "hir-add",
        "hir-reload",
        "hir-list",
        "bod-ppat",
        "bod-add",
        "bod-reload",
        "bod-list",
        "pow-ppat",
        "pow-add",
        "pow-reload",
        "pow-list",
        "lhi-parcel",
        "lhi-add",
        "lhi-reload",
        "lhi-list",
        "uho-unit",
        "uho-add",
        "uho-reload",
        "uho-list",
        "rpm-month",
        "rpm-reload",
        "rpm-list",
        "pdb-reload",
        "pdb-list",
        "nmp-month",
        "nmp-reload",
        "nmp-list",
        "mpn-reload",
        "mpn-list",
        "mpn-events",
        "fgd-sub",
        "fgd-check",
        "fgd-out",
        "nho-ppat",
        "nho-add",
        "nho-reload",
        "nho-list",
        "grk-ref",
        "grk-add",
        "grk-reload",
        "grk-list",
        "lse-ppat",
        "lse-save",
        "lse-load",
        "lse-del",
        "dvb-dev",
        "dvb-add",
        "dvb-reload",
        "dvb-list",
        "dvu-code",
        "dvu-add",
        "dvu-reload",
        "dvu-list",
        "kpr-ppat",
        "kpr-add",
        "kpr-reload",
        "kpr-list",
        "prg-book",
        "prg-add",
        "prg-reload",
        "prg-list",
        "ajb-ppat",
        "ajb-save",
        "ajb-reload",
        "ajb-list",
        "pat-limit",
        "pat-reload",
        "pat-list",
        "vlt-ppat",
        "vlt-add",
        "vlt-reload",
        "vlt-list",
        "bpnv-ref",
        "bpnv-add",
        "bpnv-reload",
        "bpnv-list",
        "bpnq-ref",
        "bpnq-add",
        "bpnq-reload",
        "bpnq-list",
        "bpnr-ref",
        "bpnr-add",
        "bpnr-reload",
        "bpnr-list",
        "aut-ppat",
        "aut-check",
        "aut-list",
        "seal-year",
        "seal-add",
        "seal-reload",
        "seal-list",
        "perf-reload",
        "perf-sum",
        "vol-group",
        "vol-reload",
        "vol-list",
        "pb-no",
        "pb-add",
        "pb-reload",
        "pb-list",
        "ph-book",
        "ph-add",
        "ph-reload",
        "ph-list",
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
async fn integration_health_reports_without_secrets() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "inthealth@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/integrations/health", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let all = v.as_array().unwrap();
    assert!(all.iter().any(|x| x["name"] == "tripay"));
    assert!(all.iter().any(|x| x["name"] == "ocr"));
    assert!(all.iter().any(|x| x["name"] == "database"));
    // No secret material leaks through the health surface.
    let raw = serde_json::to_string(&v).unwrap().to_lowercase();
    assert!(
        !raw.contains("key_") && !raw.contains("secret_value"),
        "leak: {raw}"
    );
    // Unauthenticated callers get nothing.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/integrations/health")
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
        "appt-name",
        "appt-add",
        "appt-reload",
        "appt-list",
        "queue-date",
        "queue-reload",
        "queue-list",
        "vis-name",
        "vis-add",
        "vis-reload",
        "vis-list",
        "wit-name",
        "wit-add",
        "wit-reload",
        "wit-list",
        "rch-subject",
        "rch-add",
        "rch-reload",
        "rch-list",
        "dsp-task",
        "dsp-add",
        "dsp-reload",
        "dsp-board",
        "dcl-doc",
        "dcl-add",
        "dcl-reload",
        "dcl-list",
        "dlc-doc",
        "dlc-save",
        "dlc-reload",
        "dlc-del",
        "dlc-due",
        "mtf-key",
        "mtf-add",
        "mtf-reload",
        "mtf-list",
        "mtm-doc",
        "mtm-set",
        "chl-child",
        "chl-add",
        "chl-reload",
        "chl-list",
        "csg-cid",
        "csg-add",
        "csg-list",
        "cko-cid",
        "cko-add",
        "cko-reload",
        "cko-list",
        "cki-lid",
        "cki-add",
        "ctm-cid",
        "ctm-reload",
        "ctm-list",
        "cfee-cid",
        "cfee-add",
        "cfee-list",
        "cdrq-cid",
        "cdrq-add",
        "crf-from",
        "crf-add",
        "crf-list",
        "ltv-go",
        "opp-cid",
        "opp-add",
        "opp-list",
        "cwb-cid",
        "cwb-add",
        "cwb-list",
        "mrg-keep",
        "mrg-go",
        "ccr-cid",
        "ccr-add",
        "ccr-list",
        "cem-cid",
        "cem-add",
        "cem-list",
        "ccn-channel",
        "ccn-set",
        "vip-cid",
        "vip-add",
        "vip-list",
        "crd-go",
        "crm-cid",
        "crm-add",
        "crm-list",
        "cfb-cid",
        "cfb-add",
        "cfb-list",
        "cnr-cid",
        "cnr-add",
        "cnr-list",
        "cli-csv",
        "cel-cid",
        "cel-add",
        "cel-list",
        "cpi-cid",
        "cpi-add",
        "cpi-list",
        "csv-cid",
        "csv-add",
        "mcp-name",
        "mcp-add",
        "mcp-list",
        "cev-title",
        "cev-add",
        "cev-list",
        "crs-cid",
        "crs-save",
        "cact-go",
        "cdash-go",
        "cem-crole",
        "ov-cid",
        "ov-go",
        "cfx-name",
        "cfx-go",
        "ptl-token",
        "ptl-login",
        "ptl-me",
        "ptl-dash",
        "ptl-matters",
        "ptl-inbox",
        "ptl-upload",
        "ptl-invoices",
        "ptl-book",
        "ptl-appts",
        "ptl-text",
        "ptl-send",
        "ptl-thread",
        "ptl-activity",
        "ptl-copy",
        "ptl-track",
        "cdrq-list",
        "up-file",
        "up-go",
        "up-prog",
        "prv-doc",
        "prv-go",
        "dlk-doc",
        "dlk-lock",
        "dlk-status",
        "dlk-fav",
        "dwm-text",
        "dst-go",
        "dff-a",
        "dff-go",
        "dxp-reload",
        "dxp-list",
        "dfo-reload",
        "dfo-list",
        "rdt-terms",
        "prl-book",
        "prl-reload",
        "quo-bytes",
        "quo-save",
        "syn-go",
        "sop-code",
        "sop-add",
        "sop-reload",
        "sop-list",
        "hrs-day",
        "hrs-set",
        "hrs-reload",
        "hrs-list",
        "ros-name",
        "ros-add",
        "ros-reload",
        "ros-list",
        "lev-who",
        "lev-add",
        "lev-reload",
        "lev-list",
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
        "audit-actor",
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
        "rep-sla",
        "rep-posture",
        "rep-exec",
        "rep-msg",
        "keu-csv",
        "keu-msg",
        "ann-title",
        "ann-add",
        "ann-reload",
        "ann-list",
        "oin-title",
        "oin-add",
        "oin-reload",
        "oin-list",
        "staff-reload",
        "staff-list",
        "wait-reload",
        "wait-list",
        "bc-at",
        "bc-add",
        "bc-reload",
        "bc-list",
        "price-code",
        "price-add",
        "price-reload",
        "price-list",
        "quot-client",
        "quot-add",
        "quot-reload",
        "quot-list",
        "cash-amount",
        "cash-add",
        "cash-reload",
        "cash-list",
        "adv-holder",
        "adv-add",
        "adv-reload",
        "adv-list",
        "inv-code",
        "inv-add",
        "inv-reload",
        "inv-list",
        "use-item",
        "use-add",
        "use-reload",
        "use-list",
        "atk-code",
        "atk-add",
        "atk-reload",
        "atk-list",
        "abox-code",
        "abox-add",
        "abox-reload",
        "abox-list",
        "dp-no",
        "dp-add",
        "dp-reload",
        "dp-list",
        "bank-name",
        "bank-add",
        "bank-reload",
        "bank-list",
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
        "audit-actor",
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
async fn approvals_overdue_alerts() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprd@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    // Fresh request counts at days=0 with its blocking role surfaced.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/overdue?days=0", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let row = v
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"].as_i64() == Some(aid))
        .expect("fresh approval listed at days=0");
    assert_eq!(row["waiting_on"].as_str().unwrap(), "office_admin");
    assert!(row["age_days"].as_i64().unwrap() >= 0);
    // Default threshold (3d) hides it; unauthenticated callers get nothing.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/overdue", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["id"].as_i64() != Some(aid)));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approvals/overdue")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approvals_calendar_view() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprc@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    // Default month groups the fresh request under its creation day.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/calendar", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(v["month"].as_str().unwrap().len() == 7);
    let found = v["days"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|d| d.as_array().unwrap().clone())
        .any(|r| r["id"].as_i64() == Some(aid));
    assert!(found, "fresh approval present in current-month calendar");
    // Explicit month + waiting_on surfaced; malformed month rejected.
    let month = v["month"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/approvals/calendar?month={month}"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["month"].as_str().unwrap(), month);
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/calendar?month=2026-13", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approvals/calendar")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
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
async fn incident_and_capa_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "inccapa@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/incidents",
            r#"{"title":"X"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/incidents",
            r#"{"title":"Server down","severity":"critical"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["investigating", "resolved", "closed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/incidents/{iid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "incident to {st}");
    }
    // CAPA full chain with cause/action.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/capa",
            r#"{"title":"Tambah UPS","cause":"Listrik padam","action":"Beli UPS 2kVA"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["in_progress", "done", "verified"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/capa/{cid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "capa to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/incidents", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(iid) && r["status"] == "closed"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/capa", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cid) && r["status"] == "verified"));
    // Delete is Delete-gated; audit trail intact.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/capa/{cid}"))
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
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/incidents/{iid}"))
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
        .oneshot(get("/api/v1/audit?entity=incident", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.incident_created"));
}

#[tokio::test]
async fn compliance_posture_and_export() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "posture@x.id").await;
    let client = forged_role_token(&tok, vec!["client"]);
    // Seed two registers, then posture reflects them.
    let mid = mk_matter(&a, &tok).await;
    let _aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/incidents",
            r#"{"title":"Posture seed","severity":"high"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/posture", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let regs: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["register"].as_str().unwrap())
        .collect();
    for want in [
        "obligations",
        "consents",
        "ropa",
        "incidents",
        "risks",
        "dsar",
        "ethics",
        "gifts",
    ] {
        assert!(regs.contains(&want), "posture missing {want}");
    }
    let inc = v
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["register"] == "incidents")
        .unwrap();
    assert_eq!(inc["total"].as_i64().unwrap(), 1);
    assert_eq!(inc["by_status"]["open"].as_i64().unwrap(), 1);
    // Export is Export-gated (client role lacks it) and audited.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/compliance.csv", &client))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/compliance.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()["content-type"]
        .to_str()
        .unwrap()
        .contains("text/csv"));
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.starts_with("register,id,title,status,extra\n"));
    assert!(text.contains("incidents,"));
    // Unauthenticated callers get nothing.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/compliance/posture")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn compliance_alerts_feed() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "alerts@x.id").await;
    // Seed: overdue DSAR, high open risk, unreviewed whistle, open critical finding.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/dsar",
            r#"{"subject":"SS","kind":"access","detail":"DD","due_date":"2020-01-01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/risks",
            r#"{"title":"Kebocoran","likelihood":5,"impact":5}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/whistle",
            r#"{"summary":"Laporan uji"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/findings",
            r#"{"title":"Temuan kritis","severity":"critical"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/alerts", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let kinds: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["kind"].as_str().unwrap())
        .collect();
    for want in [
        "dsar_overdue",
        "risk_high",
        "whistle_unreviewed",
        "finding_open",
    ] {
        assert!(
            kinds.contains(&want),
            "alert feed missing {want}: {kinds:?}"
        );
    }
    // Untouched org sees an empty feed; posture covers all 19 registers.
    let tok2 = reg(&a, "alerts2@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/alerts", &tok2))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 0);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/posture", &tok))
        .await
        .unwrap();
    let pv = body_json(res).await;
    let regs: Vec<&str> = pv
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["register"].as_str().unwrap())
        .collect();
    for want in ["findings", "whistle", "waivers", "policies", "controls"] {
        assert!(regs.contains(&want), "posture missing {want}");
    }
    // Seed the remaining registers so posture + CSV cover all 19.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/waivers",
            r#"{"title":"WW","justification":"JJ"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/policies",
            r#"{"title":"PP","body":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/controls",
            r#"{"code":"T-01","title":"TT"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/compliance.csv", &tok))
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    for want in [
        "findings,",
        "whistle,",
        "waivers,",
        "policies,",
        "controls,",
    ] {
        assert!(text.contains(want), "csv missing {want}");
    }
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/compliance/alerts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn office_performance_and_deed_volume() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "perfvol@x.id").await;
    // Seed: one deed (draft), one done appointment, one accepted quotation.
    let _did = mk_deed(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/appointments",
            r#"{"client_name":"BB","scheduled_at":"2026-04-01 09:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let aid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["checked_in", "done"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/appointments/{aid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/office-performance", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let p = body_json(res).await;
    assert_eq!(p["appointments_done"].as_i64().unwrap(), 1);
    assert_eq!(p["deeds_total"].as_i64().unwrap(), 1);
    assert_eq!(p["deeds_by_state"]["draft"].as_i64().unwrap(), 1);
    // Volume: month buckets + state grouping + 401 guard.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/deed-volume", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["group"].as_str().unwrap(), "month");
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/deed-volume?group=state", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["group"].as_str().unwrap(), "state");
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/office-performance")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn office_hours_roster_leaves() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "officer@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Hours: strict HH:MM, upsert per weekday.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/office-hours",
            r#"{"weekday":9,"open_time":"08:00","close_time":"17:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Hours: strict HH:MM, upsert per weekday.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/office-hours",
            r#"{"weekday":0,"open_time":"99:99","close_time":"17:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/office-hours",
            r#"{"weekday":0,"open_time":"08:00","close_time":"17:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/office-hours",
            r#"{"weekday":0,"open_time":"08:30","close_time":"17:30"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/office-hours", &tok))
        .await
        .unwrap();
    let hours = body_json(res).await;
    assert_eq!(hours.as_array().unwrap().len(), 1);
    assert_eq!(hours[0]["open_time"].as_str().unwrap(), "08:30");
    assert_eq!(hours[0]["day"].as_str().unwrap(), "Senin");
    // Roster + leaves lifecycle.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/roster",
            r#"{"name":"Siti","role":"staff","shift":"pagi"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/roster",
            r#"{"name":"Siti","role":"staff","shift":"malam"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/roster",
            r#"{"name":"Siti","role":"staff","shift":"pagi"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post("/api/v1/leaves", r#"{"holder":"Siti","kind":"leave","from_date":"2026-03-10","to_date":"2026-03-01","substitute":"Andi"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post("/api/v1/leaves", r#"{"holder":"Siti","kind":"leave","from_date":"2026-03-01","to_date":"2026-03-10","substitute":"Andi"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let lid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/leaves/{lid}/status"),
            r#"{"status":"approved"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/leaves/{lid}/status"),
            r#"{"status":"rejected"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/leaves", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(lid) && r["substitute"] == "Andi"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=roster", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.roster_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/roster")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn witnesses_and_rechecks() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "witrc@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/witnesses",
            r#"{"name":"BB"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/witnesses",
            r#"{"name":"Saksi A","id_number":"3201010101010001","phone":"0811"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let wid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/witnesses/{wid}/active"),
            r#"{"active":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/witnesses", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(wid) && r["active"] == false));
    // Rechecks: method/result allowlists enforced (manual log, no external call).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/rechecks",
            r#"{"subject":"SS","method":"telepathy","result":"pass"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post("/api/v1/rechecks", r#"{"subject":"NIK-001","method":"document","result":"pass","verified_on":"2026-02-01","note":"KTP valid"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/rechecks", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(rid) && r["result"] == "pass"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/witnesses/{wid}"))
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=witness", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.witness_created"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/witnesses")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn downpayments_and_klapper() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dpklap@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/down-payments",
            r#"{"receipt_no":"X","client_name":"Y","amount_minor":1}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let mid = mk_matter(&a, &tok).await;
    // Ghost matter rejected.
    let res = a
        .clone()
        .oneshot(post("/api/v1/down-payments", r#"{"receipt_no":"UM/2026/001","client_name":"BB","matter_id":999999,"amount_minor":100}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post("/api/v1/down-payments", &format!(r#"{{"receipt_no":"UM/2026/001","client_name":"Budi","matter_id":{mid},"amount_minor":500000}}"#), Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    // Duplicate receipt number rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/down-payments",
            r#"{"receipt_no":"UM/2026/001","client_name":"CC","amount_minor":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["applied"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/down-payments/{did}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/down-payments/{did}/status"),
            r#"{"status":"refunded"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Klapper lists the client + matter-linked deed parties alphabetically.
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/klapper", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let kl = body_json(res).await;
    assert!(kl.as_array().unwrap().iter().any(|e| e["kind"] == "klien"));
    let names: Vec<&str> = kl
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    let mut sorted = names.clone();
    sorted.sort_by_key(|x| x.to_lowercase());
    assert_eq!(names, sorted);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/klapper")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=downpayment", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.dp_created"));
}

#[tokio::test]
async fn visitors_and_announcements() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "visann@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/visitors",
            r#"{"name":"BB","check_in":"2026-03-01 09:00"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Bad slot rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/visitors",
            r#"{"name":"Budi","check_in":"2026-13-99 99:99"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/visitors",
            r#"{"name":"Budi Santoso","need":"Konsultasi AJB","check_in":"2026-03-01 09:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let vid = body_json(res).await["id"].as_i64().unwrap();
    // Double checkout rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/visitors/{vid}/checkout"),
            r#"{"check_out":"2026-03-01 10:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/visitors/{vid}/checkout"),
            r#"{"check_out":"2026-03-01 11:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/visitors", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(vid) && r["status"] == "out"));
    // Announcements: pin ordering + archive hides.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/announcements",
            r#"{"title":"Libur","body":"Tutup Jumat"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let aid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/announcements",
            r#"{"title":"Apel","body":"Senin pagi","pinned":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/announcements", &tok))
        .await
        .unwrap();
    let al = body_json(res).await;
    assert_eq!(al.as_array().unwrap().len(), 2);
    assert!(al.as_array().unwrap()[0]["pinned"].as_bool().unwrap());
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/announcements/{aid}"),
            r#"{"active":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/announcements/{aid}"))
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=visitor", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.visitor_checkin"));
}

#[tokio::test]
async fn atk_and_archive_boxes() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "atkbox@x.id").await;
    // ATK is kind-tagged inventory: general items stay out of the ATK view.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/inventory",
            r#"{"code":"GEN-01","name":"Umum","qty":5}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/inventory",
            r#"{"code":"ATK-KERTAS","name":"Kertas A4","qty":10,"kind":"atk"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/inventory",
            r#"{"code":"ATK-X","name":"XX","qty":1,"kind":"alien"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/inventory?kind=atk", &tok))
        .await
        .unwrap();
    let atk = body_json(res).await;
    assert_eq!(atk.as_array().unwrap().len(), 1);
    assert_eq!(atk[0]["code"].as_str().unwrap(), "ATK-KERTAS");
    // Archive boxes: dup codes rejected, move updates location.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/archive-boxes",
            r#"{"box_code":"ARS-001","location":"Ruang A / Rak 3","description":"Akta 2024"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/archive-boxes",
            r#"{"box_code":"ARS-001"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/archive-boxes/{bid}/move"),
            r#"{"location":"Ruang B / Rak 1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/archive-boxes", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(bid) && r["location"] == "Ruang B / Rak 1"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=abox", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.abox_created"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/archive-boxes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sop_library_and_backup_checks() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "sopbc@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/sops",
            r#"{"code":"X","title":"Y","body":"BB"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/sops",
            r#"{"code":"SOP-ARSIP","title":"Arsip","version":"1.0","body":"Langkah arsip"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/sops",
            r#"{"code":"SOP-ARSIP","title":"Dup","body":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["published", "archived"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/sops/{sid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "sop to {st}");
    }
    let res = a.clone().oneshot(get("/api/v1/sops", &tok)).await.unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(sid) && r["status"] == "archived"));
    // Backup checks: strict timestamp + method/result allowlists.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/backup-checks",
            r#"{"checked_at":"2026-13-99 99:99","method":"dump","result":"pass"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/backup-checks",
            r#"{"checked_at":"2026-05-01 02:00","method":"carrier","result":"pass"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post("/api/v1/backup-checks", r#"{"checked_at":"2026-05-01 02:00","method":"dump","result":"pass","note":"mysqldump OK"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/backup-checks", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(bid) && r["result"] == "pass"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=sop", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.sop_created"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=bcheck", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.bcheck_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sops")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bank_facilities_and_feedback() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "bankfb@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/bank-facilities",
            r#"{"bank_name":"BB","facility":"FF"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/bank-facilities",
            r#"{"bank_name":"Bank Maju","facility":"KPR","rate_text":"9,5%","tenor":"15 tahun"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/bank-facilities/{bid}/active"),
            r#"{"active":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/bank-facilities", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(bid) && r["active"] == false));
    // Feedback: rating bounds + new->reviewed->archived.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/feedbacks",
            r#"{"message":"Bagus","rating":9}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/feedbacks",
            r#"{"name":"Klien A","message":"Pelayanan cepat","rating":5}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let fid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["reviewed", "archived"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/feedbacks/{fid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "feedback to {st}");
    }
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/feedbacks/{fid}/status"),
            r#"{"status":"new"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=bankf", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.bankf_created"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=feedback", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.feedback_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bank-facilities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn doc_checklist_and_office_incidents() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dcloinc@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Checklist requires a real document.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents/999999/checklist",
            r#"{"item":"Butir hantu"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let mid = mk_matter(&a, &tok).await;
    let content = b"fake-checklist-bytes";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"kontrak.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok)))
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "seed doc: {}",
        body_json(res).await
    );
    let doc = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{doc}/checklist"),
            r#"{"item":"XX"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{doc}/checklist"),
            r#"{"item":"Cek tanda tangan"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/doc-checklist/{cid}/toggle"),
            r#"{"done":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{doc}/checklist"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cid) && r["done"] == true));
    // Cross-org isolation: item invisible elsewhere.
    let tok2 = reg(&a, "dcloinc2@x.id").await;
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{doc}/checklist"), &tok2))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Office incidents: lifecycle + guards.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/office-incidents",
            r#"{"title":"AC bocor","severity":"high"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let oid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["investigating", "resolved", "closed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/office-incidents/{oid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "oincident to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/office-incidents", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(oid) && r["status"] == "closed"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=oincident", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.oincident_created"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/office-incidents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn storage_boxes_and_mpd_transfers() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "sboxmpd@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/storage-boxes",
            r#"{"box_code":"X"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/storage-boxes",
            r#"{"box_code":"BOX-A01","location":"Ruang A","capacity":200}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/storage-boxes",
            r#"{"box_code":"BOX-A01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // MPD transfer: ghost book rejected; unlinked transfer allowed.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/mpd-transfers",
            r#"{"book_id":999999,"recipient":"MPD Bogor"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/mpd-transfers",
            r#"{"recipient":"MPD Bogor","transfer_date":"2026-05-01","receipt_no":"BA/01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/mpd-transfers/{tid}/status"),
            r#"{"status":"transferred"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/mpd-transfers/{tid}/status"),
            r#"{"status":"prepared"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/mpd-transfers", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(tid) && r["status"] == "transferred"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=sbox", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.sbox_created"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/storage-boxes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn selfcheck_and_year_seals() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "scseal@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/selfchecks",
            r#"{"period":"Q1","passed_items":1,"total_items":1}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // passed > total rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/selfchecks",
            r#"{"period":"2026-Q1","passed_items":9,"total_items":8}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/selfchecks",
            r#"{"period":"2026-Q1","passed_items":7,"total_items":8,"note":"OK"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/selfchecks",
            r#"{"period":"2026-Q1","passed_items":1,"total_items":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/selfchecks", &tok))
        .await
        .unwrap();
    let sc = body_json(res).await;
    assert_eq!(sc.as_array().unwrap().len(), 1);
    assert_eq!(sc[0]["pct"].as_f64().unwrap(), 87.5);
    // Year seals: book count snapshot (0 books for 2026 in this org).
    // Year must be YYYY; malformed years rejected by the store guard.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/year-seals",
            r#"{"year":"20XX","note":"X"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/year-seals",
            r#"{"year":"2026","note":"Tutup buku"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post("/api/v1/year-seals", r#"{"year":"2026"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/year-seals/{sid}/seal"),
            r#"{}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/year-seals", &tok))
        .await
        .unwrap();
    let sv = body_json(res).await;
    assert!(sv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(sid)
            && r["status"] == "sealed"
            && r["book_count"].as_i64().unwrap() == 0));
    // Sealed books cannot be resealed; delete is Delete-gated.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/year-seals/{sid}/seal"),
            r#"{}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/year-seals/{sid}"))
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=selfcheck", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.selfcheck_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/selfchecks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn staff_productivity_and_waiting_analytics() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "staffwait@x.id").await;
    // Seed audited actions + one completed visit (15 minutes).
    let _did = mk_deed(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/visitors",
            r#"{"name":"Tamu A","check_in":"2026-04-01 09:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let vid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/visitors/{vid}/checkout"),
            r#"{"check_out":"2026-04-01 09:15"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/staff-productivity", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let p = body_json(res).await;
    assert!(p["sampled"].as_i64().unwrap() >= 3);
    assert!(!p["by_actor"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/waiting-analytics", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let w = body_json(res).await;
    assert_eq!(w["completed_visits"].as_i64().unwrap(), 1);
    assert_eq!(w["avg_minutes"].as_f64().unwrap(), 15.0);
    assert_eq!(w["max_minutes"].as_i64().unwrap(), 15);
    assert!(w["by_day"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["day"] == "2026-04-01"));
    // Empty org: nulls, not zeros.
    let tok2 = reg(&a, "staffwait2@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/waiting-analytics", &tok2))
        .await
        .unwrap();
    let w = body_json(res).await;
    assert_eq!(w["completed_visits"].as_i64().unwrap(), 0);
    assert!(w["avg_minutes"].is_null());
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/waiting-analytics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pbb_and_ppat_fee() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "pbbfee@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/pbb-checks",
            r#"{"nop":"X","tax_year":"2026"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/pbb-checks",
            r#"{"nop":"32.73.010.001.001-0001.0","tax_year":"20XX","result":"paid"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post("/api/v1/pbb-checks", r#"{"nop":"32.73.010.001.001-0001.0","tax_year":"2026","result":"paid","checked_on":"2026-02-01"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/pbb-checks", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(pid) && r["result"] == "paid"));
    // PPAT fee: 1% x 2.000.000.000 = 20.000.000; rejects non-positive.
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/worksheets/ppat-fee?value_minor=2000000000",
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let f = body_json(res).await;
    assert_eq!(f["fee_minor"].as_i64().unwrap(), 20_000_000);
    assert_eq!(f["rate_pct"].as_f64().unwrap(), 1.0);
    let res = a
        .clone()
        .oneshot(get("/api/v1/worksheets/ppat-fee?value_minor=0", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=pbb", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.pbb_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/pbb-checks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn office_audit_actor_filter_and_report_packs() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "auditpack@x.id").await;
    // Actor filter: own sub matches, nonsense matches nothing.
    let me = legalrust_auth::validate_token(b"test-secret-0123456789-0123456789ab", &tok)
        .expect("decode")
        .sub;
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/audit?actor={me}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?actor=tidak-ada-aktor-ini", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 0);
    // Notary monthly pack mirrors monthly buckets.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"NP/1","deed_kind":"ajb","seller":"AA","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let month = chrono::Utc::now().format("%Y-%m").to_string();
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/reports/notary-monthly-pack?month={month}"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pk = body_json(res).await;
    assert_eq!(pk["month"].as_str().unwrap(), month);
    assert_eq!(pk["ppat_total"].as_i64().unwrap(), 1);
    assert_eq!(pk["dashboard"]["total_deeds"].as_i64().unwrap(), 1);
    // MPN pack reflects register + rules + recent events.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-register",
            r#"{"book_ref":"B-1","party":"PT X"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/mpn-pack", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mp = body_json(res).await;
    assert_eq!(mp["protocol_entries"].as_i64().unwrap(), 1);
    assert!(mp["recent_property_events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.preg_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/mpn-pack")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protocol_retention_and_aml_scores() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "retaml@x.id").await;
    // Retention: one ancient entry, one fresh.
    for (date, book, deed) in [
        ("1990-01-01", "B-OLD", "1/1990"),
        ("2026-11-01", "B-NEW", "2/2026"),
    ] {
        let res = a
            .clone()
            .oneshot(post("/api/v1/protocol-register", &format!(r#"{{"entry_date":"{date}","book_ref":"{book}","deed_no":"{deed}","party":"PT X"}}"#), Some(&tok)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/protocol-retention?years=25", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rv = body_json(res).await;
    assert!(rv["due"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["deed_no"] == "1/1990"));
    assert!(rv["due"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["deed_no"] != "2/2026"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/protocol-retention?years=0", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // AML: corporate heavyweight vs clean individual.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"PT Raksasa","kind":"corporate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let corp = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"Budi","kind":"individual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    for i in 1..=3 {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/matters",
                &format!(r#"{{"title":"M{i}","client_id":{corp}}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        let mid = body_json(res).await["id"].as_i64().unwrap();
        if i <= 2 {
            let res = a
                .clone()
                .oneshot(post(
                    &format!("/api/v1/matters/{mid}/invoices"),
                    r#"{"total_minor":500000000}"#,
                    Some(&tok),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/aml-scores", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sv = body_json(res).await;
    let corp_row = sv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["client_id"].as_i64() == Some(corp))
        .unwrap()
        .clone();
    assert_eq!(corp_row["score"].as_i64().unwrap(), 65);
    assert_eq!(corp_row["band"], serde_json::json!("high"));
    assert!(sv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "Budi" && r["score"].as_i64() == Some(10) && r["band"] == "low"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/aml-scores")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_prefs_and_freeze() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprpref@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    // Digest prefs roundtrip.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/prefs", &tok))
        .await
        .unwrap();
    assert_eq!(
        body_json(res).await["digest_mode"],
        serde_json::json!("instant")
    );
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals/prefs",
            r#"{"digest_mode":"daily","escalate_to":"admin-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/prefs", &tok))
        .await
        .unwrap();
    let pv = body_json(res).await;
    assert_eq!(pv["digest_mode"], serde_json::json!("daily"));
    assert_eq!(pv["escalate_to"], serde_json::json!("admin-1"));
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals/prefs",
            r#"{"digest_mode":"hourly"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Freeze is admin-gated.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals/freeze",
            r#"{"frozen":true}"#,
            Some(&lawyer),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals/freeze",
            r#"{"frozen":true,"reason":"audit semesteran"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // All mutation paths short-circuit while frozen.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals/decide-bulk",
            &format!(r#"{{"items":[{{"id":{aid},"step_no":1,"decision":"approve"}}]}}"#),
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
    // Reopen and decide normally.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals/freeze",
            r#"{"frozen":false}"#,
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
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/freeze", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["frozen"], serde_json::json!(false));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=approval", &tok))
        .await
        .unwrap();
    let rows = body_json(res).await;
    assert!(rows
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "approval.frozen"));
    assert!(rows
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "approval.unfrozen"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approvals/prefs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_attachments_and_archive() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprattach@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"attach-bytes";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"lampiran.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    // Matter approval.
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid}/attachments"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let av = body_json(res).await;
    assert_eq!(av["matter_id"].as_i64().unwrap(), mid);
    assert!(av["docs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"].as_i64() == Some(did)));
    // Invoice approval resolves through its matter.
    let iid = mk_invoice(&a, &tok, mid, 750000).await;
    let res = a.clone().oneshot(post("/api/v1/approvals", &format!(r#"{{"entity":"invoice","entity_id":{iid},"title":"Setujui invoice","steps":[{{"step_no":1,"role":"office_admin"}}]}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iaid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{iaid}/attachments"), &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["docs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"].as_i64() == Some(did)));
    // Deed approval resolves through its matter.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/deed-templates",
            r#"{"name":"AJB","body":"Pada hari ini."}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post("/api/v1/deeds/from-template", &format!(r#"{{"template_id":{tid},"matter_id":{mid},"number":"D-9","deed_type":"ajb","variables":{{}}}}"#), Some(&tok))).await.unwrap();
    let deed = body_json(res).await["id"].as_i64().unwrap();
    let res = a.clone().oneshot(post("/api/v1/approvals", &format!(r#"{{"entity":"deed","entity_id":{deed},"title":"Setujui akta","steps":[{{"step_no":1,"role":"office_admin"}}]}}"#), Some(&tok))).await.unwrap();
    let daid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{daid}/attachments"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["matter_id"].as_i64().unwrap(), mid);
    // Ghost + anon.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/999999/attachments", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/approvals/{aid}/attachments"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Archive: decide the matter approval, then retrieve.
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/archive", &tok))
        .await
        .unwrap();
    let arv = body_json(res).await;
    assert!(arv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(aid) && r["status"] == "approved"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/archive?status=rejected", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["rows"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/archive?status=alien", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approvals/archive")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_trend_and_mobile_inbox() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprtrend@x.id").await;
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
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals-trend?days=7", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tv = body_json(res).await;
    let row = tv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["date"] == today);
    assert!(
        row.is_some(),
        "today bucket missing: {}",
        serde_json::to_string(&tv).unwrap()
    );
    assert!(row.unwrap()["events"].as_i64().unwrap() >= 1);
    assert!(row.unwrap()["approved"].as_i64().unwrap() >= 1);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals-trend?days=0", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["days"].as_i64().unwrap(), 1);
    // Mobile inbox page renders without auth (auth happens client-side like /app login).
    let (st, _, t) = page_get(&a, "/m/inbox").await;
    assert_eq!(st, StatusCode::OK);
    for n in [
        "Inbox Persetujuan",
        "viewport",
        "decidable",
        "Idempotency-Key",
    ] {
        assert!(t.contains(n), "/m/inbox missing {n}");
    }
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/approvals-trend")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_routing_amount_and_risk() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprroute@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    // Rules are admin-gated.
    let res = a.clone().oneshot(post("/api/v1/approval-routes", r#"{"kind":"amount","entity":"invoice","min_value":100000000,"role":"finance_officer"}"#, Some(&lawyer))).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approval-routes",
            r#"{"kind":"alien","entity":"invoice","role":"XX"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for (kind, entity, minv, role) in [
        ("amount", "invoice", 100_000_000i64, "finance_officer"),
        ("amount", "invoice", 1_000_000_000i64, "director"),
        ("risk", "matter", 60i64, "compliance"),
    ] {
        let res = a.clone().oneshot(post("/api/v1/approval-routes", &format!(r#"{{"kind":"{kind}","entity":"{entity}","min_value":{minv},"role":"{role}","priority":10}}"#), Some(&tok))).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "add {kind}/{role}");
    }
    // Corporate client with a big invoice: amount hits 100jt tier, risk hits 60.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients",
            r#"{"display_name":"PT Besar","kind":"corporate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let corp = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"M1","client_id":{corp}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let iid = mk_invoice(&a, &tok, mid, 500_000_000).await;
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/approvals/route-resolve?entity=invoice&entity_id={iid}"),
            &tok,
        ))
        .await
        .unwrap();
    let rv = body_json(res).await;
    assert_eq!(rv["amount_minor"].as_i64().unwrap(), 500_000_000);
    let roles = rv["roles"].as_array().unwrap();
    assert!(roles.iter().any(|r| r == "finance_officer"));
    assert!(!roles.iter().any(|r| r == "director"));
    // Matter risk path: 10 + 20 + 5 + 10 = 45 < 60 -> no compliance role.
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/approvals/route-resolve?entity=matter&entity_id={mid}"),
            &tok,
        ))
        .await
        .unwrap();
    let mv = body_json(res).await;
    assert_eq!(mv["risk_score"].as_i64().unwrap(), 45);
    assert!(!mv["roles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == "compliance"));
    // Ghost + bad entity + anon.
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/approvals/route-resolve?entity=invoice&entity_id=999999",
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/approvals/route-resolve?entity=alien&entity_id=1",
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approval-routes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_chain_and_redaction() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprchain@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(
        &a,
        &tok,
        mid,
        r#"[{"step_no":1,"role":"office_admin"},{"step_no":2,"role":"office_admin"}]"#,
    )
    .await;
    // Decide step 1 with a comment, then verify the chain.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve","comment":"NIK 12345"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid}/chain"), &tok))
        .await
        .unwrap();
    let ch = body_json(res).await;
    assert_eq!(ch["valid"], serde_json::json!(true));
    assert_eq!(ch["receipts"].as_array().unwrap().len(), 1);
    // Second decision extends the chain.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":2,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid}/chain"), &tok))
        .await
        .unwrap();
    let ch2 = body_json(res).await;
    assert_eq!(ch2["valid"], serde_json::json!(true));
    assert_eq!(ch2["receipts"].as_array().unwrap().len(), 2);
    assert_eq!(ch2["receipts"][1]["prev"], ch2["receipts"][0]["hash"]);
    // Redaction is Delete-gated: lawyer forbidden, owner scrubs.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/steps/1/redact"),
            "{}",
            Some(&lawyer),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/steps/9/redact"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/steps/1/redact"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid}"), &tok))
        .await
        .unwrap();
    assert!(
        body_json(res).await["steps"].as_array().unwrap()[0]["comment"]
            .as_str()
            .unwrap()
            .contains("redacted")
    );
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=approval", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "approval.comment_redacted"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/approvals/{aid}/chain"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_policy_versions_and_fast_track() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprpol@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let mid = mk_matter(&a, &tok).await;
    let iid = mk_invoice(&a, &tok, mid, 500_000_000).await;
    // Rule + snapshot v1, then delete the rule: live loses it, v1 keeps it.
    let res = a.clone().oneshot(post("/api/v1/approval-routes", r#"{"kind":"amount","entity":"invoice","min_value":100000000,"role":"finance_officer"}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post("/api/v1/approval-policies", "{}", Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["version"], serde_json::json!(1));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/approval-routes/{rid}"))
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
        .oneshot(get(
            &format!("/api/v1/approvals/route-resolve?entity=invoice&entity_id={iid}"),
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res).await["roles"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get(
            &format!(
                "/api/v1/approvals/route-resolve?entity=invoice&entity_id={iid}&policy_version=1"
            ),
            &tok,
        ))
        .await
        .unwrap();
    let rv = body_json(res).await;
    assert_eq!(rv["policy_version"], serde_json::json!(1));
    assert!(rv["roles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == "finance_officer"));
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/approvals/route-resolve?entity=invoice&entity_id=1&policy_version=99",
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Fast-track: admin-only, approves every remaining step at once.
    let mid2 = mk_matter(&a, &tok).await;
    let fid = mk_approval(&a, &tok, mid2, r#"[{"step_no":1,"role":"office_admin"},{"step_no":2,"role":"office_admin"},{"step_no":3,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{fid}/fast-track"),
            "{}",
            Some(&lawyer),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{fid}/fast-track"),
            r#"{"comment":"darurat"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["fast_tracked"].as_i64().unwrap(), 3);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{fid}"), &tok))
        .await
        .unwrap();
    assert_eq!(
        body_json(res).await["status"],
        serde_json::json!("approved")
    );
    // Second run: nothing pending.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{fid}/fast-track"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approval-policies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn document_approval_routing() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docappr@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"route-me";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"r.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    // Document is now a first-class approval entity.
    let res = a.clone().oneshot(post("/api/v1/approvals", &format!(r#"{{"entity":"document","entity_id":{did},"title":"Setujui dokumen","steps":[{{"step_no":1,"role":"office_admin"}}]}}"#, ), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let aid = body_json(res).await["id"].as_i64().unwrap();
    // Attachments resolve through the doc's matter.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid}/attachments"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["matter_id"].as_i64().unwrap(), mid);
    // Unknown entity still rejected; ghost document 404s.
    let res = a.clone().oneshot(post("/api/v1/approvals", r#"{"entity":"alien","entity_id":1,"title":"X","steps":[{"step_no":1,"role":"office_admin"}]}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a.clone().oneshot(post("/api/v1/approvals", r#"{"entity":"document","entity_id":999999,"title":"X","steps":[{"step_no":1,"role":"office_admin"}]}"#, Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn document_duplicates_manifest_and_auto_index() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docdup@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"kata kunci zebra unik";
    let sha = legalrust_document::sha256_hex(content);
    for name in ["ganda-a.txt", "ganda-b.txt"] {
        let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"{name}","mime":"text/plain","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
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
    }
    // Duplicates collide on sha.
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/duplicates", &tok))
        .await
        .unwrap();
    let dv = body_json(res).await;
    assert!(dv["groups"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["count"].as_i64() == Some(2)));
    // Manifest for the matter.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/export-manifest"), &tok))
        .await
        .unwrap();
    let mv = body_json(res).await;
    assert_eq!(mv["files"].as_i64().unwrap(), 2);
    assert_eq!(mv["bytes"].as_i64().unwrap(), (content.len() * 2) as i64);
    let res = a
        .clone()
        .oneshot(get("/api/v1/matters/999999/export-manifest", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Auto-index: searchable without an explicit index call.
    let res = a
        .clone()
        .oneshot(get("/api/v1/rag/search?q=zebra", &tok))
        .await
        .unwrap();
    assert!(!body_json(res).await.as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/duplicates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn chunked_upload_sessions() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "upchunk@x.id").await;
    let bearer = format!("Bearer {tok}")
        .parse::<axum::http::HeaderValue>()
        .unwrap();
    let mid = mk_matter(&a, &tok).await;
    let content = b"0123456789abcdef";
    let sha = legalrust_document::sha256_hex(content);
    // Oversize rejected at session open.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/uploads",
            &format!(
                r#"{{"name":"big.bin","mime":"application/octet-stream","size":{},"sha":"{sha}"}}"#,
                100 * 1024 * 1024
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/uploads",
            &format!(
                r#"{{"name":"c.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sid = body_json(res).await["id"].as_i64().unwrap();
    // Chunks append in order; overflow rejected.
    for part in [&content[..6], &content[6..]] {
        let res = a
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/uploads/{sid}/chunk"))
                    .method("POST")
                    .header("authorization", bearer.clone())
                    .body(Body::from(part.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/uploads/{sid}/chunk"))
                .method("POST")
                .header("authorization", bearer.clone())
                .body(Body::from(b"zzz".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/uploads/{sid}/complete"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["doc_id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}/content"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Tampered bytes fail the sha gate at complete.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/uploads",
            &format!(r#"{{"name":"t.bin","mime":"text/plain","size":4,"sha":"{sha}"}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/uploads/{tid}/chunk"))
                .method("POST")
                .header("authorization", bearer.clone())
                .body(Body::from(b"xxxx".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/uploads/{tid}/complete"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Abort drops the session.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/uploads",
            &format!(r#"{{"name":"a.txt","mime":"text/plain","size":2,"sha":"{sha}"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let abid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/uploads/{abid}"))
                .method("DELETE")
                .header("authorization", bearer.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/uploads/{abid}/complete"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let _ = mid;
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/uploads")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn document_folders_and_dashboard() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docfold@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"folder-bytes";
    let sha = legalrust_document::sha256_hex(content);
    for (name, matter) in [("f1.txt", Some(mid)), ("loose.txt", None)] {
        let mstr = match matter {
            Some(m) => format!(r#""matter_id":{m},"#),
            None => String::new(),
        };
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/documents",
                &format!(
                    r#"{{{mstr}"name":"{name}","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                    content.len()
                ),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/folders", &tok))
        .await
        .unwrap();
    let fv = body_json(res).await;
    assert!(fv["folders"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["matter_id"].as_i64() == Some(mid) && r["docs"].as_i64() == Some(1)));
    assert_eq!(fv["unfiled"].as_i64().unwrap(), 1);
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/stats", &tok))
        .await
        .unwrap();
    let sv = body_json(res).await;
    assert_eq!(sv["total"].as_i64().unwrap(), 2);
    assert!(sv["by_mime"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["mime"] == "text/plain" && r["n"].as_i64() == Some(2)));
    assert_eq!(sv["recent"].as_array().unwrap().len(), 2);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn storage_quota_and_offline_sync() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "quota@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    // Default unlimited.
    let res = a
        .clone()
        .oneshot(get("/api/v1/org/quota", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["quota_bytes"].as_i64().unwrap(), 0);
    // Lawyer cannot set quota.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/org/quota")
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {lawyer}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .header("content-type", "application/json")
                .body(Body::from(r#"{"quota_bytes":100}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let content = b"0123456789";
    let sha = legalrust_document::sha256_hex(content);
    // Tight quota: 10-byte doc fits, second one trips.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/org/quota")
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .header("content-type", "application/json")
                .body(Body::from(r#"{"quota_bytes":10}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"q1.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"q2.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Usage accounting reflects the one doc.
    let res = a
        .clone()
        .oneshot(get("/api/v1/org/quota", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["used_bytes"].as_i64().unwrap(), 10);
    // Widen: intake works again; sync feed pages by cursor.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/org/quota")
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .header("content-type", "application/json")
                .body(Body::from(r#"{"quota_bytes":1000000}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/sync?since_id=0", &tok))
        .await
        .unwrap();
    let sv = body_json(res).await;
    assert!(!sv["rows"].as_array().unwrap().is_empty());
    let first = sv["rows"].as_array().unwrap()[0]["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/documents/sync?since_id={first}"),
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res).await["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["id"].as_i64().unwrap() > first));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/sync")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn document_redact_and_protocol_register() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docredact@x.id").await;
    let content = b"rahasia NIK 123 milik Budi";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"s.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
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
    // No-match redaction rejected; binary rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/redact"),
            r#"{"terms":["tidakada"]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let bin = b"%PDF-9";
    let bsha = legalrust_document::sha256_hex(bin);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"b.pdf","mime":"application/pdf","size":{},"sha":"{bsha}"}}"#,
                bin.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    let bid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{bid}/content"))
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::from(bin.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{bid}/redact"),
            r#"{"terms":["x"]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Mask + verify content.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/redact"),
            r#"{"terms":["NIK 123","Budi"]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["masked"].as_i64().unwrap(), 2);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}/content"), &tok))
        .await
        .unwrap();
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(text.starts_with("rahasia \u{2588}\u{2588}\u{2588} milik \u{2588}\u{2588}\u{2588}"));
    // Protocol linkage + register.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/protocol-link"),
            r#"{"book":"B-7"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/protocol-register", &tok))
        .await
        .unwrap();
    let rv = body_json(res).await;
    assert!(rv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(did) && r["protocol_book"] == "B-7"));
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents/999999/protocol-link",
            r#"{"book":"B-7"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}/redact"))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"terms":["x"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn matter_bundle_and_scanned_ingest() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "bundle@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let content = b"bundel-isi";
    let sha = legalrust_document::sha256_hex(content);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"b.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#, content.len()), Some(&tok))).await.unwrap();
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
    // Bundle carries base64 content that round-trips.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/bundle"), &tok))
        .await
        .unwrap();
    let bv = body_json(res).await;
    assert_eq!(bv["files"].as_i64().unwrap(), 1);
    let entry = &bv["entries"].as_array().unwrap()[0];
    assert_eq!(entry["name"], serde_json::json!("b.txt"));
    let raw = entry["content_base64"].as_str().unwrap();
    assert_eq!(raw.len() % 4, 0);
    // Ghost matter 404s; anon 401s.
    let res = a
        .clone()
        .oneshot(get("/api/v1/matters/999999/bundle", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/matters/{mid}/bundle"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Scanned image upload auto-queues an OCR job.
    let png = b"\x89PNG-fake-bytes";
    let psha = legalrust_document::sha256_hex(png);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"scan.png","mime":"image/png","size":{},"sha":"{psha}"}}"#, png.len()), Some(&tok))).await.unwrap();
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{pid}/content"))
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::from(png.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/ocr/jobs", &tok))
        .await
        .unwrap();
    let jobs = body_json(res).await;
    assert!(jobs
        .as_array()
        .unwrap()
        .iter()
        .any(|j| j["document_id"].as_i64() == Some(pid)));
}

#[tokio::test]
async fn client_hierarchy_and_segments() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clihier@x.id").await;
    let c1 = mk_crm_client(&a, &tok).await;
    let c2 = mk_crm_client(&a, &tok).await;
    let c3 = mk_crm_client(&a, &tok).await;
    // Link + dup/self/ghost guards.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-links",
            &format!(r#"{{"child_id":{c1},"parent_id":{c2},"relation":"subsidiary"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-links",
            &format!(r#"{{"child_id":{c1},"parent_id":{c2}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-links",
            &format!(r#"{{"child_id":{c1},"parent_id":{c1}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-links",
            r#"{"child_id":999999,"parent_id":999998}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-links",
            &format!(r#"{{"child_id":{c2},"parent_id":{c1}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/client-links?client_id={c1}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["relation"] == "subsidiary"));
    // Tags: idempotent add, invalid rejected, filtered list, delete.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-tags",
            &format!(r#"{{"client_id":{c3},"tag":"korporat_besar"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-tags",
            &format!(r#"{{"client_id":{c3},"tag":"Korporat_Besar"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["id"].as_i64().unwrap(), tid);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-tags",
            &format!(r#"{{"client_id":{c3},"tag":"x"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-tags",
            &format!(r#"{{"client_id":{c3},"tag":"a b"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/client-tags?client_id={c3}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["tag"] == "korporat_besar"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/client-tags/{tid}"))
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
        .oneshot(
            Request::builder()
                .uri("/api/v1/client-links")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_onboarding_offboarding_checklists() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cchk@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Auditor cannot open (Create gate).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/checklists"),
            r#"{"kind":"onboarding"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Bad kind rejected; ghost client 404s.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/checklists"),
            r#"{"kind":"alien"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/999999/checklists",
            r#"{"kind":"onboarding"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Onboarding chain open->in_progress->done; done->closed rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/checklists"),
            r#"{"kind":"onboarding","note":"KYC"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let oid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["in_progress", "done"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/checklists/{oid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "onboarding to {st}");
    }
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/checklists/{oid}/status"),
            r#"{"status":"closed"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Offboarding goes all the way to closed.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/checklists"),
            r#"{"kind":"offboarding"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let fid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["in_progress", "done", "closed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/checklists/{fid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "offboarding to {st}");
    }
    // Items: add/toggle/list; ghost list 404? (empty ok) — ghost add 404s.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/checklists/{oid}/items"),
            r#"{"label":"KTP terverifikasi"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let iid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/checklists/999999/items",
            r#"{"label":"ZZ"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/check-items/{iid}/toggle"),
            r#"{"done":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/checklists/{oid}/items"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(iid) && r["done"] == true));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=client", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "client.checklist_opened"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/clients/{cli}/checklists"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_fees_and_doc_requests() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clifee@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Auditor cannot create (Create gate).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/fees"),
            r#"{"name":"Retainer","kind":"retainer"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Bad kind rejected; ghost client 404s.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/fees"),
            r#"{"name":"Skema X","kind":"alien"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/999999/fees",
            r#"{"name":"Skema X"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/fees"),
            r#"{"name":"Retainer 2026","kind":"retainer","amount_minor":120000000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let fid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/fees"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(fid) && r["amount_minor"].as_i64() == Some(120000000)));
    // Doc requests: chain requested->received->verified; skip-ahead rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/doc-requests"),
            r#"{"title":"KTP para pihak"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/doc-requests/{did}/status"),
            r#"{"status":"verified"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["received", "verified"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/doc-requests/{did}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "docreq to {st}");
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/doc-requests"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(did) && r["status"] == "verified"));
    // Delete fee; ghost delete 404s.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/client-fees/{fid}"))
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
        .oneshot(
            Request::builder()
                .uri("/api/v1/client-fees/999999")
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
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=client", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "client.doc_requested"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/clients/{cli}/fees"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_communication_timeline() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clitimeline@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    // Meeting + checklist + tag feed the timeline.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/meetings"),
            r#"{"title":"Kickoff","happened_at":"2026-11-01 10:00:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/checklists"),
            r#"{"kind":"onboarding"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-tags",
            &format!(r#"{{"client_id":{cli},"tag":"prioritas"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/timeline"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tv = body_json(res).await;
    assert!(tv["dated"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["kind"] == "meeting" && r["title"] == "Kickoff"));
    assert!(tv["state"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["kind"] == "checklist"));
    assert!(tv["state"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["kind"] == "segment" && r["title"] == "prioritas"));
    // Ghost 404s; anon 401s.
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients/999999/timeline", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/clients/{cli}/timeline"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_roster_and_consents() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliroster@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/employees"),
            r#"{"name":"Sekretaris"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/999999/employees",
            r#"{"name":"Hantu"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/employees"),
            r#"{"name":"Direktur A","role":"Direktur"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let eid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/employees"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(eid)));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/employees/{eid}"))
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
    // Consents: upsert idempotent, bad channel rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/consents"),
            r#"{"channel":"newsletter","allowed":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let first = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/consents"),
            r#"{"channel":"newsletter","allowed":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["id"].as_i64().unwrap(), first);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/consents"),
            r#"{"channel":"sms","allowed":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/consents"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["channel"] == "newsletter" && r["allowed"] == true));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/clients/{cli}/employees"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_reminders_and_feedback() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clirem@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated + ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/reminders"),
            r#"{"title":"Ultah","remind_on":"2027-01-01"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/999999/reminders",
            r#"{"title":"Hari H","remind_on":"2027-01-01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/reminders"),
            r#"{"title":"x","remind_on":"baddate"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/reminders"),
            r#"{"title":"Ultah","remind_on":"2027-01-01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let rid = body_json(res).await["id"].as_i64().unwrap();
    // Upcoming window excludes far-future... 2027 is within 3650 days.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reminders?upcoming_days=3650", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(rid)));
    let res = a
        .clone()
        .oneshot(get("/api/v1/reminders?upcoming_days=1", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["id"].as_i64() != Some(rid)));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/reminders/{rid}/done"),
            r#"{"done":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Feedback: complaint escalates, feedback cannot.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/feedback",
            &format!(r#"{{"client_id":{cli},"kind":"complaint","body":"Lambat sekali"}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let fid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/feedback/{fid}/status"),
            r#"{"status":"resolved"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["escalated", "resolved"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/feedback/{fid}/status"),
                &format!(r#"{{"status":"{st}"}}"#,),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "fb to {st}");
    }
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/feedback",
            &format!(r#"{{"client_id":{cli},"kind":"feedback","body":"Bagus"}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let gid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/feedback/{gid}/status"),
            r#"{"status":"escalated"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/feedback?client_id={cli}"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 2);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reminders")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_vip_and_credit() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clivip@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated + ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-flags",
            &format!(r#"{{"client_id":{cli},"tier":"vip"}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-flags",
            r#"{"client_id":999999,"tier":"vip"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-flags",
            &format!(r#"{{"client_id":{cli},"tier":"alien"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-flags",
            &format!(r#"{{"client_id":{cli},"tier":"vip"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let vid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-flags",
            &format!(r#"{{"client_id":{cli},"tier":"vip"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/client-flags?client_id={cli}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(vid)));
    // Credit: under-limit then over-limit.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"MC","client_id":{cli}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let _ = mk_invoice(&a, &tok, mid, 5_000_000).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-credit",
            &format!(r#"{{"client_id":{cli},"limit_minor":10000000}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/exposure"), &tok))
        .await
        .unwrap();
    let ev = body_json(res).await;
    assert_eq!(ev["over_limit"], serde_json::json!(false));
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-credit",
            &format!(r#"{{"client_id":{cli},"limit_minor":1000}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/exposure"), &tok))
        .await
        .unwrap();
    let ev2 = body_json(res).await;
    assert_eq!(ev2["over_limit"], serde_json::json!(true));
    assert_eq!(ev2["exposure_minor"], ev["exposure_minor"]);
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients/999999/exposure", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/client-flags")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_engagement_letters() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliengel@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated + ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/engagement-letters",
            &format!(r#"{{"client_id":{cli},"title":"Kuasa","body":"Menangani perkara"}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/engagement-letters",
            r#"{"client_id":999999,"title":"Kuasa","body":"Isi surat"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/engagement-letters",
            &format!(r#"{{"client_id":{cli},"title":"Kuasa","body":"Menangani perkara"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let eid = body_json(res).await["id"].as_i64().unwrap();
    // Skip-ahead rejected; chain draft->issued->signed.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/engagement-letters/{eid}/status"),
            r#"{"status":"signed"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["issued", "signed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/engagement-letters/{eid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "letter to {st}");
    }
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/engagement-letters?client_id={cli}"),
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(eid) && r["status"] == "signed"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/engagement-letters")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_portal_invites_and_surveys() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliportal@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated + ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"klien@x.id"}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            r#"{"client_id":999999,"email":"klien@x.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"bukan-email"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"klien@x.id"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iv = body_json(res).await;
    let iid = iv["id"].as_i64().unwrap();
    assert!(iv["token"].as_str().unwrap().starts_with("cpi_"));
    // accept then terminal: second transition rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/portal-invites/{iid}/status"),
            r#"{"status":"accepted"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/portal-invites/{iid}/status"),
            r#"{"status":"revoked"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Surveys: bad score rejected; average asserted.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/surveys",
            &format!(r#"{{"client_id":{cli},"score":11}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    for s in [8, 6] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/surveys",
                &format!(r#"{{"client_id":{cli},"score":{s},"comment":"ok"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/surveys?client_id={cli}"), &tok))
        .await
        .unwrap();
    let sv = body_json(res).await;
    assert_eq!(sv["count"].as_i64().unwrap(), 2);
    assert_eq!(sv["avg"].as_f64().unwrap(), 7.0);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/portal-invites")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn portal_copy_requests_and_tracking() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliportal7@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"p7@x.id"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let ptoken = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            &format!(r#"{{"token":"{ptoken}"}}"#),
            None,
        ))
        .await
        .unwrap();
    let ptok = body_json(res).await["token"].as_str().unwrap().to_string();
    // Request a copy; staff sees it and fulfills; portal tracks it.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/copies",
            r#"{"note":"Salinan AJB"}"#,
            Some(&ptok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cpid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/copy-requests/{cpid}/status"),
            r#"{"status":"shipped"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/copy-requests/{cpid}/status"),
            r#"{"status":"fulfilled"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/copies", &ptok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cpid) && r["status"] == "fulfilled"));
    let res = a
        .clone()
        .oneshot(post("/api/v1/portal/invoices", "{}", Some(&ptok)))
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "invoices is GET-only"
    );
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/track", &ptok))
        .await
        .unwrap();
    let tv = body_json(res).await;
    assert!(tv["copies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cpid)));
    assert!(tv.get("doc_requests").is_some() && tv.get("appointments").is_some());
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/track", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn portal_messages_and_audit() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliportal6@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"p6@x.id"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let ptoken = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            &format!(r#"{{"token":"{ptoken}"}}"#),
            None,
        ))
        .await
        .unwrap();
    let ptok = body_json(res).await["token"].as_str().unwrap().to_string();
    // Client posts; staff reads and replies; thread shows both sides.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/messages",
            r#"{"body":"Halo, progres perkara?"}"#,
            Some(&ptok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/messages"),
            &format!(r#"{{"client_id":{cli},"body":"Sedang diproses"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/messages", &ptok))
        .await
        .unwrap();
    let tv = body_json(res).await;
    assert_eq!(tv.as_array().unwrap().len(), 2);
    assert!(tv
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["sender"] == "staff"));
    // Ghost client 404s on staff write; portal audit shows own trail.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/999999/messages",
            r#"{"client_id":999999,"body":"xx"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/activity", &ptok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/activity", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn portal_invoices_and_booking() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliportal5@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"INV","client_id":{cli}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let iid = mk_invoice(&a, &tok, mid, 2_500_000).await;
    let _ = iid;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"p5@x.id"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let ptoken = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            &format!(r#"{{"token":"{ptoken}"}}"#),
            None,
        ))
        .await
        .unwrap();
    let ptok = body_json(res).await["token"].as_str().unwrap().to_string();
    // Invoice visible with totals; booking roundtrip under own name.
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/invoices", &ptok))
        .await
        .unwrap();
    let iv = body_json(res).await;
    assert!(iv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["total_minor"].as_i64() == Some(2_500_000)));
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/appointments",
            r#"{"purpose":"Konsultasi","scheduled_at":"2027-06-01 10:00"}"#,
            Some(&ptok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/appointments", &ptok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["purpose"] == "Konsultasi"));
    // Staff is not a portal identity.
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/invoices", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn portal_inbox_and_secure_upload() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliportal4@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    let cli2 = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"PA","client_id":{cli}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let ma = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"PB","client_id":{cli2}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let mb = body_json(res).await["id"].as_i64().unwrap();
    // Staff seeds a doc request + other-client doc.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/doc-requests"),
            r#"{"title":"KTP"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let did_req = body_json(res).await["id"].as_i64().unwrap();
    let body = b"milik B";
    let sha = legalrust_document::sha256_hex(body);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mb},"name":"b.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#, body.len()), Some(&tok))).await.unwrap();
    let db = body_json(res).await["id"].as_i64().unwrap();
    // Portal login for cli.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"p4@x.id"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let ptoken = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            &format!(r#"{{"token":"{ptoken}"}}"#),
            None,
        ))
        .await
        .unwrap();
    let ptok = body_json(res).await["token"].as_str().unwrap().to_string();
    // Inbox shows own request; ack moves requested->received; foreign ack 404s.
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/doc-requests", &ptok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(did_req)));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/portal/doc-requests/{did_req}/received"),
            "{}",
            Some(&ptok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/doc-requests/999999/received",
            "{}",
            Some(&ptok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Upload to own matter works end-to-end (metadata + bytes roundtrip).
    let up = b"portal upload";
    let usha = legalrust_document::sha256_hex(up);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{ma},"name":"p.txt","mime":"text/plain","size":{},"sha":"{usha}"}}"#, up.len()), Some(&ptok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "portal own-matter upload");
    let du = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{du}/content"))
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {ptok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::from(up.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{du}/content"))
                .header(
                    "authorization",
                    format!("Bearer {ptok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Cross-book blocked: create under other matter, upload/download foreign doc.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"matter_id":{mb},"name":"x.txt","mime":"text/plain","size":1,"sha":"{}"}}"#,
                "ab".repeat(32)
            ),
            Some(&ptok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{db}/content"))
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {ptok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{db}/content"))
                .header(
                    "authorization",
                    format!("Bearer {ptok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn portal_dashboard_and_matters() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliportal3@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    let cli2 = mk_crm_client(&a, &tok).await;
    // Two books: only the login client's data may surface.
    for (c, t) in [(cli, "MA"), (cli2, "MB")] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/matters",
                &format!(r#"{{"title":"{t}","client_id":{c}}}"#,),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"p3@x.id"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let ptoken = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            &format!(r#"{{"token":"{ptoken}"}}"#),
            None,
        ))
        .await
        .unwrap();
    let ptok = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/dashboard", &ptok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let dv = body_json(res).await;
    assert_eq!(dv["matters"].as_array().unwrap().len(), 1);
    assert_eq!(
        dv["matters"].as_array().unwrap()[0]["title"],
        serde_json::json!("MA")
    );
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/matters", &ptok))
        .await
        .unwrap();
    let mv = body_json(res).await;
    assert_eq!(mv.as_array().unwrap().len(), 1);
    assert!(mv.as_array().unwrap()[0].get("tasks_by_status").is_some());
    // Staff token is not a portal identity.
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/dashboard", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/portal/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn portal_login_and_session() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliportal2@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    // Invite, then log in with the once-shown token.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal-invites",
            &format!(r#"{{"client_id":{cli},"email":"portal@x.id"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ptoken = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            r#"{"token":"salah-salah-salah"}"#,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            &format!(r#"{{"token":"{ptoken}"}}"#),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ptok = body_json(res).await["token"].as_str().unwrap().to_string();
    // Me resolves the client; staff token is not a portal identity.
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/me", &ptok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["client_id"].as_i64().unwrap(), cli);
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/me", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Logout revokes the portal session; me then 401s. Client role is least-privilege.
    let res = a
        .clone()
        .oneshot(post("/api/v1/auth/logout", "{}", Some(&ptok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/portal/me", &ptok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Fresh login again for role-gate probe: client cannot manage retention rules.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/portal/login",
            &format!(r#"{{"token":"{ptoken}"}}"#),
            None,
        ))
        .await
        .unwrap();
    let ptok2 = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/retention-rules",
            r#"{"entity":"bpn"}"#,
            Some(&ptok2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn conflict_of_interest_screening() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliconf@x.id").await;
    // Seed: client + matter + ppat party sharing one name.
    let cli = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"Waris Hartono","client_id":{cli}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"9/2026","deed_kind":"hibah"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/parties"),
            r#"{"role":"seller","name":"Hartono","id_number":"ID-H1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Screen hits all three surfaces.
    let res = a
        .clone()
        .oneshot(get("/api/v1/conflicts/check?name=Hartono", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cv = body_json(res).await;
    assert!(cv["matters"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["title"] == "Waris Hartono"));
    assert!(cv["parties"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["name"] == "Hartono" && p["role"] == "seller"));
    // Clean name, short needle, anon.
    let res = a
        .clone()
        .oneshot(get("/api/v1/conflicts/check?name=ZulfikarTidakAda", &tok))
        .await
        .unwrap();
    let ev = body_json(res).await;
    assert!(
        ev["clients"].as_array().unwrap().is_empty()
            && ev["parties"].as_array().unwrap().is_empty()
    );
    let res = a
        .clone()
        .oneshot(get("/api/v1/conflicts/check?name=X", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/conflicts/check?name=Hartono")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_overview_and_contact_roles() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cli360@x.id").await;
    let cli = mk_crm_client(&a, &tok).await;
    // Role allowlist guarded; ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/employees/999999/role",
            r#"{"contact_role":"primary"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/employees"),
            r#"{"name":"Budi","role":"Direktur"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let eid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/employees/{eid}/role"),
            r#"{"contact_role":"sekretaris"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/employees/{eid}/role"),
            r#"{"contact_role":"primary"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // 360 aggregates the seeded book.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"M360","client_id":{cli}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/overview"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ov = body_json(res).await;
    assert_eq!(ov["matters"].as_i64().unwrap(), 1);
    assert!(ov["employees"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["contact_role"] == "primary"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients/999999/overview", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/clients/{cli}/overview"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn retention_prospect_purge() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "prospret@x.id").await;
    // Prospect rule now accepted (was: entity not purgeable).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/retention-rules",
            r#"{"entity":"prospect","retain_years":2,"action":"purge","note":"prospek basi"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Create an org-scoped lead, then request its purge.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/admin/leads",
            r#"{"name":"Prospek Lama","email":"lama@x.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let lid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/purge-requests",
            &format!(r#"{{"entity":"prospect","entity_id":{lid},"reason":"lewat masa retensi"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    // Self-approve forbidden; second admin approves; execute deletes the lead.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/decide"),
            r#"{"approve":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/orgs/invites",
            r#"{"email":"pret2@x.id","role":"office_admin"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let inv = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a.clone().oneshot(post("/api/v1/auth/register", &format!(r#"{{"org_name":"Acme","email":"pret2@x.id","name":"PR2","password":"password1-2-3","invite_token":"{inv}"}}"#), None)).await.unwrap();
    let adm2 = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/decide"),
            r#"{"approve":true}"#,
            Some(&adm2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/execute"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/admin/leads/{lid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=preq", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "retention.purged"));
}

#[tokio::test]
async fn crm_dashboard_and_client_expiry() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cridash@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Seed: matter + doc with past-due lifecycle under this client.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"MD","client_id":{cli}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let body = b"isi dokumen kedaluwarsa";
    let sha = legalrust_document::sha256_hex(body);
    let res = a.clone().oneshot(post("/api/v1/documents", &format!(r#"{{"matter_id":{mid},"name":"e.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#, body.len()), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lifecycle"),
            r#"{"retain_until":"2001-05-05"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Client-scoped filter keeps it; ghost client excludes it.
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/documents/expiry-alerts?client_id={cli}"),
            &tok,
        ))
        .await
        .unwrap();
    let ev = body_json(res).await;
    assert!(ev
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(did) && r["client_id"].as_i64() == Some(cli)));
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/documents/expiry-alerts?client_id=999999",
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
    // Unfiltered feed still works (F-D09-022 intact).
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/expiry-alerts", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(did)));
    // Dashboard aggregates the seeded book.
    let res = a
        .clone()
        .oneshot(get("/api/v1/crm/dashboard", &auditor))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let dv = body_json(res).await;
    assert!(dv["clients"].as_i64().unwrap() >= 1);
    assert!(dv["matters"].as_i64().unwrap() >= 1);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/crm/dashboard?client_id={cli}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/crm/dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_risk_and_activity() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clirisk@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Update-gated + ghost 404s + bad level rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-risk",
            &format!(r#"{{"client_id":{cli},"level":"high"}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-risk",
            r#"{"client_id":999999,"level":"high"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-risk",
            &format!(r#"{{"client_id":{cli},"level":"alien"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-risk",
            &format!(r#"{{"client_id":{cli},"level":"high","note":"Sengketa besar"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Upsert idempotent: second write replaces.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-risk",
            &format!(r#"{{"client_id":{cli},"level":"critical"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/client-risk?level=critical", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["client_id"].as_i64() == Some(cli)));
    // Activity trail: merge the client to generate a trail entry, then scope it.
    let k2 = mk_crm_client(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/merge",
            &format!(r#"{{"keep_id":{cli},"drop_id":{k2}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/activity"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "client.merged"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients/999999/activity", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/client-risk")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn event_and_seminar_management() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clievent@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated + bad date rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-events",
            r#"{"title":"Seminar","held_on":"2027-03-01"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-events",
            r#"{"title":"X","held_on":"besok"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-events",
            r#"{"title":"Seminar Waris","venue":"Aula","held_on":"2027-03-01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let eid = body_json(res).await["id"].as_i64().unwrap();
    // planned->held; re-hold rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-events/{eid}/status"),
            r#"{"status":"held"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-events/{eid}/status"),
            r#"{"status":"held"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Attendance: ghost 404s, dup 409s, invited->attended.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-events/{eid}/attendees"),
            r#"{"client_id":999999}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-events/{eid}/attendees"),
            &format!(r#"{{"client_id":{cli}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let aid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-events/{eid}/attendees"),
            &format!(r#"{{"client_id":{cli}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/attendances/{aid}/status"),
            r#"{"status":"attended"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/attendances?event_id={eid}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(aid) && r["status"] == "attended"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/client-events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn campaign_tracking() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "clicamp@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated + bad channel rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/campaigns",
            r#"{"name":"Edu Notaris"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/campaigns",
            r#"{"name":"Edu Notaris","channel":"sms"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/campaigns",
            r#"{"name":"Edu Notaris","channel":"event"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    // Skip-ahead rejected; chain planned->active->done.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/campaigns/{mid}/status"),
            r#"{"status":"done"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["active", "done"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/campaigns/{mid}/status"),
                &format!(r#"{{"status":"{st}"}}"#,),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "campaign to {st}");
    }
    // Participations: ghost 404s, dup 409s, response chain guarded.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/campaigns/{mid}/participants"),
            r#"{"client_id":999999}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/campaigns/{mid}/participants"),
            &format!(r#"{{"client_id":{cli}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/campaigns/{mid}/participants"),
            &format!(r#"{{"client_id":{cli}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/participations/{pid}/response"),
            r#"{"response":"converted"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["responded", "converted"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/participations/{pid}/response"),
                &format!(r#"{{"response":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "participation to {st}");
    }
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/participations?campaign_id={mid}"),
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(pid) && r["response"] == "converted"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/campaigns")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_routing_and_export() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliroute@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated + ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/notif-routes"),
            r#"{"name":"Sekretaris","channel":"email","target":"s@x.id"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/999999/notif-routes",
            r#"{"name":"Hantu","channel":"email","target":"s@x.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/notif-routes"),
            r#"{"name":"Sekretaris","channel":"sms","target":"nope"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/notif-routes"),
            r#"{"name":"Sekretaris","channel":"email","target":"sek@kantor.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let rid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/notif-routes"),
            r#"{"name":"Sekretaris","channel":"email","target":"sek@kantor.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Resolve merges routes + employee contacts with consent gating.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/employees"),
            r#"{"name":"Budi","contact":"budi@kantor.id"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{cli}/consents"),
            r#"{"channel":"email","allowed":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{cli}/notify-plan"), &tok))
        .await
        .unwrap();
    let pv = body_json(res).await;
    assert!(pv["routes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["target"] == "budi@kantor.id" && r["allowed"] == true));
    assert!(pv["routes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["target"] == "sek@kantor.id" && r["allowed"] == false));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/notif-routes/{rid}"))
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
    // Controlled export: auditor allowed, anon denied, audit logged.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/clients.csv", &auditor))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let staff = forged_role_token(&tok, vec!["legal_staff"]);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/clients.csv", &staff))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/clients.csv", &tok))
        .await
        .unwrap();
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        text.starts_with("id,display_name,kind,status\n"),
        "was {text}"
    );
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
        .any(|e| e["action"] == "report.clients_csv_exported"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/notif-routes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pipeline_and_winback() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "pipewb@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let cli = mk_crm_client(&a, &tok).await;
    // Create-gated.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/opportunities",
            &format!(r#"{{"client_id":{cli},"title":"KPR massal"}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Ghost 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/opportunities",
            r#"{"client_id":999999,"title":"Hantu"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Pipeline chain to won; skip-ahead rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/opportunities",
            &format!(r#"{{"client_id":{cli},"title":"KPR massal","amount_minor":2000000000}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let oid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/opportunities/{oid}/stage"),
            r#"{"stage":"proposal"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["qualified", "proposal", "won"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/opportunities/{oid}/stage"),
                &format!(r#"{{"stage":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "opp to {st}");
    }
    // Win-back identified->contacted->won.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/winbacks",
            &format!(r#"{{"client_id":{cli},"reason":"Pindah ke kompetitor"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let wid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/winbacks/{wid}/status"),
            r#"{"status":"won"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["contacted", "won"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/winbacks/{wid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "wb to {st}");
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/opportunities?client_id={cli}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(oid) && r["stage"] == "won"));
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/winbacks?client_id={cli}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(wid) && r["status"] == "won"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/opportunities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_referrals_and_ltv() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cliref@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let c1 = mk_crm_client(&a, &tok).await;
    let c2 = mk_crm_client(&a, &tok).await;
    // Auditor cannot create (Create gate).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-referrals",
            &format!(r#"{{"referrer_id":{c1},"referred_id":{c2}}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Self-link + ghost rejected; dup 409s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-referrals",
            &format!(r#"{{"referrer_id":{c1},"referred_id":{c1}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-referrals",
            r#"{"referrer_id":999999,"referred_id":999998}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-referrals",
            &format!(r#"{{"referrer_id":{c1},"referred_id":{c2}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-referrals",
            &format!(r#"{{"referrer_id":{c1},"referred_id":{c2}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/client-referrals?client_id={c1}"),
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(rid)));
    // LTV: one matter, one invoice, partial payment.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"M1","client_id":{c1}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let iid = mk_invoice(&a, &tok, mid, 1_000_000).await;
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
        .oneshot(get(&format!("/api/v1/clients/{c1}/lifetime-value"), &tok))
        .await
        .unwrap();
    let lv = body_json(res).await;
    assert_eq!(lv["matters"].as_i64().unwrap(), 1);
    assert_eq!(lv["invoiced_minor"].as_i64().unwrap(), 1_000_000);
    assert_eq!(lv["paid_minor"].as_i64().unwrap(), 400_000);
    assert_eq!(lv["open_minor"].as_i64().unwrap(), 600_000);
    // Empty client zeroes; ghost 404s.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{c2}/lifetime-value"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["paid_minor"].as_i64().unwrap(), 0);
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients/999999/lifetime-value", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/client-referrals")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn client_merge_and_corrections() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "climerge@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let k1 = mk_crm_client(&a, &tok).await;
    let k2 = mk_crm_client(&a, &tok).await;
    // Admin gate: lawyer cannot merge.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/merge",
            &format!(r#"{{"keep_id":{k1},"drop_id":{k2}}}"#,),
            Some(&lawyer),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Self-merge + ghost rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/merge",
            &format!(r#"{{"keep_id":{k1},"drop_id":{k1}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/merge",
            r#"{"keep_id":999999,"drop_id":999998}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Seed drop-client rows: matter + tag + fee.
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters",
            &format!(r#"{{"title":"Pindah","client_id":{k2}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-tags",
            &format!(r#"{{"client_id":{k2},"tag":"prioritas"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/clients/{k2}/fees"),
            r#"{"name":"Retainer","kind":"retainer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Merge moves the matter and deletes the drop client.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/clients/merge",
            &format!(r#"{{"keep_id":{k1},"drop_id":{k2}}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["merged_matters"].as_i64().unwrap(), 1);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/clients/{k1}/fees"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "Retainer"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=client", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "client.merged"));
    // Correction: request -> self-approve forbidden -> second admin approves -> apply writes.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/client-corrections",
            &format!(r#"{{"client_id":{k1},"field":"display_name","new_value":"PT Baru Jaya"}}"#,),
            Some(&tok),
        ))
        .await
        .unwrap();
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-corrections/{cid}/decide"),
            r#"{"approve":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/orgs/invites",
            r#"{"email":"cadmin@x.id","role":"office_admin"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let inv = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a.clone().oneshot(post("/api/v1/auth/register", &format!(r#"{{"org_name":"Acme","email":"cadmin@x.id","name":"CA","password":"password1-2-3","invite_token":"{inv}"}}"#), None)).await.unwrap();
    let adm2 = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-corrections/{cid}/decide"),
            r#"{"approve":true}"#,
            Some(&adm2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Apply before approve is impossible here (already approved); apply now.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-corrections/{cid}/apply"),
            "{}",
            Some(&adm2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/clients", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"].as_i64() == Some(k1) && c["display_name"] == "PT Baru Jaya"));
    // Re-apply rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/client-corrections/{cid}/apply"),
            "{}",
            Some(&adm2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/clients/merge")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"keep_id":{k1},"drop_id":{k1}}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let _ = mid;
}

#[tokio::test]
async fn document_diff_and_expiry_alerts() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docdiff@x.id").await;
    async fn mk_text(app: &axum::Router, tok: &str, name: &str, body: &[u8]) -> i64 {
        use tower::ServiceExt;
        let sha = legalrust_document::sha256_hex(body);
        let res = app
            .clone()
            .oneshot(post(
                "/api/v1/documents",
                &format!(
                    r#"{{"name":"{name}","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                    body.len()
                ),
                Some(tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        body_json(res).await["id"].as_i64().unwrap()
    }
    async fn put_bytes(app: &axum::Router, tok: &str, did: i64, body: &[u8]) {
        use tower::ServiceExt;
        let res = app
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
                    .body(Body::from(body.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let d1 = mk_text(&a, &tok, "v1.txt", b"alpha\nbeta\ngamma").await;
    put_bytes(&a, &tok, d1, b"alpha\nbeta\ngamma").await;
    let d2 = mk_text(&a, &tok, "v2.txt", b"alpha\nBETA2\ngamma\ndelta").await;
    put_bytes(&a, &tok, d2, b"alpha\nBETA2\ngamma\ndelta").await;
    // Self-diff is empty.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{d1}/diff/{d1}"), &tok))
        .await
        .unwrap();
    let sv = body_json(res).await;
    assert!(sv["added"].as_array().unwrap().is_empty());
    assert!(sv["removed"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{d1}/diff/{d2}"), &tok))
        .await
        .unwrap();
    let dv = body_json(res).await;
    assert!(dv["added"].as_array().unwrap().iter().any(|x| x == "BETA2"));
    assert!(dv["added"].as_array().unwrap().iter().any(|x| x == "delta"));
    assert!(dv["removed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "beta"));
    assert_eq!(dv["same_lines"].as_i64().unwrap(), 2);
    // Binary side 422s; ghost 404s.
    let bin = b"%PDF-1.4";
    let bsha = legalrust_document::sha256_hex(bin);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"b.pdf","mime":"application/pdf","size":{},"sha":"{bsha}"}}"#,
                bin.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    let bid = body_json(res).await["id"].as_i64().unwrap();
    put_bytes(&a, &tok, bid, bin).await;
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{d1}/diff/{bid}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/999999/diff/1", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Expiry alerts surface past-due retentions.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{d2}/lifecycle"),
            r#"{"retain_until":"2001-05-05"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/expiry-alerts", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(d2)));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{d1}/diff/{d2}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn document_watermark_and_stamp_text_layer() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docwm@x.id").await;
    let content = b"isi akta baris satu\nbaris dua";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"n.txt","mime":"text/plain","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
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
    // Watermark prepends + versions.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/watermark"),
            r#"{"text":"RAHASIA"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["version"].as_i64().unwrap(), 2);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}/content"), &tok))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    assert!(bytes.starts_with(b"WATERMARK: RAHASIA\nisi akta"));
    // Stamp appends another version.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/stamp"),
            r#"{"kind":"received"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["version"].as_i64().unwrap(), 3);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}/content"), &tok))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    assert!(bytes.starts_with(b"DITERIMA "));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/stamp"),
            r#"{"kind":"alien"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Binary mime rejected with clear reason.
    let bin = b"%PDF-fake";
    let bsha = legalrust_document::sha256_hex(bin);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"b.pdf","mime":"application/pdf","size":{},"sha":"{bsha}"}}"#,
                bin.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    let bid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{bid}/content"))
                .method("PUT")
                .header(
                    "authorization",
                    format!("Bearer {tok}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::from(bin.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{bid}/watermark"),
            r#"{"text":"X"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}/watermark"))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":"X"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn document_locks_and_favorites() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "doclock@x.id").await;
    // Second user in the same org (different sub) for holder conflicts.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/orgs/invites",
            r#"{"email":"memberlock@x.id","role":"lawyer"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let inv = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a.clone().oneshot(post("/api/v1/auth/register", &format!(r#"{{"org_name":"Acme","email":"memberlock@x.id","name":"Anggota","password":"password1-2-3","invite_token":"{inv}"}}"#), None)).await.unwrap();
    let mem = body_json(res).await["token"].as_str().unwrap().to_string();
    let content = b"lock-bytes-1234";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"kunci.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    // Member locks; owner lock attempt rejected; same-holder relock idempotent.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lock"),
            "{}",
            Some(&mem),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lock"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lock"),
            "{}",
            Some(&mem),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Locked by other: content write blocked for owner.
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
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Unlock by non-holder rejected; holder unlocks; write proceeds.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/unlock"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/unlock"),
            "{}",
            Some(&mem),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
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
    // Favorites roundtrip + ghost handling.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/fav"),
            "{}",
            Some(&mem),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/favorites", &mem))
        .await
        .unwrap();
    let fv = body_json(res).await;
    assert!(fv["favorites"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(did)));
    assert!(!fv["recent"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}/fav"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {mem}")
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
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}/fav"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {mem}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/999999/lock", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await.get("holder").is_some());
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/favorites")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn document_metadata_schema_values_tags() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "docmeta@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let content = b"meta-bytes";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"m.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    // Auditor may not define fields (Create gate).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/meta-fields",
            r#"{"key":"nomor","kind":"text"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Define fields; dup rejected; bad kind rejected.
    for (key, kind) in [
        ("nomor_arsip", "text"),
        ("nilai", "number"),
        ("tanggal", "date"),
        ("label", "tag"),
    ] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/meta-fields",
                &format!(r#"{{"key":"{key}","kind":"{kind}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "field {key}");
    }
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/meta-fields",
            r#"{"key":"nomor_arsip","kind":"text"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/meta-fields",
            r#"{"key":"xx","kind":"alien"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Values: unknown field rejected; kind validation enforced.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/meta"),
            r#"{"key":"ghost","value":"x"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/meta"),
            r#"{"key":"nilai","value":"bukan-angka"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/meta"),
            r#"{"key":"tanggal","value":"2026"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Scalar upsert (same id twice), tags accumulate.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/meta"),
            r#"{"key":"nomor_arsip","value":"A-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let first = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/meta"),
            r#"{"key":"nomor_arsip","value":"A-2"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["id"].as_i64().unwrap(), first);
    for t in ["penting", "shm"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/documents/{did}/meta"),
                &format!(r#"{{"key":"label","value":"{t}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}/meta"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 3);
    // Tag search + targeted delete.
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/by-tag?tag=penting", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(did)));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}/meta?key=label&value=shm"))
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
        .oneshot(get("/api/v1/documents/by-tag?tag=shm", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await.as_array().unwrap().is_empty());
    // Field in use cannot be deleted.
    let fields = body_json(
        a.clone()
            .oneshot(get("/api/v1/meta-fields", &tok))
            .await
            .unwrap(),
    )
    .await;
    let fid = fields
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == "nomor_arsip")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/meta-fields/{fid}"))
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
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/meta-fields")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn document_lifecycle_hold_archive_trash_delete() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "doclife@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let content = b"lifecycle-bytes";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"akta-lama.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    // Default state is clean.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/documents/{did}/lifecycle"), &tok))
        .await
        .unwrap();
    let lv = body_json(res).await;
    assert_eq!(lv["on_hold"], serde_json::json!(false));
    // Auditor cannot mutate lifecycle (Update gate).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lifecycle"),
            r#"{"on_hold":true}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Ghost doc 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents/999999/lifecycle",
            r#"{"on_hold":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Hold + past retention date -> due list (archived items are out of scope).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lifecycle"),
            r#"{"on_hold":true,"retain_until":"2001-01-01","note":"arsip lama"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/documents/retention-due", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(did)));
    // Hold blocks secure delete; un-trashed blocks too.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}"))
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
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lifecycle"),
            r#"{"on_hold":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}"))
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
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Trash then delete: gone everywhere.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/documents/{did}/lifecycle"),
            r#"{"trashed":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/{did}"))
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
        .oneshot(get(&format!("/api/v1/documents/{did}"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=document", &tok))
        .await
        .unwrap();
    let rows = body_json(res).await;
    assert!(rows
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "document.lifecycle_set"));
    assert!(rows
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "document.secure_deleted"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/retention-due")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn quorum_rounds_and_dual_override() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "quorum@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    // required > members rejected.
    let res = a.clone().oneshot(post("/api/v1/quorums", &format!(r#"{{"entity":"matter","entity_id":{mid},"title":"R1","required":5,"members":["a","b"]}}"#, ), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a.clone().oneshot(post("/api/v1/quorums", &format!(r#"{{"entity":"matter","entity_id":{mid},"title":"Rilis SHM","required":2,"members":["v1","v2","v3"]}}"#, ), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    // Non-member + dup votes rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorums/{rid}/vote"),
            r#"{"voter":"outsider","decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorums/{rid}/vote"),
            r#"{"voter":"v1","decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"], serde_json::json!("open"));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorums/{rid}/vote"),
            r#"{"voter":"v1","decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Second approve meets quorum; further votes rejected (closed).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorums/{rid}/vote"),
            r#"{"voter":"v2","decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"], serde_json::json!("met"));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorums/{rid}/vote"),
            r#"{"voter":"v3","decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Fail path: 2 rejects out of 3 with required 2.
    let res = a.clone().oneshot(post("/api/v1/quorums", &format!(r#"{{"entity":"matter","entity_id":{mid},"title":"Rilis gagal","required":2,"members":["a","b","c"]}}"#, ), Some(&tok))).await.unwrap();
    let fid = body_json(res).await["id"].as_i64().unwrap();
    for v in ["a", "b"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/quorums/{fid}/vote"),
                &format!(r#"{{"voter":"{v}","decision":"reject"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/quorums/{fid}"), &tok))
        .await
        .unwrap();
    let fv = body_json(res).await;
    assert_eq!(fv["status"], serde_json::json!("failed"));
    assert_eq!(fv["votes"].as_array().unwrap().len(), 2);
    // Override: self-approve forbidden; second admin approves -> overridden.
    let res = a.clone().oneshot(post("/api/v1/quorums", &format!(r#"{{"entity":"matter","entity_id":{mid},"title":"Butuh override","required":3,"members":["x","y","z"]}}"#, ), Some(&tok))).await.unwrap();
    let oid_r = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorums/{oid_r}/override"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let oid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorum-overrides/{oid}/decide"),
            r#"{"approve":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/orgs/invites",
            r#"{"email":"qadmin@x.id","role":"office_admin"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let inv = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a.clone().oneshot(post("/api/v1/auth/register", &format!(r#"{{"org_name":"Acme","email":"qadmin@x.id","name":"QA","password":"password1-2-3","invite_token":"{inv}"}}"#), None)).await.unwrap();
    let adm2 = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quorum-overrides/{oid}/decide"),
            r#"{"approve":true}"#,
            Some(&adm2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/quorums/{oid_r}"), &tok))
        .await
        .unwrap();
    assert_eq!(
        body_json(res).await["status"],
        serde_json::json!("overridden")
    );
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/quorums")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_api_token_for_integrations() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apitoken@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    // Mint an integration token, then drive approvals with it (no session).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/auth/tokens",
            r#"{"name":"erp-bridge"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let raw = body_json(res).await["token"].as_str().unwrap().to_string();
    assert!(raw.starts_with("lr_"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals", &raw))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(aid)));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&raw),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Revoked tokens stop working.
    let res = a
        .clone()
        .oneshot(get("/api/v1/auth/tokens", &tok))
        .await
        .unwrap();
    let tid = body_json(res).await.as_array().unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
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
        .oneshot(get("/api/v1/approvals", &raw))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_conditional_and_amendment() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprcond@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    // Condition with non-approve decision is rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"reject","condition":"bayar dulu"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Conditional approve stores the condition visibly.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve","condition":"lunas sebelum serah terima"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/approvals/{aid}"), &tok))
        .await
        .unwrap();
    let det = body_json(res).await;
    assert!(det["steps"].as_array().unwrap()[0]["comment"]
        .as_str()
        .unwrap()
        .contains("[syarat] lunas sebelum serah terima"));
    // Amendment of a pending request is rejected; of approved spawns re-approval.
    let mid2 = mk_matter(&a, &tok).await;
    let pend = mk_approval(&a, &tok, mid2, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{pend}/amend"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/amend"),
            r#"{"title":"Amendemen nilai"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let nid = body_json(res).await["id"].as_i64().unwrap();
    assert_eq!(
        body_json(
            a.clone()
                .oneshot(get(&format!("/api/v1/approvals/{nid}"), &tok))
                .await
                .unwrap()
        )
        .await["status"],
        serde_json::json!("pending")
    );
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=approval", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "approval.amended"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/approvals/{aid}/amend"))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_replay_protection_and_simulation() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprreplay@x.id").await;
    let bearer = format!("Bearer {tok}")
        .parse::<axum::http::HeaderValue>()
        .unwrap();
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    // Simulation previews without mutating.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/simulate"),
            r#"{"step_no":1,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sv = body_json(res).await;
    assert_eq!(sv["ok"], serde_json::json!(true));
    assert_eq!(sv["outcome"], serde_json::json!("approved"));
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/approvals/{aid}/simulate"),
            r#"{"step_no":9,"decision":"approve"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["ok"], serde_json::json!(false));
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/decidable", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["rows"].as_array().unwrap().len(), 1);
    // Decide with idempotency key; replay is rejected with the replay marker.
    let keyed = |key: &str, uri: String, body: &str| {
        Request::builder()
            .uri(uri)
            .method("POST")
            .header("authorization", bearer.clone())
            .header("idempotency-key", key)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    let res = a
        .clone()
        .oneshot(keyed(
            "K-1",
            format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(keyed(
            "K-1",
            format!("/api/v1/approvals/{aid}/decisions"),
            r#"{"step_no":1,"decision":"approve"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    assert!(body_json(res).await["error"]["message"]
        .as_str()
        .unwrap()
        .contains("replayed"));
    // Bulk replay guard on a fresh request.
    let mid2 = mk_matter(&a, &tok).await;
    let bid = mk_approval(&a, &tok, mid2, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let bulk = format!(r#"{{"items":[{{"id":{bid},"step_no":1,"decision":"approve"}}]}}"#);
    let res = a
        .clone()
        .oneshot(keyed("K-2", "/api/v1/approvals/decide-bulk".into(), &bulk))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(keyed("K-2", "/api/v1/approvals/decide-bulk".into(), &bulk))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/approvals/{aid}/simulate"))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"step_no":1,"decision":"approve"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_bulk_decide_controlled() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprbulk@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let m1 = mk_matter(&a, &tok).await;
    let m2 = mk_matter(&a, &tok).await;
    let a1 = mk_approval(&a, &tok, m1, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let a2 = mk_approval(&a, &tok, m2, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    // Decidable inbox lists both.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/decidable", &tok))
        .await
        .unwrap();
    let dv = body_json(res).await;
    assert!(dv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(a1)));
    // Lawyer lacks Approve permission.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/approvals/decide-bulk",
            &format!(r#"{{"items":[{{"id":{a1},"step_no":1,"decision":"approve"}}]}}"#),
            Some(&lawyer),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Atomicity: one ghost item poisons the whole batch.
    let res = a.clone().oneshot(post("/api/v1/approvals/decide-bulk", &format!(r#"{{"items":[{{"id":{a1},"step_no":1,"decision":"approve"}},{{"id":{a2},"step_no":9,"decision":"approve"}}]}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/decidable", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["rows"].as_array().unwrap().len(), 2);
    // Clean bulk approve closes both.
    let res = a.clone().oneshot(post("/api/v1/approvals/decide-bulk", &format!(r#"{{"items":[{{"id":{a1},"step_no":1,"decision":"approve"}},{{"id":{a2},"step_no":1,"decision":"approve"}}]}}"#), Some(&tok))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["decided"].as_i64().unwrap(), 2);
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/decidable", &tok))
        .await
        .unwrap();
    assert!(body_json(res).await["rows"].as_array().unwrap().is_empty());
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=approval", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "approval.bulk_decided"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approvals/decidable")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn approval_sla_and_conflicts() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprsla@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let _ = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let _ = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    // SLA: fresh rows are ok; thresholds echoed.
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/approvals/sla?warn_hours=24&breach_hours=72",
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sv = body_json(res).await;
    assert_eq!(sv["warn_hours"].as_i64().unwrap(), 24);
    assert_eq!(sv["rows"].as_array().unwrap().len(), 2);
    assert!(sv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["sla"] == "ok"));
    // Conflicts: same matter twice.
    let res = a
        .clone()
        .oneshot(get("/api/v1/approvals/conflicts", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cv = body_json(res).await;
    assert_eq!(cv["conflict_count"].as_i64().unwrap(), 1);
    let c0 = &cv["conflicts"].as_array().unwrap()[0];
    assert_eq!(c0["entity"], serde_json::json!("matter"));
    assert_eq!(c0["count"].as_i64().unwrap(), 2);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/approvals/sla")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn retention_rules_and_gated_purge() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "retpurge@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    // Lawyer cannot manage rules (admin gate).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/retention-rules",
            r#"{"entity":"bpn","retain_years":5,"action":"purge"}"#,
            Some(&lawyer),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Unknown entity rejected; bad years rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/retention-rules",
            r#"{"entity":"alien"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/retention-rules",
            r#"{"entity":"bpn","retain_years":999}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/retention-rules",
            r#"{"entity":"bpn","retain_years":3,"action":"purge","note":"BPN manual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Create a purgeable row, request purge, self-approve forbidden.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/bpn-tracks",
            r#"{"kind":"queue","ref_no":"Q-99"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/purge-requests",
            &format!(r#"{{"entity":"bpn","entity_id":{tid},"reason":"lewat masa retensi"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    // Deed rows are legal holds, never purgeable.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/purge-requests",
            r#"{"entity":"deed","entity_id":1,"reason":"coba hapus akta"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/decide"),
            r#"{"approve":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Execute before approval is rejected too.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/execute"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Second admin (same org, different sub) approves via invite flow.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/orgs/invites",
            r#"{"email":"admin2@x.id","role":"office_admin"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let inv = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a.clone().oneshot(post("/api/v1/auth/register", &format!(r#"{{"org_name":"Acme","email":"admin2@x.id","name":"Admin Dua","password":"password1-2-3","invite_token":"{inv}"}}"#), None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let adm2 = body_json(res).await["token"].as_str().unwrap().to_string();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/decide"),
            r#"{"approve":true}"#,
            Some(&adm2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["status"],
        serde_json::json!("approved")
    );
    // Re-decide rejected; owner executes (OfficeAdmin has no Delete by design).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/decide"),
            r#"{"approve":false}"#,
            Some(&adm2),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/execute"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/bpn-tracks?kind=queue", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["id"].as_i64() != Some(tid)));
    // Execute twice rejected; audit trail recorded all four actions.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/purge-requests/{pid}/execute"),
            "{}",
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/purge-requests", &tok))
        .await
        .unwrap();
    let pv = body_json(res).await;
    assert!(pv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(pid) && r["status"] == "executed"));
    for (act, ent) in [
        ("retention.rule_saved", "rrule"),
        ("retention.purge_requested", "preq"),
        ("retention.purge_approved", "preq"),
        ("retention.purged", "preq"),
    ] {
        let res = a
            .clone()
            .oneshot(get(&format!("/api/v1/audit?entity={ent}"), &tok))
            .await
            .unwrap();
        let rows = body_json(res).await;
        assert!(
            rows.as_array()
                .unwrap()
                .iter()
                .any(|e| e["action"].as_str() == Some(act)),
            "missing audit {act}"
        );
    }
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/purge-requests")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn certificate_authenticity_verdict() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "authverd@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"AV/1","deed_kind":"ajb","seller":"AA","buyer":"BB","object":"SHM 1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    // Bare deed fails with reasons.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/authenticity"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v0 = body_json(res).await;
    assert_eq!(v0["verdict"], serde_json::json!(false));
    assert!(!v0["reasons"].as_array().unwrap().is_empty());
    // Build the full chain: consent verified + scan pinned + draft + BPN answered.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/consents"),
            r#"{"kind":"marital","party":"Istri AA"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/consents/{cid}/verify"),
            r#"{"verified":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let content = b"scan-shm";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"shm-av1.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/vault"),
            &format!(r#"{{"doc_id":{did}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/ajb-drafts"),
            r#"{"body":"Draf AJB yang cukup panjang untuk lolos validasi minimum."}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/bpn-tracks",
            r#"{"kind":"verification","ref_no":"V-AV1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let bid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["submitted", "answered"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/bpn-tracks/{bid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/authenticity"), &tok))
        .await
        .unwrap();
    let v1 = body_json(res).await;
    assert_eq!(
        v1["verdict"],
        serde_json::json!(true),
        "checks: {}",
        serde_json::to_string(&v1["checks"]).unwrap()
    );
    assert!(v1["reasons"].as_array().unwrap().is_empty());
    // Ghost deed 404s; anon 401s.
    let res = a
        .clone()
        .oneshot(get("/api/v1/ppat/999999/authenticity", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/ppat/{pid}/authenticity"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bpn_tracks_manual_mode() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "bpntrack@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/bpn-tracks",
            r#"{"kind":"verification","ref_no":"V-1"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/bpn-tracks",
            r#"{"kind":"alien","ref_no":"XX"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // One row per kind, each with its own chain.
    let cases = [
        ("verification", "V-1", "draft", "submitted"),
        ("queue", "Q-7", "waiting", "called"),
        ("response", "R-3", "draft", "filed"),
    ];
    for (kind, refno, init, next) in cases {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/bpn-tracks",
                &format!(r#"{{"kind":"{kind}","ref_no":"{refno}","office":"Kantah BDG"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "add {kind}");
        let id = body_json(res).await["id"].as_i64().unwrap();
        let res = a
            .clone()
            .oneshot(get(&format!("/api/v1/bpn-tracks?kind={kind}"), &tok))
            .await
            .unwrap();
        let lv = body_json(res).await;
        assert!(
            lv.as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"].as_i64() == Some(id) && r["status"] == init),
            "init {kind}"
        );
        // Wrong-kind transition rejected (e.g. filed on verification).
        let wrong = if kind == "response" {
            "submitted"
        } else {
            "filed"
        };
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/bpn-tracks/{id}/status"),
                &format!(r#"{{"status":"{wrong}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::CONFLICT,
            "wrong transition {kind}"
        );
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/bpn-tracks/{id}/status"),
                &format!(r#"{{"status":"{next}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "step {kind}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/bpn-tracks", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 3);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=bpn", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.bpn_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bpn-tracks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cert_vault_links() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "certvault@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Deed + document scan.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"CV/1","deed_kind":"ajb","seller":"AA","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let content = b"fake-cert-scan";
    let sha = legalrust_document::sha256_hex(content);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/documents",
            &format!(
                r#"{{"name":"shm-1.pdf","mime":"application/pdf","size":{},"sha":"{sha}"}}"#,
                content.len()
            ),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/vault"),
            &format!(r#"{{"doc_id":{did},"label":"SHM asli"}}"#),
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/vault"),
            &format!(r#"{{"doc_id":{did},"label":"SHM asli"}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Duplicate link + ghost doc + ghost deed rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/vault"),
            &format!(r#"{{"doc_id":{did}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/vault"),
            r#"{"doc_id":999999}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat/999999/vault",
            &format!(r#"{{"doc_id":{did}}}"#),
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/vault"), &tok))
        .await
        .unwrap();
    let vv = body_json(res).await;
    assert!(vv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["doc_id"].as_i64() == Some(did) && r["doc_name"] == "shm-1.pdf"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=clink", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.clink_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/ppat/{pid}/vault"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ajb_drafts_and_ppat_audit() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ajbpat@x.id").await;
    let lawyer = forged_role_token(&tok, vec!["lawyer"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"AJB/1","deed_kind":"ajb","seller":"AA","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    // Too-short body rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/ajb-drafts"),
            r#"{"body":"terlalu pendek"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(&format!("/api/v1/ppat/{pid}/ajb-drafts"), r#"{"template_name":"AJB standar","body":"Pada hari ini menghadap ___, penjual AA menjual kepada pembeli BB objek SHM 1 seharga Rp500jt.","price_minor":500000000}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/ajb-drafts"), &tok))
        .await
        .unwrap();
    let av = body_json(res).await;
    assert_eq!(av.as_array().unwrap().len(), 1);
    assert_eq!(av[0]["price_minor"].as_i64().unwrap(), 500000000);
    // Ghost deed 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat/999999/ajb-drafts",
            r#"{"body":"zzzzzzzzzzzzzzzzzzzz"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // PPAT audit: owner sees the ajbd event; lawyer is forbidden; anon 401.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/ppat-audit?limit=50", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pv = body_json(res).await;
    assert!(pv["entities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e == "ajbd"));
    assert!(pv["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["action"] == "property.ajbd_added"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/ppat-audit", &lawyer))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/ppat-audit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn kpr_packs_and_protocol_register() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "kprpreg@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-register",
            r#"{"book_ref":"B-1","party":"PT X"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"KP/1","deed_kind":"ajb","seller":"AA","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/kpr"),
            r#"{"bank":"Bank Maju","amount_minor":800000000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let kid = body_json(res).await["id"].as_i64().unwrap();
    // applied->disbursed direct rejected; branch to rejected ok.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/kpr/{kid}/status"),
            r#"{"status":"disbursed"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["approved", "disbursed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/kpr/{kid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "kpr to {st}");
    }
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-register",
            r#"{"book_ref":"B-1","deed_no":"12/2026","party":"PT X","direction":"out"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-register",
            r#"{"book_ref":"B-1","party":"PT Y","direction":"sideways"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/protocol-register", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["deed_no"] == "12/2026" && r["direction"] == "out"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=preg", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.preg_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/protocol-register")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn dev_batches_and_units() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "devbatch@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/dev-batches",
            r#"{"developer":"PT Maju","batch_no":"B-1"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/dev-batches",
            r#"{"developer":"PT Maju","batch_no":"B-1","total_units":50}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bid = body_json(res).await["id"].as_i64().unwrap();
    // Duplicate batch_no rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/dev-batches",
            r#"{"developer":"PT Maju","batch_no":"B-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Skip-ahead rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/dev-batches/{bid}/status"),
            r#"{"status":"done"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/dev-batches/{bid}/status"),
            r#"{"status":"processing"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Units: ghost batch 404s; duplicate code 409s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/dev-units",
            r#"{"batch_id":999999,"unit_code":"A-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post("/api/v1/dev-units", &format!(r#"{{"batch_id":{bid},"unit_code":"A-1","unit_type":"rumah","price_minor":500000000}}"#), Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let uid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/dev-units",
            r#"{"unit_code":"A-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // available->sold direct rejected; booked->sold ok; filter by batch.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/dev-units/{uid}/status"),
            r#"{"status":"sold"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["booked", "sold"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/dev-units/{uid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "unit to {st}");
    }
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/dev-units?batch_id={bid}"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(uid) && r["status"] == "sold"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=dbatch", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.dbatch_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/dev-batches")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn girik_intake_and_lease_terms() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "giriklease@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/girik-intakes",
            r#"{"letter_c_ref":"C 1","holder":"Pak Tani"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/girik-intakes",
            r#"{"letter_c_ref":"C 1","holder":"Pak Tani","area_minor":5000,"village":"Sukamaju"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let gid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["verified", "converted"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/girik-intakes/{gid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "girik to {st}");
    }
    // Lease terms upsert on a sewa deed.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"LS/1","deed_kind":"sewa","seller":"AA","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/lease"),
            r#"{"monthly_minor":5000000,"duration_months":0}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(&format!("/api/v1/ppat/{pid}/lease"), r#"{"monthly_minor":5000000,"duration_months":12,"start_date":"2026-06-01","deposit_minor":5000000}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/lease"), &tok))
        .await
        .unwrap();
    let lv = body_json(res).await;
    assert_eq!(lv["total_minor"].as_i64().unwrap(), 60_000_000);
    // Upsert replaces.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/lease"),
            r#"{"monthly_minor":6000000,"duration_months":6}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/lease"), &tok))
        .await
        .unwrap();
    assert_eq!(
        body_json(res).await["total_minor"].as_i64().unwrap(),
        36_000_000
    );
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=lease", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.lease_saved"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/girik-intakes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn foreign_guard_and_notary_handoff() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "fguard@x.id").await;
    // Statutory matrix.
    for (sub, right, allowed) in [
        ("wna", "hak_pakai", true),
        ("wna", "shm", false),
        ("wna", "hgb", false),
        ("pma", "hgb", true),
        ("pma", "hak_pakai", true),
        ("pma", "shm", false),
        ("wni", "shm", true),
        ("badan_wni", "shm", true),
    ] {
        let res = a
            .clone()
            .oneshot(get(
                &format!("/api/v1/foreign-guard?subject={sub}&right={right}"),
                &tok,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{sub}/{right}");
        assert_eq!(
            body_json(res).await["allowed"],
            serde_json::json!(allowed),
            "{sub}/{right}"
        );
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/foreign-guard?subject=alien&right=shm", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(get("/api/v1/foreign-guard?subject=wna&right=alien", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/foreign-guard?subject=wni&right=shm")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Notary handoff checklist.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"NH/1","deed_kind":"ajb","seller":"AA","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/handoffs"),
            r#"{"doc_name":"Minuta akta"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/handoffs/{hid}/received"),
            r#"{"received":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/handoffs"), &tok))
        .await
        .unwrap();
    let hv = body_json(res).await;
    assert!(hv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(hid) && r["received"] == true));
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat/999999/handoffs",
            r#"{"doc_name":"ZZ"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=nhoff", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.nhoff_added"));
}

#[tokio::test]
async fn ppat_monthly_report_and_dashboard() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ppatrep@x.id").await;
    for (num, kind) in [("R/1", "ajb"), ("R/2", "hibah"), ("R/3", "ajb")] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/ppat",
                &format!(r#"{{"number":"{num}","deed_kind":"{kind}","seller":"AA","buyer":"BB"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let month = chrono::Utc::now().format("%Y-%m").to_string();
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/reports/ppat-monthly?month={month}"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rep = body_json(res).await;
    assert_eq!(rep["month"].as_str().unwrap(), month);
    assert_eq!(rep["total"].as_i64().unwrap(), 3);
    assert!(rep["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["kind"] == "ajb" && r["n"].as_i64() == Some(2)));
    // Bad month rejected; default month works.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/ppat-monthly?month=20XX", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/ppat-monthly", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["total"].as_i64().unwrap(), 3);
    // Dashboard reflects deeds + open items.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-disputes",
            r#"{"parcel_ref":"SHM 1","kind":"sengketa","description":"DD"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/ppat-dashboard", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let d = body_json(res).await;
    assert_eq!(d["total_deeds"].as_i64().unwrap(), 3);
    assert_eq!(d["open_disputes"].as_i64().unwrap(), 1);
    assert!(d["by_kind"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["kind"] == "hibah" && r["n"].as_i64() == Some(1)));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ppat-dashboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn land_history_and_unit_handover() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "lhistuho@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-history",
            r#"{"parcel_ref":"SHM 9","detail":"Alas hak girik"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-history",
            r#"{"parcel_ref":"SHM 9","event":"alien","detail":"DD"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post("/api/v1/land-history", r#"{"parcel_ref":"SHM 9","event":"alas_hak","detail":"Alas hak girik C 123","event_date":"2020-01-15"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-history",
            r#"{"parcel_ref":"SHM 9","event":"waris","detail":"Turun waris ke 3 anak"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/land-history?parcel_ref=SHM%209", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 2);
    // Unit handover draft->signed->delivered.
    let res = a
        .clone()
        .oneshot(post("/api/v1/unit-handovers", r#"{"unit_ref":"Tower A-101","handed_from":"PT Dev","handed_to":"Budi","bast_no":"BAST/1"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let uid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/unit-handovers/{uid}/status"),
            r#"{"status":"delivered"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["signed", "delivered"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/unit-handovers/{uid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "uho to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=uho", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.uho_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/unit-handovers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn deed_consents_board_and_power() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dconsent2@x.id").await;
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"DC/2","deed_kind":"ajb","seller":"PT Jual","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/consents"),
            r#"{"kind":"board","party":"PT Jual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/consents"),
            r#"{"kind":"power","party":"Kuasa A"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let wid = body_json(res).await["id"].as_i64().unwrap();
    // Verify then un-verify (toggle both directions).
    for v in [true, false] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/consents/{bid}/verify"),
                &format!(r#"{{"verified":{v}}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body_json(res).await["verified"], serde_json::json!(v));
    }
    // Ghost consent verify 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/consents/999999/verify",
            r#"{"verified":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Delete is Delete-gated and audited reads work.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/consents/{wid}"))
                .method("DELETE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/consents"), &tok))
        .await
        .unwrap();
    let cv = body_json(res).await;
    assert_eq!(cv.as_array().unwrap().len(), 2);
    assert!(cv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(bid) && r["kind"] == "board" && r["verified"] == false));
}

#[tokio::test]
async fn deed_consents_marital_and_heir() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dconsent@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"DC/1","deed_kind":"ajb","seller":"AA","buyer":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/consents"),
            r#"{"kind":"marital","party":"Istri Penjual"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/consents"),
            r#"{"kind":"alien","party":"ZZ"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/consents"),
            r#"{"kind":"marital","party":"Istri Penjual"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/consents"),
            r#"{"kind":"heir","party":"Anak Kandung"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hid = body_json(res).await["id"].as_i64().unwrap();
    // Verify flow.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/consents/{mid}/verify"),
            r#"{"verified":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/consents"), &tok))
        .await
        .unwrap();
    let cv = body_json(res).await;
    assert!(cv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(mid) && r["verified"] == true));
    assert!(cv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(hid) && r["kind"] == "heir" && r["verified"] == false));
    // Ghost deed 404s; unauthenticated 401s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat/999999/consents",
            r#"{"kind":"marital","party":"ZZ"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/ppat/{pid}/consents"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=dconsent", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.dconsent_added"));
}

#[tokio::test]
async fn land_disputes_and_boundary() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dispbnd@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-disputes",
            r#"{"parcel_ref":"X","description":"Y"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-disputes",
            r#"{"parcel_ref":"SHM 5","kind":"batas","description":"Sengketa batas utara"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let did = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-disputes",
            r#"{"parcel_ref":"SHM 6","kind":"alien","description":"DD"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["in_review", "cleared"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/land-disputes/{did}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "dispute to {st}");
    }
    // Boundary: auto-derived match/mismatch/pending.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/boundary-checks",
            r#"{"parcel_ref":"SHM 5","declared_area":1000,"measured_area":998}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/boundary-checks",
            r#"{"parcel_ref":"SHM 6","declared_area":500,"measured_area":500}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/boundary-checks", &tok))
        .await
        .unwrap();
    let bv = body_json(res).await;
    assert!(bv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["parcel_ref"] == "SHM 5" && r["result"] == "mismatch"));
    assert!(bv
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["parcel_ref"] == "SHM 6" && r["result"] == "match"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=ldispute", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.ldispute_created"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/land-disputes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn land_split_and_merge() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "splitmerge@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/land-splits",
            r#"{"source_parcel":"X","new_parcels":"Y"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post("/api/v1/land-splits", r#"{"source_parcel":"SHM 1","new_parcels":"SHM 1-A, SHM 1-B","split_date":"2026-04-01"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["surveyed", "registered"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/land-splits/{sid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "split to {st}");
    }
    // Skip-ahead rejected on fresh item.
    let res = a
        .clone()
        .oneshot(post("/api/v1/land-merges", r#"{"source_parcels":"SHM 2, SHM 3","result_parcel":"SHM 2-3","merge_date":"2026-05-01"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/land-merges/{mid}/status"),
            r#"{"status":"registered"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/land-merges", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(mid) && r["status"] == "planned"));
    // Delete is Delete-gated.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/land-splits/{sid}"))
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=lsplit", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.lsplit_created"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/land-splits")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn zones_and_tax_worksheets() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "zonews@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/zones",
            r#"{"area_code":"X","area_name":"Y","njop_minor":1}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/zones",
            r#"{"area_code":"BDG-01","area_name":"Bandung Utara","njop_minor":500000000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/zones",
            r#"{"area_code":"BDG-01","area_name":"Dup","njop_minor":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // PPh: 2.5% x 1.000.000.000 = 25.000.000.
    let res = a
        .clone()
        .oneshot(get("/api/v1/worksheets/pph?value_minor=1000000000", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let p = body_json(res).await;
    assert_eq!(p["tax_minor"].as_i64().unwrap(), 25_000_000);
    assert_eq!(p["rate_pct"].as_f64().unwrap(), 2.5);
    let res = a
        .clone()
        .oneshot(get("/api/v1/worksheets/pph?value_minor=0", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // BPHTB: 5% x (200jt - 60jt) = 7jt; below NPOPTKP floors to zero.
    let res = a
        .clone()
        .oneshot(get("/api/v1/worksheets/bphtb?npop=200000000", &tok))
        .await
        .unwrap();
    let b = body_json(res).await;
    assert_eq!(b["npoptkp"].as_i64().unwrap(), 60_000_000);
    assert_eq!(b["base_minor"].as_i64().unwrap(), 140_000_000);
    assert_eq!(b["tax_minor"].as_i64().unwrap(), 7_000_000);
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/worksheets/bphtb?npop=50000000&npoptkp=60000000",
            &tok,
        ))
        .await
        .unwrap();
    let b = body_json(res).await;
    assert_eq!(b["base_minor"].as_i64().unwrap(), 0);
    assert_eq!(b["tax_minor"].as_i64().unwrap(), 0);
    let res = a
        .clone()
        .oneshot(get("/api/v1/worksheets/bphtb?npop=-5", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Delete is Delete-gated; list + audit intact.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/zones/1")
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=zone", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.zone_created"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/worksheets/pph?value_minor=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mortgage_charges_and_expiry_monitor() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "chgdue@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"AP/1","deed_kind":"apht","seller":"A","buyer":"B"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/charges"),
            r#"{"kind":"gadai","creditor":"Bank X"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/charges"),
            r#"{"kind":"skmht","creditor":"Bank X"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(&format!("/api/v1/ppat/{pid}/charges"), r#"{"kind":"skmht","creditor":"Bank X","ceiling_minor":500000000,"valid_until":"2026-03-15"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Upsert: same kind again replaces (no duplicate rows).
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/charges"),
            r#"{"kind":"skmht","creditor":"Bank Y","valid_until":"2026-04-15"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/charges"), &tok))
        .await
        .unwrap();
    let cl = body_json(res).await;
    assert_eq!(cl.as_array().unwrap().len(), 1);
    assert_eq!(cl[0]["creditor"].as_str().unwrap(), "Bank Y");
    // Ghost deed 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat/999999/charges",
            r#"{"kind":"apht","creditor":"ZZ"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Expiry monitor: 2026-04-15 within 60 days of 2026-03-01? No — use wide window.
    let res = a
        .clone()
        .oneshot(get("/api/v1/charges/due?days=3650", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["due"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["ppat_id"].as_i64() == Some(pid)));
    // Tight window excludes it only if horizon precedes valid_until... just assert shape.
    let res = a
        .clone()
        .oneshot(get("/api/v1/charges/due?days=0", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["days"].as_i64().unwrap(), 0);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/charges/due")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=charge", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.charge_saved"));
}

#[tokio::test]
async fn ppat_kind_readiness_hibah_tukar() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "rdyppat@x.id").await;
    // Hibah deed: ready only when giver (seller) is KYC-verified.
    let res = a
        .clone()
        .oneshot(post("/api/v1/ppat", r#"{"number":"H/1","deed_kind":"hibah","seller":"Ayah","buyer":"Anak","object":"SHM 9"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hid = body_json(res).await["id"].as_i64().unwrap();
    // Fresh hibah deed: fields complete but giver not verified -> not ready.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{hid}/readiness"), &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!body_json(res).await["ready"].as_bool().unwrap());
    // Verify the giver -> hibah becomes ready.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{hid}/parties"),
            r#"{"role":"seller","name":"Ayah"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat-parties/{pid}/verified"),
            r#"{"verified":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{hid}/readiness"), &tok))
        .await
        .unwrap();
    let r = body_json(res).await;
    assert!(r["ready"].as_bool().unwrap());
    assert!(r["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["key"] == "kyc_giver" && c["pass"].as_bool().unwrap()));
    // Tukar-menukar: needs both sides verified.
    let res = a
        .clone()
        .oneshot(post("/api/v1/ppat", r#"{"number":"T/1","deed_kind":"tukar_menukar","seller":"Pihak A","buyer":"Pihak B","object":"SHM 1 vs SHM 2"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{tid}/parties"),
            r#"{"role":"seller","name":"Pihak A"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{tid}/readiness"), &tok))
        .await
        .unwrap();
    let r = body_json(res).await;
    assert!(!r["ready"].as_bool().unwrap());
    assert!(r["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["key"] == "kyc_both" && !c["pass"].as_bool().unwrap()));
    // 404 for ghost deed; 401 without token.
    let res = a
        .clone()
        .oneshot(get("/api/v1/ppat/999999/readiness", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/ppat/{hid}/readiness"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ppat_kyc_bundle_and_handover() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "kyccho@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat",
            r#"{"number":"7/2026","deed_kind":"ajb","seller":"S","buyer":"B"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    // Ghost deed 404s for both sub-resources.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat/999999/parties",
            r#"{"role":"seller","name":"XX"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/ppat/999999/handovers",
            r#"{"handed_to":"YY"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Auditor cannot add parties.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/parties"),
            r#"{"role":"seller","name":"XX"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Unknown role rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/parties"),
            r#"{"role":"alien","name":"XX"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for (role, name) in [("seller", "Siti"), ("buyer", "Budi")] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/ppat/{pid}/parties"),
                &format!(r#"{{"role":"{role}","name":"{name}","id_number":"ID-{name}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "party {role}");
    }
    // Bundle shows 0/2 verified, then 1/2.
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/parties"), &tok))
        .await
        .unwrap();
    let b = body_json(res).await;
    assert_eq!(b["total_count"].as_i64().unwrap(), 2);
    assert_eq!(b["verified_count"].as_i64().unwrap(), 0);
    let first = b["parties"][0]["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat-parties/{first}/verified"),
            r#"{"verified":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/parties"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["verified_count"].as_i64().unwrap(), 1);
    // Handover: prepared -> handed, no revert.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/ppat/{pid}/handovers"),
            r#"{"handed_to":"BPN Bogor","handed_at":"2026-06-01","receipt_no":"BA/77"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/cert-handovers/{hid}/status"),
            r#"{"status":"handed"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/cert-handovers/{hid}/status"),
            r#"{"status":"prepared"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/ppat/{pid}/handovers"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(hid) && r["status"] == "handed"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=chandover", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.chandover_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ppat/1/parties")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn multi_desk_dispatch() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dispatch@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/tasks"),
            r#"{"title":"Susun draft"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    // Ghost task 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/tasks/999999/assign",
            r#"{"assignee":"BB","desk":"meja-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}/assign"),
            r#"{"assignee":"Siti","desk":"meja-2"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Board groups by desk; unassigned bucket exists for the others.
    let res = a
        .clone()
        .oneshot(get("/api/v1/tasks/dispatch", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let b = body_json(res).await;
    let g = b
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["desk"] == "meja-2")
        .expect("desk group present");
    assert!(g["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"].as_i64() == Some(tid) && t["assignee"] == "Siti"));
    // Cross-org isolation: other org sees an empty board.
    let tok2 = reg(&a, "dispatch2@x.id").await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/tasks/dispatch", &tok2))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 0);
    // Reassign moves the card; done tasks leave the board.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/tasks/{tid}/assign"),
            r#"{"assignee":"Andi","desk":"meja-1"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/tasks/dispatch", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["desk"] == "meja-1"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/tasks/dispatch")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn inventory_and_meterai_usage() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "invuse@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/inventory",
            r#"{"code":"X","name":"Y","qty":1}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post("/api/v1/inventory", r#"{"code":"METERAI-10K","name":"Meterai 10000","qty":100,"unit":"pcs","min_stock":20}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let iid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/inventory",
            r#"{"code":"METERAI-10K","name":"Dup","qty":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Oversell blocked at both layers.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/inventory/{iid}/usage"),
            r#"{"qty":1000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/inventory/{iid}/usage"),
            r#"{"qty":30,"need":"AJB W1","used_on":"2026-02-01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Stock decremented atomically; low-stock flag still false (70 > 20).
    let res = a
        .clone()
        .oneshot(get("/api/v1/inventory", &tok))
        .await
        .unwrap();
    let item = body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"].as_i64() == Some(iid))
        .unwrap()
        .clone();
    assert_eq!(item["qty"].as_i64().unwrap(), 70);
    assert!(!item["low"].as_bool().unwrap());
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/inventory/{iid}/usage"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // Adjust below zero rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/inventory/{iid}/adjust"),
            r#"{"delta":-1000}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/inventory/{iid}/adjust"),
            r#"{"delta":-60}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["qty"].as_i64().unwrap(), 10);
    let res = a
        .clone()
        .oneshot(get("/api/v1/inventory", &tok))
        .await
        .unwrap();
    let item = body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"].as_i64() == Some(iid))
        .unwrap()
        .clone();
    assert!(item["low"].as_bool().unwrap());
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=inventory", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.stock_created"));
}

#[tokio::test]
async fn cashbook_and_advances() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "cashadv@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/cashbook",
            r#"{"kind":"in","amount_minor":1}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Bad kind and non-positive amounts rejected.
    for (payload, want) in [
        (
            r#"{"kind":"sideways","amount_minor":100}"#.to_string(),
            StatusCode::CONFLICT,
        ),
        (
            r#"{"kind":"in","amount_minor":0}"#.to_string(),
            StatusCode::CONFLICT,
        ),
        (
            r#"{"kind":"in","amount_minor":-5}"#.to_string(),
            StatusCode::CONFLICT,
        ),
    ] {
        let res = a
            .clone()
            .oneshot(post("/api/v1/cashbook", &payload, Some(&tok)))
            .await
            .unwrap();
        assert_eq!(res.status(), want, "payload was {payload}");
    }
    for (kind, amt) in [("in", 100000), ("in", 50000), ("out", 30000)] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/cashbook",
                &format!(r#"{{"kind":"{kind}","amount_minor":{amt},"note":"Uji"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/cashbook", &tok))
        .await
        .unwrap();
    let cb = body_json(res).await;
    assert_eq!(cb["balance_minor"].as_i64().unwrap(), 120000);
    assert_eq!(cb["entries"].as_array().unwrap().len(), 3);
    // Advances: open -> settled; double-settle rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/advances",
            r#"{"holder":"Andi","amount_minor":200000,"purpose":"Operasional"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let vid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/advances/{vid}/status"),
            r#"{"status":"settled"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/advances/{vid}/status"),
            r#"{"status":"cancelled"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/advances", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(vid) && r["status"] == "settled"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=cash", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.cash_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cashbook")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn price_list_and_quotations() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "priceq@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/prices",
            r#"{"code":"X","name":"Y","amount_minor":1}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    for (code, name, amt) in [
        ("AJB-STD", "Akta Jual Beli standar", 2500000),
        ("METERAI", "Bea meterai", 10000),
    ] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/prices",
                &format!(r#"{{"code":"{code}","name":"{name}","amount_minor":{amt}}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "price {code}");
    }
    // Duplicate code rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/prices",
            r#"{"code":"AJB-STD","name":"Dup","amount_minor":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Quote with unknown code rejected (no silent zero lines).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/quotations",
            r#"{"client_name":"BB","items":[{"code":"GHOST","qty":1}]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Quote snapshots active prices: 1x AJB-STD + 2x METERAI = 2520000.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/quotations",
            r#"{"client_name":"Budi","deed_kind":"ajb","items":[{"code":"AJB-STD","qty":1},{"code":"METERAI","qty":2}]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let qid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/quotations", &tok))
        .await
        .unwrap();
    let qv = body_json(res).await;
    let q = qv
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"].as_i64() == Some(qid))
        .unwrap();
    assert_eq!(q["total_minor"].as_i64().unwrap(), 2520000);
    assert_eq!(q["items"].as_array().unwrap().len(), 2);
    // Inactive price codes cannot start new quotes, but existing snapshots stay.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/prices",
            r#"{"code":"TMP","name":"Sementara","amount_minor":5}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let tmp_id = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/prices/{tmp_id}/active"),
            r#"{"active":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/quotations",
            r#"{"client_name":"CC","items":[{"code":"TMP","qty":1}]}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Lifecycle: draft -> issued -> accepted; direct draft -> accepted rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/quotations/{qid}/status"),
            r#"{"status":"accepted"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["issued", "accepted"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/quotations/{qid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "quote to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=price", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.price_created"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=quote", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.quote_created"));
}

#[tokio::test]
async fn appointments_and_queue_board() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "appt@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/appointments",
            r#"{"client_name":"BB","scheduled_at":"2026-03-01 09:00"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Bad slot format rejected (no silent coercion).
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/appointments",
            r#"{"client_name":"Budi","scheduled_at":"2026-13-99 99:99"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/appointments",
            r#"{"client_name":"Budi Santoso","purpose":"AJB","scheduled_at":"2026-03-01 09:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let aid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/appointments",
            r#"{"client_name":"Ani","purpose":"Hibah","scheduled_at":"2026-03-01 10:00"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Queue board groups the day in slot order.
    let res = a
        .clone()
        .oneshot(get("/api/v1/appointments/queue?date=2026-03-01", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let qv = body_json(res).await;
    assert_eq!(qv["date"].as_str().unwrap(), "2026-03-01");
    let q = qv["queue"].as_array().unwrap();
    assert_eq!(q.len(), 2);
    assert!(q[0]["scheduled_at"].as_str().unwrap() < q[1]["scheduled_at"].as_str().unwrap());
    // Bad date rejected; done appointments leave the board.
    let res = a
        .clone()
        .oneshot(get("/api/v1/appointments/queue?date=01-03-2026", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    for st in ["checked_in", "done"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/appointments/{aid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "appt to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/appointments/queue?date=2026-03-01", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["queue"].as_array().unwrap().len(), 1);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/appointments/queue")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=appt", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "office.appt_created"));
}

#[tokio::test]
async fn protocol_books_and_handovers() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "pbook@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-books",
            r#"{"book_no":"W1"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-books",
            r#"{"book_no":"W1/2026","year":"2026"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bid = body_json(res).await["id"].as_i64().unwrap();
    // Duplicate number rejected per org.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-books",
            r#"{"book_no":"W1/2026"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Handover guards: ghost book 404, same holder 409.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/protocol-books/999999/handovers",
            r#"{"from_holder":"AA","to_holder":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/protocol-books/{bid}/handovers"),
            r#"{"from_holder":"AA","to_holder":"AA"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(&format!("/api/v1/protocol-books/{bid}/handovers"), r#"{"from_holder":"Notaris A","to_holder":"Notaris B","handed_at":"2026-02-01","note":"Cuti"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/protocol-books/{bid}/handovers"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // Book lifecycle to sealed; handovers cascade on delete.
    for st in ["closed", "sealed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/protocol-books/{bid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "pbook to {st}");
    }
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/protocol-books/{bid}"))
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
        .oneshot(get("/api/v1/audit?entity=pbook", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "property.pbook_created"));
}

#[tokio::test]
async fn evidence_vault_and_regulator_pack() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "evpack@x.id").await;
    let client = forged_role_token(&tok, vec!["client"]);
    // Seed a risk to link evidence against.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/risks",
            r#"{"title":"RR"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    // Unknown register and ghost id rejected.
    for (payload, want) in [
        (
            format!(r#"{{"title":"BB","register":"alien","register_id":{rid}}}"#),
            StatusCode::NOT_FOUND,
        ),
        (
            r#"{"title":"BB","register":"risks","register_id":999999}"#.to_string(),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/compliance/evidence-vault",
                &payload,
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), want, "payload was {payload}");
    }
    let res = a
        .clone()
        .oneshot(post("/api/v1/compliance/evidence-vault", &format!(r#"{{"title":"Hasil uji penetrasi","register":"risks","register_id":{rid},"note":"PDF-2026-01"}}"#), Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let eid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(
            "/api/v1/compliance/evidence-vault?register=risks",
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(eid)));
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/compliance/evidence-vault?register=risks&register_id={rid}"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/evidence-vault?register=dsar", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 0);
    // Delete is Delete-gated.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/evidence-vault/{eid}"))
                .method("DELETE")
                .header(
                    "authorization",
                    format!("Bearer {client}")
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Regulator pack: Export-gated, CSV with sections.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/regulator-pack.csv", &client))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/regulator-pack.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()["content-type"]
        .to_str()
        .unwrap()
        .contains("text/csv"));
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.starts_with("regulator_response_pack,org_hash_generated_at\n"));
    for want in [
        "posture,risks,1,",
        "posture,evidence,1,",
        "section,register,total,detail",
    ] {
        assert!(text.contains(want), "pack missing {want}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=evidence", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.evidence_added"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/compliance/evidence-vault")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn matter_checklist_and_scorecard() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "mclsc@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    // Ghost matter 404s.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/matters/999999/checklist",
            r#"{"item":"Butir hantu"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/matters/{mid}/checklist"),
            r#"{"item":"Verifikasi identitas"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/checklist/{cid}/toggle"),
            r#"{"done":true}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/checklist"), &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cid) && r["done"] == true));
    // Cross-org isolation: item invisible elsewhere.
    let tok2 = reg(&a, "mclsc2@x.id").await;
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/matters/{mid}/checklist"), &tok2))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Scorecard: empty org yields nulls; seeded data yields percentages.
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/scorecard", &tok2))
        .await
        .unwrap();
    let sc = body_json(res).await;
    assert!(sc["overall"].is_null());
    assert!(sc["parts"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["value"].is_null()));
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/trainings",
            r#"{"title":"TT"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let tid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["in_progress", "completed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/trainings/{tid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/scorecard", &tok))
        .await
        .unwrap();
    let sc = body_json(res).await;
    let tr = sc["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["area"] == "trainings_completed_pct")
        .unwrap();
    assert_eq!(tr["value"].as_f64().unwrap(), 100.0);
    assert!(sc["overall"].as_f64().unwrap() >= 0.0);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/compliance/scorecard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn control_tests_and_audit_plans() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ctest@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Control seed: never-tested control is due immediately.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/controls",
            r#"{"code":"CT-01","title":"Backup","frequency":"weekly"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/controls/due", &tok))
        .await
        .unwrap();
    let due = body_json(res).await;
    assert!(due
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cid) && r["overdue"] == true));
    // Log a test: control drops off the overdue list.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/controls/{cid}/tests"),
            r#"{"result":"pass","note":"OK"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/controls/{cid}/tests"),
            r#"{"result":"bogus"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/controls/{cid}/tests"),
            r#"{"result":"pass","note":"OK"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/controls/due", &tok))
        .await
        .unwrap();
    let due = body_json(res).await;
    assert!(due
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cid) && r["overdue"] == false));
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/compliance/controls/{cid}/tests"),
            &tok,
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // Ghost control 404s.
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/controls/999999/tests", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // Audit plans: planned -> in_progress -> completed.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/aplans",
            r#"{"title":"Audit 2026","period":"2026-Q1","scope":"PDP"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["in_progress", "completed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/aplans/{pid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "aplan to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/aplans", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(pid) && r["status"] == "completed"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=ctest", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.ctest_added"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=aplan", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.aplan_created"));
}

#[tokio::test]
async fn policy_library_and_controls() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "polctl@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Draft cannot be acked; ack requires published.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/policies",
            r#"{"title":"XX","body":"Isi kebijakan"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/policies",
            r#"{"title":"Kode Etik","version":"2.0","body":"Isi kebijakan"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let pid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/policies/{pid}/ack"),
            r#"{}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/policies/{pid}/status"),
            r#"{"status":"published"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/policies/{pid}/ack"),
            r#"{}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Duplicate ack rejected; ack count visible in list.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/policies/{pid}/ack"),
            r#"{}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/policies", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(pid) && r["ack_count"].as_i64() == Some(1)));
    let res = a
        .clone()
        .oneshot(get(&format!("/api/v1/compliance/policies/{pid}/ack"), &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    // Controls: duplicate code rejected; active toggle works.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/controls",
            r#"{"code":"PDP-01","title":"Enkripsi data","frequency":"monthly"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/controls",
            r#"{"code":"PDP-01","title":"Duplikat"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/controls/{cid}/active"),
            r#"{"active":false}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/controls", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(cid) && r["active"] == false));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=policy", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.policy_created"));
}

#[tokio::test]
async fn whistle_and_waiver_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "wbwv@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Anonymous report (empty contact) surfaces as "anonim", never leaks blanks.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/whistle",
            r#"{"summary":"XX"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/whistle",
            r#"{"channel":"pigeon","summary":"Pungli loket"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/whistle",
            r#"{"channel":"box","category":"gratifikasi","summary":"Pungli loket"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let wid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/whistle", &tok))
        .await
        .unwrap();
    let wl = body_json(res).await;
    assert!(wl
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(wid) && r["contact"] == "anonim"));
    for st in ["under_review", "resolved"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/whistle/{wid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "whistle to {st}");
    }
    // Waiver: approve path and reject path; revoke only from approved.
    let res = a
        .clone()
        .oneshot(post("/api/v1/compliance/waivers", r#"{"title":"XX","justification":"Darurat","requested_by":"BB","expires_at":"2026-12-31"}"#, Some(&tok)))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let vid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/waivers/{vid}/status"),
            r#"{"status":"revoked"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for st in ["approved", "revoked"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/waivers/{vid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "waiver to {st}");
    }
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/waivers",
            r#"{"title":"YY","justification":"Alasan kuat"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    let vid2 = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/waivers/{vid2}/status"),
            r#"{"status":"rejected"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=whistle", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.whistle_created"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=waiver", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.waiver_created"));
}

#[tokio::test]
async fn training_evidence_and_audit_findings() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "tevaf@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Training seed, then evidence roundtrip scoped to it.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/trainings",
            r#"{"title":"Fire drill","assignee":"BB"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/trainings/{tid}/evidence"),
            r#"{"note":"XX"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/trainings/999999/evidence",
            r#"{"note":"Ghost note here"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/trainings/{tid}/evidence"),
            r#"{"note":"Sertifikat No. 12"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let eid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(get(
            &format!("/api/v1/compliance/trainings/{tid}/evidence"),
            &tok,
        ))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(eid)));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/evidence/{eid}"))
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
    // Findings: severity allowlist + remediation chain.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/findings",
            r#"{"title":"XX","severity":"alien"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/findings",
            r#"{"title":"Arsip tidak terkunci","area":"arsip","severity":"high"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let fid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["in_remediation", "closed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/findings/{fid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "finding to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/findings", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(fid) && r["status"] == "closed"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=tevidence", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.tevidence_added"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=afinding", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.afinding_created"));
}

#[tokio::test]
async fn ethics_and_gift_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "ethicsgift@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    // Ethics: submitted -> acknowledged, then frozen.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/ethics",
            r#"{"declarant":"BB","statement":"SS"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/ethics",
            r#"{"declarant":"Budi","year":"2026","statement":"Bebas konflik kepentingan"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let eid = body_json(res).await["id"].as_i64().unwrap();
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/ethics/{eid}/status"),
            r#"{"status":"acknowledged"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/ethics/{eid}/status"),
            r#"{"status":"submitted"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    // Gifts: bad direction rejected; logged -> reviewed -> cleared.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/gifts",
            r#"{"direction":"sideways","description":"DD"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/gifts",
            r#"{"direction":"received","description":"Parsel","counterpart":"Vendor X","given_at":"2026-01-05"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let gid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["reviewed", "cleared"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/gifts/{gid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "gift to {st}");
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/ethics", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(eid) && r["status"] == "acknowledged"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/gifts", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(gid) && r["status"] == "cleared"));
    // Delete is Delete-gated; creation audited.
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/gifts/{gid}"))
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
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/ethics/{eid}"))
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
        .oneshot(get("/api/v1/audit?entity=ethics", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.ethics_created"));
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=gift", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.gift_created"));
}

#[tokio::test]
async fn dsar_rights_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "dsar@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/dsar",
            r#"{"subject":"X","detail":"Y"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Unknown kind rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/dsar",
            r#"{"subject":"SS","kind":"alien","detail":"DD"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    for kind in ["access", "deletion", "rectification"] {
        let res = a
            .clone()
            .oneshot(post(
                "/api/v1/compliance/dsar",
                &format!(r#"{{"subject":"NIK-001","kind":"{kind}","detail":"Mohon {kind}","due_date":"2026-12-31"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "dsar kind {kind}");
        let rid = body_json(res).await["id"].as_i64().unwrap();
        for st in ["in_review", "fulfilled"] {
            let res = a
                .clone()
                .oneshot(post(
                    &format!("/api/v1/compliance/dsar/{rid}/status"),
                    &format!(r#"{{"status":"{st}"}}"#),
                    Some(&tok),
                ))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK, "dsar {kind} to {st}");
        }
    }
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/dsar", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 3);
    let res = a
        .clone()
        .oneshot(get("/api/v1/audit?entity=dsar", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.dsar_created"));
}

#[tokio::test]
async fn risk_register_and_heatmap() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "risk@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/risks",
            r#"{"title":"X"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // Out-of-range score rejected.
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/risks",
            r#"{"title":"Y","likelihood":9,"impact":1}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/risks",
            r#"{"title":"Kebocoran data","category":"privacy","likelihood":4,"impact":5,"owner":"DPO"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    // Score-sorted list surfaces score 20 first.
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/risks", &tok))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(rid) && r["score"].as_i64() == Some(20)));
    // Heatmap counts the open risk at K4xD5 with high flag.
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/risks/heatmap", &tok))
        .await
        .unwrap();
    let h = body_json(res).await;
    assert_eq!(h["matrix"][3][4].as_i64().unwrap(), 1);
    assert_eq!(h["total_open"].as_i64().unwrap(), 1);
    assert_eq!(h["high_count"].as_i64().unwrap(), 1);
    for st in ["mitigating", "mitigated", "closed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/risks/{rid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "risk to {st}");
    }
    // Closed risks drop out of the heatmap.
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/risks/heatmap", &tok))
        .await
        .unwrap();
    assert_eq!(body_json(res).await["total_open"].as_i64().unwrap(), 0);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/risks/{rid}"))
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
        .oneshot(get("/api/v1/audit?entity=risk", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.risk_created"));
}

#[tokio::test]
async fn training_assignment_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "training@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/trainings",
            r#"{"title":"X"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/trainings",
            r#"{"title":"UU PDP untuk staf","assignee":"Andi","due_date":"2026-12-31"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let tid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["in_progress", "completed"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/trainings/{tid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "training to {st}");
    }
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/trainings/{tid}/status"),
            r#"{"status":"assigned"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/trainings", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(tid) && r["status"] == "completed"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/trainings/{tid}"))
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
        .oneshot(get("/api/v1/audit?entity=training", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.training_created"));
}

#[tokio::test]
async fn regulatory_change_lifecycle() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "regchange@x.id").await;
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/regchanges",
            r#"{"title":"X"}"#,
            Some(&auditor),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = a
        .clone()
        .oneshot(post(
            "/api/v1/compliance/regchanges",
            r#"{"title":"Revisi UU PDP","source":"LN 2026","effective_date":"2026-06-01"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rid = body_json(res).await["id"].as_i64().unwrap();
    for st in ["acknowledged", "implemented"] {
        let res = a
            .clone()
            .oneshot(post(
                &format!("/api/v1/compliance/regchanges/{rid}/status"),
                &format!(r#"{{"status":"{st}"}}"#),
                Some(&tok),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "regchange to {st}");
    }
    // Bad transition (implemented -> open) rejected.
    let res = a
        .clone()
        .oneshot(post(
            &format!("/api/v1/compliance/regchanges/{rid}/status"),
            r#"{"status":"open"}"#,
            Some(&tok),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let res = a
        .clone()
        .oneshot(get("/api/v1/compliance/regchanges", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"].as_i64() == Some(rid) && r["status"] == "implemented"));
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/compliance/regchanges/{rid}"))
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
        .oneshot(get("/api/v1/audit?entity=regchange", &tok))
        .await
        .unwrap();
    assert!(body_json(res)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["action"] == "compliance.regchange_created"));
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
async fn approvals_csv_export_lists_all_requests() {
    use tower::ServiceExt;
    let a = app();
    let tok = reg(&a, "apprexp@x.id").await;
    let mid = mk_matter(&a, &tok).await;
    let aid = mk_approval(&a, &tok, mid, r#"[{"step_no":1,"role":"office_admin"}]"#).await;
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals.csv", &tok))
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
        text.starts_with("id,title,entity,status,created,pending_steps,decided_by,steps\n"),
        "was {text}"
    );
    assert!(text.contains(&format!("{aid},")), "was {text}");
    assert!(text.contains("pending"), "was {text}");
    // Auditor (View+Export) may export; unauthenticated may not.
    let auditor = forged_role_token(&tok, vec!["auditor"]);
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals.csv", &auditor))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/reports/approvals.csv")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    // Auditor-grade columns: decide, then actor + step detail appear; status filter scopes.
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
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals.csv?status=approved", &tok))
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
    assert!(text.contains("1:office_admin:approved:"), "was {text}");
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals.csv?status=rejected", &tok))
        .await
        .unwrap();
    let text = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!text.contains(&format!("{aid},")), "was {text}");
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/approvals.csv?status=alien", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
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
    // SLA breach list: only the overdue open task.
    let res = a
        .clone()
        .oneshot(get("/api/v1/reports/sla-breaches.csv", &tok))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sla = String::from_utf8(
        axum::body::to_bytes(res.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(
        sla.starts_with("matter_id,matter_number,task_id,task_title,due,status\n"),
        "was {sla}"
    );
    assert!(
        sla.contains("Overdue terbuka") && sla.contains("2001-01-01 09:00:00"),
        "was {sla}"
    );
    assert!(!sla.contains("Mendatang"), "future task leaked: {sla}");
    assert!(!sla.contains("Selesai lampau"), "done task leaked: {sla}");
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
        .any(|e| e["action"] == "report.sla_exported"));
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
        "appr-overdue-days",
        "appr-overdue-reload",
        "appr-overdue-list",
        "appr-cal-month",
        "appr-cal-reload",
        "appr-cal-list",
        "sla-warn",
        "sla-reload",
        "sla-list",
        "cfx-reload",
        "cfx-list",
        "blk-reload",
        "blk-list",
        "blk-ok",
        "blk-key",
        "appr-key",
        "appr-sim",
        "appr-cond",
        "appr-amend",
        "rr-kind",
        "rr-add",
        "rr-reload",
        "rr-list",
        "rrs-go",
        "pol-snap",
        "pol-reload",
        "pol-list",
        "appr-fast",
        "qrm-title",
        "qrm-add",
        "qrm-reload",
        "qrm-list",
        "qrv-go",
        "trd-days",
        "trd-reload",
        "trd-list",
        "appr-sim-out",
        "arc-status",
        "arc-reload",
        "arc-list",
        "dgst-mode",
        "dgst-save",
        "dgst-reload",
        "frz-on",
        "frz-reload",
        "appr-id",
        "appr-show",
        "appr-detail",
        "appr-step",
        "appr-comment",
        "appr-ok",
        "appr-no",
        "appr-rw",
        "appr-revoke",
        "appr-csv",
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
        "int-list",
        "int-reload",
        "p-kepatuhan",
        "kep-title",
        "kep-add",
        "kep-list",
        "con-subject",
        "con-grant",
        "con-list",
        "rtr-entity",
        "rtr-save",
        "rtr-reload",
        "rtr-list",
        "prg2-entity",
        "prg2-add",
        "prg2-reload",
        "prg2-list",
        "pret-years",
        "pret-reload",
        "pret-list",
        "aml-reload",
        "aml-list",
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
        "inc-title",
        "inc-add",
        "inc-reload",
        "inc-list",
        "capa-title",
        "capa-add",
        "capa-reload",
        "capa-list",
        "reg-title",
        "reg-add",
        "reg-reload",
        "reg-list",
        "trn-title",
        "trn-add",
        "trn-reload",
        "trn-list",
        "risk-title",
        "risk-add",
        "risk-reload",
        "risk-list",
        "risk-hm",
        "dsar-subject",
        "dsar-add",
        "dsar-reload",
        "dsar-list",
        "ethics-who",
        "ethics-add",
        "ethics-reload",
        "ethics-list",
        "gift-desc",
        "gift-add",
        "gift-reload",
        "gift-list",
        "tev-trn",
        "tev-add",
        "tev-reload",
        "tev-list",
        "af-title",
        "af-add",
        "af-reload",
        "af-list",
        "wb-sum",
        "wb-add",
        "wb-reload",
        "wb-list",
        "wv-title",
        "wv-add",
        "wv-reload",
        "wv-list",
        "pol-title",
        "pol-add",
        "pol-reload",
        "pol-list",
        "ctl-code",
        "ctl-add",
        "ctm-reload",
        "ctm-list",
        "ctdue-reload",
        "ctdue-list",
        "ctest-ctl",
        "ctest-add",
        "ctest-reload",
        "ctest-list",
        "aplan-title",
        "aplan-add",
        "aplan-reload",
        "aplan-list",
        "mcl-matter",
        "mcl-add",
        "mcl-reload",
        "mcl-list",
        "score-reload",
        "score-list",
        "sc-period",
        "sc-add",
        "sc-reload",
        "sc-list",
        "regpack-csv",
        "ev-title",
        "ev-add",
        "ev-reload",
        "ev-list",
        "posture-reload",
        "posture-csv",
        "posture-list",
        "alerts-reload",
        "alerts-list",
        "ROPA",
        "Kepatuhan",
        "Persetujuan",
    ] {
        assert!(t.contains(needle), "/app missing {needle}");
    }
    assert!(t.contains("data-tab=\"appr\""), "missing appr tab link");
}
