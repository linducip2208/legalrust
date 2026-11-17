# TEST RESULTS
## 2026-11-17 — full suite (SQLX_OFFLINE=true, no live DB)
- `cargo fmt --all -- --check`: PASS. - `cargo check --workspace`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **31 passed, 0 failed** (server api 8 incl. full_flow_memory/ocr/deed/rag-invoice/ai-blocked/webhook+rate-limit; auth 3; ai 3; app 2; doc 2; ocr 2; workflow 2; 9×1).
- Ignored (CI live-DB): `api_db::live_mysql_register_and_matter`.
- Blocked: live migration run, provider-backed OCR/AI, DR drill.
