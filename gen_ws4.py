#!/usr/bin/env python3
"""Part 3: apps, migrations, deploy, new docs structure, workstream registry."""
from pathlib import Path
import json
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

# ---------- apps/server ----------
w("apps/server/Cargo.toml", """[package]
name = "legalrust-server"
version = "0.1.0"
edition = "2021"

[lib]
name = "legalrust_server"
path = "src/lib.rs"

[[bin]]
name = "legalrust-server"
path = "src/main.rs"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.7", features = ["trace", "cors", "request-id"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
legalrust-application = { path = "../../crates/application" }
legalrust-infrastructure = { path = "../../crates/infrastructure" }
legalrust-database = { path = "../../crates/database" }
legalrust-auth = { path = "../../crates/auth" }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
""")
w("apps/server/src/lib.rs", r'''
use axum::{Json, Router, extract::State, http::HeaderName, routing::{get, post}};
use legalrust_application::{ApiResponse, AppError};
use legalrust_infrastructure::AppState;
use serde::Deserialize;
use serde_json::{Value, json};
use tower_http::{cors::CorsLayer, request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer}, trace::TraceLayer};
use validator::Validate;

const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Deserialize, Validate)]
struct LoginRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

async fn healthz() -> Json<Value> {
    Json(json!({"data": {"status": "ok", "service": "legalrust-server"}, "meta": {"version": "v1"}}))
}

async fn version(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::new(json!({"version": state.version}), "bootstrap".into()))
}

async fn readyz(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    match &state.pool {
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => Ok(Json(json!({"ready": true}))),
            Err(_) => Err(AppError::BlockedExternal("database unreachable".into())),
        },
        None => Err(AppError::BlockedExternal("database not configured".into())),
    }
}

async fn login(Json(body): Json<LoginRequest>) -> Result<Json<Value>, AppError> {
    body.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    Err(AppError::BlockedExternal("user store offline in scaffold; Phase 4 wires DB-backed login".into()))
}

async fn not_found() -> AppError { AppError::NotFound("route".into()) }

pub fn build_router(state: AppState) -> Router {
    let request_id = HeaderName::from_static(REQUEST_ID_HEADER);
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/version", get(version))
        .route("/api/v1/auth/login", post(login))
        .fallback(not_found)
        .layer(SetRequestIdLayer::new(request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(request_id))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub fn dev_state() -> AppState { AppState { version: "0.1.0", pool: None } }
''')
w("apps/server/src/main.rs", r'''
use legalrust_database as db;
use legalrust_infrastructure::{AppState, Config, init_tracing};
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let config = Config::from_env();
    if !config.jwt_secret_set {
        tracing::warn!("JWT_SECRET is not set; auth tokens are disabled until configured");
    }
    let pool = match tokio::time::timeout(Duration::from_secs(3), db::connect(&config.database_url)).await {
        Ok(Ok(p)) => {
            tracing::info!("database connected; running migrations");
            if let Err(e) = db::migrate(&p).await { tracing::error!("migration failed: {e}"); }
            Some(p)
        }
        _ => {
            tracing::warn!("database unreachable; serving without live pool (see /readyz)");
            None
        }
    };
    let state = AppState { version: "0.1.0", pool };
    let app = legalrust_server::build_router(state);
    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!("listening on {}", config.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await?;
    Ok(())
}
''')
w("apps/server/tests/api.rs", r'''
use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;
#[tokio::test]
async fn healthz_ok() {
    let app = legalrust_server::build_router(legalrust_server::dev_state());
    let res = app.oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
#[tokio::test]
async fn unknown_route_is_json_404() {
    let app = legalrust_server::build_router(legalrust_server::dev_state());
    let res = app.oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn login_validates_then_blocks_without_db() {
    use axum::body::to_bytes;
    let app = legalrust_server::build_router(legalrust_server::dev_state());
    let res = app.oneshot(Request::builder().uri("/api/v1/auth/login").method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"email":"not-an-email","password":"short"}"#)).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = to_bytes(res.into_body(), 8192).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("VALIDATION"));
}
''')

# ---------- apps/worker ----------
w("apps/worker/Cargo.toml", """[package]
name = "legalrust-worker"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
legalrust-infrastructure = { path = "../../crates/infrastructure" }
""")
w("apps/worker/src/main.rs", r'''
use legalrust_infrastructure::init_tracing;
use std::time::Duration;
#[tokio::main]
async fn main() {
    init_tracing();
    tracing::info!("worker started (durable job wiring lands in Phase 3; no in-memory critical tasks)");
    let mut tick = tokio::time::interval(Duration::from_secs(30));
    loop {
        tick.tick().await;
        tracing::info!("worker heartbeat: 0 due jobs (broker adapter pending configuration)");
    }
}
''')

# ---------- apps/web ----------
w("apps/web/Cargo.toml", """[package]
name = "legalrust-web"
version = "0.1.0"
edition = "2021"

[lib]
name = "legalrust_web"
path = "src/lib.rs"

[dependencies]
leptos = { version = "0.8", features = ["ssr"] }
""")
w("apps/web/src/lib.rs", r'''
use leptos::prelude::*;
#[component]
pub fn App() -> impl IntoView {
    view! {
        <main>
            <h1>"LEGALRUST"</h1>
            <p>"Enterprise Legal Operations — workspace shell (Phase 5 builds full UI)."</p>
            <nav>
                <ul>
                    <li>"Dashboard (pending Phase 6 data)"</li>
                    <li>"Clients (pending)"</li>
                    <li>"Documents (pending)"</li>
                </ul>
            </nav>
        </main>
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ssr_renders_shell() {
        let html = leptos::ssr::render_to_string(App);
        assert!(html.contains("LEGALRUST"), "shell must render brand");
        assert!(html.contains("Phase 5"), "shell must state its phase honestly");
    }
}
''')

# ---------- migrations ----------
w("migrations/0001_foundation.sql", """-- 0001 foundation: organizations, identity, RBAC, sessions, audit
CREATE TABLE organizations (
  id CHAR(36) NOT NULL PRIMARY KEY,
  name VARCHAR(200) NOT NULL, status VARCHAR(20) NOT NULL DEFAULT 'active',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE users (
  id CHAR(36) NOT NULL PRIMARY KEY, org_id CHAR(36) NOT NULL,
  email VARCHAR(255) NOT NULL, password_hash VARCHAR(255) NOT NULL,
  full_name VARCHAR(200) NOT NULL, status VARCHAR(20) NOT NULL DEFAULT 'active',
  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_users_org_email (org_id, email),
  CONSTRAINT fk_users_org FOREIGN KEY (org_id) REFERENCES organizations (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE roles (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, name VARCHAR(60) NOT NULL,
  UNIQUE KEY uq_roles_org_name (org_id, name),
  CONSTRAINT fk_roles_org FOREIGN KEY (org_id) REFERENCES organizations (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE permissions (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(60) NOT NULL UNIQUE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE role_permissions (
  role_id BIGINT UNSIGNED NOT NULL, permission_id BIGINT UNSIGNED NOT NULL,
  PRIMARY KEY (role_id, permission_id),
  CONSTRAINT fk_rp_role FOREIGN KEY (role_id) REFERENCES roles (id) ON DELETE CASCADE,
  CONSTRAINT fk_rp_perm FOREIGN KEY (permission_id) REFERENCES permissions (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE user_roles (
  user_id CHAR(36) NOT NULL, role_id BIGINT UNSIGNED NOT NULL,
  PRIMARY KEY (user_id, role_id),
  CONSTRAINT fk_ur_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_ur_role FOREIGN KEY (role_id) REFERENCES roles (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE sessions (
  id CHAR(36) NOT NULL PRIMARY KEY, user_id CHAR(36) NOT NULL,
  expires_at TIMESTAMP NOT NULL, revoked TINYINT(1) NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_sessions_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  KEY ix_sessions_user (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE api_tokens (
  id CHAR(36) NOT NULL PRIMARY KEY, user_id CHAR(36) NOT NULL,
  name VARCHAR(120) NOT NULL, token_hash CHAR(64) NOT NULL,
  expires_at TIMESTAMP NULL, revoked TINYINT(1) NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_tokens_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE audit_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id CHAR(36) NOT NULL, actor_id CHAR(36) NULL,
  action VARCHAR(120) NOT NULL, entity VARCHAR(120) NOT NULL, entity_id BIGINT NULL,
  before_hash CHAR(64) NULL, after_hash CHAR(64) NULL, ip VARCHAR(64) NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_audit_tenant_entity (tenant_id, entity, entity_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
""")
w("migrations/0002_core.sql", """-- 0002 core: clients, matters, documents, tasks
CREATE TABLE clients (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, kind VARCHAR(20) NOT NULL DEFAULT 'individual',
  display_name VARCHAR(255) NOT NULL, status VARCHAR(20) NOT NULL DEFAULT 'active',
  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_clients_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_clients_org_status (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE matters (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, client_id BIGINT UNSIGNED NOT NULL,
  number VARCHAR(40) NOT NULL, title VARCHAR(300) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'open',
  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_matters_org_number (org_id, number),
  CONSTRAINT fk_matters_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_matters_client FOREIGN KEY (client_id) REFERENCES clients (id),
  KEY ix_matters_org_status (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE documents (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, matter_id BIGINT UNSIGNED NULL,
  name VARCHAR(255) NOT NULL, mime VARCHAR(120) NOT NULL, size_bytes BIGINT UNSIGNED NOT NULL,
  sha256 CHAR(64) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'active',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_docs_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_docs_matter FOREIGN KEY (matter_id) REFERENCES matters (id),
  KEY ix_docs_matter (matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE document_versions (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  document_id BIGINT UNSIGNED NOT NULL, version_no INT UNSIGNED NOT NULL,
  object_key VARCHAR(500) NOT NULL, sha256 CHAR(64) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uq_docver (document_id, version_no),
  CONSTRAINT fk_docver_doc FOREIGN KEY (document_id) REFERENCES documents (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE tasks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, matter_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(300) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'open',
  due_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_tasks_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_tasks_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_tasks_matter_status (matter_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
""")
w("migrations/0003_deeds_finance.sql", """-- 0003 deeds + finance
CREATE TABLE deeds (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, matter_id BIGINT UNSIGNED NOT NULL,
  number VARCHAR(60) NOT NULL, deed_type VARCHAR(60) NOT NULL, state VARCHAR(40) NOT NULL DEFAULT 'draft',
  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_deeds_org_number (org_id, number),
  CONSTRAINT fk_deeds_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_deeds_matter FOREIGN KEY (matter_id) REFERENCES matters (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE deed_parties (
  deed_id BIGINT UNSIGNED NOT NULL, client_id BIGINT UNSIGNED NOT NULL, role VARCHAR(40) NOT NULL,
  PRIMARY KEY (deed_id, client_id, role),
  CONSTRAINT fk_dp_deed FOREIGN KEY (deed_id) REFERENCES deeds (id) ON DELETE CASCADE,
  CONSTRAINT fk_dp_client FOREIGN KEY (client_id) REFERENCES clients (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE invoices (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, matter_id BIGINT UNSIGNED NOT NULL,
  number VARCHAR(60) NOT NULL, total_minor BIGINT NOT NULL DEFAULT 0, currency CHAR(3) NOT NULL DEFAULT 'IDR',
  status VARCHAR(40) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uq_invoices_org_number (org_id, number),
  CONSTRAINT fk_inv_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_inv_matter FOREIGN KEY (matter_id) REFERENCES matters (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE payments (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  invoice_id BIGINT UNSIGNED NOT NULL, amount_minor BIGINT NOT NULL,
  method VARCHAR(40) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'recorded',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_pay_invoice FOREIGN KEY (invoice_id) REFERENCES invoices (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
""")

# ---------- deploy ----------
w("deploy/Dockerfile.server", """FROM rust:1-bookworm AS build
WORKDIR /app
COPY . .
RUN cargo build --release -p legalrust-server
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/legalrust-server /usr/local/bin/
CMD ["legalrust-server"]
""")
w("deploy/README.md", """# Deploy (Linux)
1. Provision MySQL 8.4, Redis, S3-compatible storage. 2. Set env (see .env.example; JWT_SECRET from vault).
3. Run migrations (sqlx). 4. Deploy server + worker binaries (systemd/k8s). 5. TLS terminate at LB, AES-256 at rest, encrypted backups with restore drills.
""")
print("PART3A OK")
