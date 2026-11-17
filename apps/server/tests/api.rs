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
