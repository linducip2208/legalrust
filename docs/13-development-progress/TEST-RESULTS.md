# TEST RESULTS
## 2026-11-17 — full workspace (SQLX_OFFLINE=true, isolated target dir, no live DB)
- `cargo fmt --all -- --check`: PASS. - `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **89 passed, 0 failed** (server API incl. auth dual-mode, tokens, password, roles, notifications, shares+public download, reports CSV, audit viewer, jobs, trust double-entry + overdraft guard, SSE hello, rate-limit 429, webhook HMAC, OCR/deed/RAG/invoice flows; crate unit tests incl. trust double-entry atomicity).
- Ignored (CI live-DB): `api_db::live_mysql_register_and_matter`. Blocked: live migration run, engine-backed OCR/AI, DR drill.
