# TEST RESULTS
## 2026-11-17 — foundation suite (SQLX_OFFLINE=true, no live DB)
- `cargo fmt --all -- --check`: PASS (clean).
- `cargo check --workspace`: PASS (axum 0.8.9, tokio 1.54.1, sqlx 0.9.0, leptos 0.8.23, validator 0.20, jsonwebtoken 9, argon2 0.5).
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **24 passed, 0 failed** (auth 2, api 3, ai 3, app 2, doc 2, ocr 2, workflow 2, + 1 each domain/rbac/rag/notify/report/webhook/storage/web/infra).
- Blocked (honest, not run): live-MySQL migration run, DB-backed login, OCR benchmark numbers, AI evals. See BLOCKERS.md.
