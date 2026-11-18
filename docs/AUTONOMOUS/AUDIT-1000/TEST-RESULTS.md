# TEST-RESULTS (AUDIT-1000 wave 2026-11-18)

Gates (SQLX_OFFLINE=true):
- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo test --workspace`: **288 passed, 0 failed** (server API 105 incl.
  3 approvals tests; integrations 30; OCR 21; AI 19; full matrix)

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
