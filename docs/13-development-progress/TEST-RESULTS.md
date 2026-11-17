# TEST RESULTS
## 2026-11-17 — full workspace (SQLX_OFFLINE=true, isolated target dir, no live DB)
- `cargo fmt --all -- --check`: PASS. - `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **171 passed, 0 failed** (server API 33 incl. 13 marketing + trust double-entry; marketing store 11; AI 16; OCR 16; mobile 12; auth 12; app pages; full crate matrix).
- Live (same day, :8080 + Laragon proxy): Tabler landing + vendor CSS + ComfyUI hero/CTA PNGs 200; /public/pricing + /public/faqs 200; H1 verified in served HTML.
- Live-DB (same day, Laragon MySQL 8.4.9): migrations 0001–0009 applied to `legalrust_drill`; `live_mysql_register_and_matter` PASS (1.28s); row counts organizations=1 users=1 roles=12 permissions=12.
- DR restore drill (same day, local): mysqldump→`legalrust_staging` restore OK, counts match, dump deleted post-drill.
- Benchmarks (same day, dev machine i.e. Ultra 7 155H, debug, ephemeral store, n=200/20, concurrency 1): /healthz p50 0.45ms p95 4.61ms; /version p50 0.30ms; 404 p50 0.29ms; register 486ms / login 587ms (argon2); full table in `docs/30-BENCHMARK-PROTOCOLS/BENCHMARKS.md`.
- Ignored (CI live-DB): `api_db::live_mysql_register_and_matter` (run manually instead — PASS, see above). Blocked: engine-backed OCR/AI, production deploy.
