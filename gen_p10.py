#!/usr/bin/env python3
"""Final push part 7: router wiring, state construction, security headers, dev_state."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def edit(rel, old, new, count=1):
    p = R/rel; t = p.read_text(encoding="utf-8")
    assert old in t, f"anchor missing in {rel}: {old[:70]}"
    p.write_text(t.replace(old, new, count), encoding="utf-8", newline="\n")

# set-header import already applied manually; skipping.

# router: add routes before fallback
edit("apps/server/src/lib.rs",
 '''        .route(
            "/api/v1/documents/{id}/content",
            put(put_content).get(get_content),
        )
        .fallback(not_found)''',
 '''        .route("/api/v1/documents/{id}/content", put(put_content).get(get_content))
        .route("/api/v1/ocr/jobs", post(ocr_create).get(ocr_list))
        .route("/api/v1/ocr/jobs/:id", get(ocr_get))
        .route("/api/v1/ocr/jobs/:id/review", post(ocr_review))
        .route("/api/v1/ocr/jobs/:id/process", post(ocr_process))
        .route("/api/v1/deed-templates", post(tpl_create).get(tpl_list))
        .route("/api/v1/deed-templates/:id", put(tpl_update))
        .route("/api/v1/deeds/from-template", post(deed_from_template))
        .route("/api/v1/deeds", get(deed_list))
        .route("/api/v1/deeds/:id/transition", post(deed_transition))
        .route("/api/v1/ai/chat", post(ai_chat))
        .route("/api/v1/ai/usage", get(ai_usage))
        .route("/api/v1/rag/index", post(rag_index))
        .route("/api/v1/rag/search", get(rag_search))
        .route("/api/v1/matters/:mid/invoices", post(inv_create).get(inv_list))
        .route("/api/v1/invoices/:id", get(inv_get))
        .route("/api/v1/invoices/:id/payments", post(pay_add))
        .route("/api/v1/webhooks/incoming/:provider", post(webhook_in))
        .route("/api/v1/integrations", get(integrations))
        .fallback(not_found)''')

# security headers
edit("apps/server/src/lib.rs",
 '''        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)''',
 '''        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(SetResponseHeaderLayer::if_not_present(HeaderName::from_static("x-content-type-options"), HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(HeaderName::from_static("x-frame-options"), HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(HeaderName::from_static("referrer-policy"), HeaderValue::from_static("strict-origin-when-cross-origin")))
        .with_state(state)''')

# dev_state with ops
edit("apps/server/src/lib.rs",
 '''pub fn dev_state() -> FullState {
    let m = std::sync::Arc::new(legalrust_database::memory::MemoryStores::default());
    FullState {
        version: "0.1.0",
        pool: None,
        stores: Stores {
            users: m.clone(),
            core: m,
            objects: Arc::new(LocalStore::new(std::env::temp_dir().join("lr-dev-objects"))),
            jwt_secret: b"test-secret".to_vec(),
        },
    }
}''',
 '''pub fn dev_state() -> FullState {
    let m = std::sync::Arc::new(legalrust_database::memory::MemoryStores::default());
    let o = std::sync::Arc::new(legalrust_database::ops_memory::MemoryOps::default());
    let mut stores = Stores::new(m.clone(), m, o, Arc::new(LocalStore::new(std::env::temp_dir().join("lr-dev-objects"))), b"test-secret".to_vec());
    stores.integration_secret = b"test-integration".to_vec();
    FullState { version: "0.1.0", pool: None, stores }
}''')

# main.rs state construction
edit("apps/server/src/main.rs",
 '''    let (users, core): (Arc<dyn legalrust_database::UserStore>, Arc<dyn legalrust_database::CoreStore>) = match &pool {
        Some(p) => {
            let m = Arc::new(legalrust_database::mysql::MysqlStores::new(p.clone()));
            (m.clone(), m)
        }
        None => {
            let m = Arc::new(legalrust_database::memory::MemoryStores::default());
            (m.clone(), m)
        }
    };
    let state = FullState {
        version: "0.1.0",
        pool,
        stores: Stores {
            users,
            core,
            objects: Arc::new(LocalStore::new(std::path::PathBuf::from("./var/objects")))),
            jwt_secret: secret,
        },
    };''',
 '''    let (users, core, ops): (
        Arc<dyn legalrust_database::UserStore>,
        Arc<dyn legalrust_database::CoreStore>,
        Arc<dyn legalrust_database::OpsStore>,
    ) = match &pool {
        Some(p) => {
            let m = Arc::new(legalrust_database::mysql::MysqlStores::new(p.clone()));
            let o = Arc::new(legalrust_database::ops_mysql::MysqlOps::new(p.clone()));
            (m.clone(), m, o)
        }
        None => {
            let m = Arc::new(legalrust_database::memory::MemoryStores::default());
            let o = Arc::new(legalrust_database::ops_memory::MemoryOps::default());
            (m.clone(), m, o)
        }
    };
    let mut stores = Stores::new(
        users, core, ops,
        Arc::new(LocalStore::new(std::path::PathBuf::from("./var/objects"))),
        secret,
    );
    stores.integration_secret = std::env::var("INTEGRATION_SECRET").unwrap_or_default().into_bytes();
    let state = FullState { version: "0.1.0", pool, stores };''')

# api_db test state construction
edit("apps/server/tests/api_db.rs",
 '''    let m = std::sync::Arc::new(legalrust_database::mysql::MysqlStores::new(pool.clone()));
    let state = legalrust_server::FullState {
        version: "test",
        pool: Some(pool),
        stores: legalrust_server::Stores {
            users: m.clone(),
            core: m,
            objects: std::sync::Arc::new(legalrust_storage::LocalStore::new(std::env::temp_dir().join("lr-db-test"))),
            jwt_secret: b"ci-secret".to_vec(),
        },
    };''',
 '''    let m = std::sync::Arc::new(legalrust_database::mysql::MysqlStores::new(pool.clone()));
    let o = std::sync::Arc::new(legalrust_database::ops_mysql::MysqlOps::new(pool.clone()));
    let state = legalrust_server::FullState {
        version: "test",
        pool: Some(pool),
        stores: legalrust_server::Stores::new(
            m.clone(), m, o,
            std::sync::Arc::new(legalrust_storage::LocalStore::new(std::env::temp_dir().join("lr-db-test"))),
            b"ci-secret".to_vec(),
        ),
    };''')
print("P7 OK")
