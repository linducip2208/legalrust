//! Live-MySQL verification. Run: DATABASE_URL=mysql://root:legalrust@127.0.0.1:3306/legalrust_test cargo test -- --ignored
use axum::{body::Body, http::Request};
use tower::ServiceExt;
#[tokio::test]
#[ignore]
async fn live_mysql_register_and_matter() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let pool = legalrust_database::connect(&url).await.expect("connect");
    legalrust_database::migrate(&pool).await.expect("migrate");
    let m = std::sync::Arc::new(legalrust_database::mysql::MysqlStores::new(pool.clone()));
    let o = std::sync::Arc::new(legalrust_database::ops_mysql::MysqlOps::new(pool.clone()));
    let state = legalrust_server::FullState {
        version: "test",
        pool: Some(pool),
        stores: legalrust_server::Stores::new(
            m.clone(),
            m,
            o.clone(),
            o.clone(),
            o,
            std::sync::Arc::new(legalrust_storage::LocalStore::new(
                std::env::temp_dir().join("lr-db-test"),
            )),
            b"ci-secret-0123456789abcdef01234567".to_vec(),
        ),
    };
    let app = legalrust_server::build_router(state);
    let email = format!("ci{}@x.id", uuid::Uuid::new_v4());
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/register")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"org_name":"CI","email":"{email}","name":"CI","password":"password1-2-3"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::OK);
}
