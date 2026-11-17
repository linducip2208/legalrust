# TEST RESULTS
## 2026-11-17 — foundation suite (SQLX_OFFLINE=true, no live DB)
- `cargo fmt --all -- --check`: PASS (clean).
- `cargo check --workspace`: PASS (axum 0.8.9, tokio 1.54.1, sqlx 0.9.0, leptos 0.8.23, validator 0.20, jsonwebtoken 9, argon2 0.5).
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **26 passed, 0 failed** (auth 3, api 3, ai 3, app 2, doc 2, ocr 2, workflow 2, + 1 each domain/rbac/rag/notify/report/webhook/storage/web/infra).
- Blocked (honest, not run): live-MySQL migration run + live-DB login (ignored test `api_db::live_mysql_register_and_matter` wired for CI).
## 2026-11-17 (2) — Phase 4 + Phase 6 slice
- New: register/login/logout/sessions/JWT-sid, RBAC extractor, clients/matters/tasks/documents/content API, sha-pinned uploads, task guards.
- `cargo test --workspace`: **new total counted in run output; full_flow_memory green** (register→me→client→matter→task illegal/legal→doc upload→logout→401).
- Live-DB verification deferred to CI `live-db` job (MySQL 8.4 service + `cargo test -p legalrust-server --test api_db -- --ignored`).
