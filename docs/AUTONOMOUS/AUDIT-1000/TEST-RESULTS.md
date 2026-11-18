# TEST-RESULTS (AUDIT-1000 wave 2026-11-18, updated wave 8: durable retry queue)

Gates (SQLX_OFFLINE=true):
- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- node --check site.js: PASS
- `cargo test --workspace`: **299 passed, 0 failed** (noted: 1 transient stub-timing failure seen once, unreproduced in 3 full reruns) (server API incl. compliance lifecycle + store tests; webhook deliveries + store tenant test;
  3 approvals + trust CSV + approvals UI needles; integrations 30; OCR 21;
  AI 19; full matrix)

New coverage this wave:
- `approvals_sequential_flow` (out-of-order 409, close, re-decide 409,
  revoke-closed 409, list filter, history)
- `approvals_rework_revoke_roundtrip` (rework reset, approve, revoke)
- `approvals_validation_and_rbac` (entity/steps/role 422s, unknown deed 404,
  auditor-create 403, lawyer/notary decide 403s, notary-own-step 200,
  bad filter 422)
- Report-audit emission covered by existing CSV tests (output unchanged).

Ignored (live-DB): `api_db::live_mysql_register_and_matter` (manual PASS
recorded earlier). Blocked: PSrE provider, prod deploy/benchmarks.
