use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
#[tokio::test]
async fn healthz_ok() {
    let app = legalrust_server::build_router(legalrust_server::dev_state());
    let res = app
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
async fn unknown_route_is_json_404() {
    let app = legalrust_server::build_router(legalrust_server::dev_state());
    let res = app
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn login_validates_then_blocks_without_db() {
    use axum::body::to_bytes;
    let app = legalrust_server::build_router(legalrust_server::dev_state());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"not-an-email","password":"short"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = to_bytes(res.into_body(), 8192).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("VALIDATION"));
}
