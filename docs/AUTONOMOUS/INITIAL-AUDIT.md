# INITIAL-AUDIT — LegalRust repo (formal, read-only)

Date: 2026-11-18. Auditor: autonomous subagent (F-AUDIT). No code/migration edits made.

## 1. Repo identity
- Branch: `main`. HEAD: `d4c4c5d` ("Docs: wave 5 test results + daily log").
- Toolchain: `rustc 1.99.0`, `cargo 1.99.0`.
- Working tree at audit time was DIRTY (uncommitted): `M apps/server/src/lib.rs`, `M apps/server/static/site.css`, `M apps/server/tests/api.rs`, plus untracked `docs/AUTONOMOUS/`, `docs/MARKETING/LANDING-*.md`, `scripts/autonomous/`. Numbers below reflect the working tree, not HEAD.
- Rule baseline: `AGENTS.md` — 100% Rust app code; forward-only migrations; RBAC server-side per endpoint; no secrets in code/logs; external-dependent features return 501 + docs; progress states NOT_STARTED…ACCEPTED.

## 2. Workspace members (resolver 2, `apps/*` + `crates/*`)
Apps (4): `legalrust-mobile`, `legalrust-server` (lib+binary), `legalrust-web` (leptos 0.8 SSR stub), `legalrust-worker`.
Crates (16): ai, application, audit, auth, authorization, database, document, domain, infrastructure, integrations, notification, ocr, rag, reporting, storage, workflow.
- `apps/server` (3,532-line `lib.rs` at first measure; working tree now larger) depends on 13 internal crates: application, infrastructure, database, auth, domain, authorization, workflow, rag, integrations, reporting, document, storage, audit. External: axum, tokio(full), tower/tower-http, serde, sqlx(mysql), uuid, validator, chrono, anyhow, tracing.
- `apps/worker` (50-line `main.rs`) depends on infrastructure + database + sqlx + tokio + tracing only.
- `crates/auth` deps: argon2, rand_core, jsonwebtoken, chrono, uuid, thiserror, anyhow + internal database/application. `crates/database`: sqlx(mysql,migrate,chrono,uuid), tokio(full), async-trait, thiserror, serde, chrono, uuid.
- `apps/web` is a leptos-SSR placeholder (no routes wired into server binary). `apps/mobile` is a serde/chrono/uuid sync-core only.

## 3. Routes (`apps/server/src/lib.rs`, ripgrep count: 100 `.route(` call sites)
- Site/public ≈ 29: `/healthz`, `/readyz`, `/`, `/login`, `/app`, `/fitur` ×7, `/solusi` ×5, `/harga`, `/tentang`, `/kontak`, `/demo`, `/artikel` ×2, `/faq`, `/kebijakan-privasi`, `/syarat-ketentuan`, `/sitemap.xml`, `/robots.txt`, `/static/{*path}` (+ `.fallback(not_found)`).
- API/admin ≈ 71 under `/api/v1`: auth/orgs/tokens (9: version, register, login, logout, me, tokens ×2, password, user-roles); core (7: clients, matters, matter-tasks, task-patch, documents ×3); OCR (5); deeds/templates (7); AI/RAG (4); finance (7: matter-invoices, invoice-get, payments, pay-gateway, reports-csv, webhooks, integrations); collab (14: notifications ×2, shares ×2, shared ×2, reports matters.csv, audit, jobs, trust ×4, SSE events); public marketing (9); admin marketing (10).
- Security layers: `RequestBodyLimitLayer`, request-id + `TraceLayer`, permissive CORS, `nosniff`/`DENY`/`referrer-policy`/CSP `default-src 'self'`/HSTS headers.

## 4. Migrations `0001–0010` (40 CREATE TABLE, forward-only, no destructive edits seen)
| File | Content |
|---|---|
| 0001_foundation | orgs, users, roles, permissions, role_permissions, user_roles, sessions, api_tokens, audit_events (9) |
| 0002_core | clients, matters, documents, document_versions, tasks (5) |
| 0003_deeds_finance | deeds, deed_parties, invoices, payments (4) |
| 0004_client_optional | ALTER: `matters.client_id` NULL (intake-first) |
| 0005_ocr | ocr_jobs, ocr_fields, ocr_reviews (3) |
| 0006_deeds | deed_templates (1) |
| 0007_ai_finance | ai_conversations, ai_messages, ai_usage, rag_chunks (4) |
| 0008_ops | notifications, document_shares, jobs (3) |
| 0009_trust | trust_accounts, trust_entries (2) |
| 0010_marketing | leads, demo_requests, contact_messages, newsletter_subscribers, lead_notes, follow_ups, pricing_plans, faqs, articles (9) |

## 5. Stores architecture (dual backend)
- Traits: `UserStore` + `CoreStore` (`crates/database/src/store.rs`, 131 lines) plus `OpsStore` (marketing/ops/audit/jobs/trust/AI/RAG/shares).
- Backends: `memory.rs` (365 lines, `MemoryStores`/`MemoryOps`, default dev/test) and `mysql.rs` (513 lines) + `ops_mysql.rs` (live pool). `FullState { version, pool: Option<MySqlPool>, stores }`; server runs with or without DB (`pool: None` → memory path; `/readyz` reflects DB state).
- `dev_state()` wires memory stores + `LocalStore(temp_dir/lr-dev-objects)` + 36-byte `test-secret…` JWT key.

## 6. AuthN/Z summary
- AuthN: Argon2id password hashing; `MIN_PASSWORD_LEN = 12`, `SESSION_TTL_HOURS = 8`, `MIN_SECRET_LEN = 32` (all hardened 2026-11-17, unit-tested). JWT (`Claims { sub, org, roles, sid, exp }`) + revocable server-side sessions (`session_live` checked per request in `auth_user`) + SHA-256-hashed API-token fallback + `LoginGuard` (5 fails/300 s window → 900 s lockout, per-key in-memory).
- AuthZ: every mutating/query handler takes `AuthUser` from `auth_user(headers, state)`; all queries are org-scoped (`org_id`/`tenant_id` bound into every SQL); role assignment endpoint exists (`POST /api/v1/users/{uid}/roles`); admin marketing endpoints gate through `admin_auth` + `admin_lead_scope`. Roles surfaced on login page: org_owner, office_admin, notary/ppat, lawyer, finance_officer.
- Audit: `audit_events` written on mutations, readable only via `GET /api/v1/audit` (authorized roles).

## 7. Test totals (read-only run: `SQLX_OFFLINE=true`, `CARGO_TARGET_DIR=target-audit`, `cargo test --workspace --offline`)
- RESULT: **197 passed, 0 failed, 1 ignored**, across unit + integration + doc-test binaries. 0 failures.
- Breakdown: unit libs 122 (ai 16, application 5, audit 3, auth 12, authorization 1, database 9, document 3, domain 14, infrastructure 1, integrations 19, mobile-lib 12, notification 3, ocr 16, rag 5, reporting 3); integration `apps/server/tests/api.rs` **59 passed**; remaining binaries 2+12+2 (storage/worker/mobile suites); 1 ignored.
- DISCREPANCY: `docs/AUTONOMOUS/MASTER-PROGRESS.md` claims "187 passed". The +10 delta is attributable to working-tree additions (marketing admin endpoints + tests, uncommitted `M apps/server/tests/api.rs`). MASTER-PROGRESS is stale relative to the working tree.

## 8. TIMESTAMP fix verification (commit `1b79105`)
- Claim: `DATE_FORMAT(...)`/`UNIX_TIMESTAMP` string reads replaced binary TIMESTAMP decodes. VERIFIED PRESENT: 6 `DATE_FORMAT(expires_at|created_at|due_at,'%Y-%m-%d %H:%i:%s')` sites in `crates/database/src/ops_mysql.rs` (lines ~579, 593, 681–683, 988, 1012), consumed as strings and parsed via `chrono::NaiveDateTime::parse_from_str` (`apps/server/src/lib.rs:1533`).
- Remaining `NaiveDateTime` hits (8 total) are all type-level and benign: `store.rs:67` trait signature, `memory.rs:8,67` in-memory map, `mysql.rs:110` session-insert param, `ops_mysql.rs:550,1060` `Option<NaiveDateTime>` string-parse targets, `auth/lib.rs:111` `expiry()`, `lib.rs:1533` parser. No `query_as!` TIMESTAMP struct decodes remain. No `get::<NaiveDateTime>` on raw TIMESTAMP columns.
- Known related risk: string-format reads silently yield `None`/parse-fail rows instead of hard errors — acceptable, but parse failures should be logged (see RISK-REGISTER R-08).

## 9. Known issues / dead-code candidates
- TODO/FIXME/`todo!`/`unimplemented!`: **zero hits** across `apps/server`, `apps/worker`, all crates. Clean.
- `apps/web` (leptos) and `apps/mobile` (sync-core) are unintegrated stubs — not dead code, but unshipped surface; must not be presented as working apps.
- `crates/authorization`, `crates/notification`, `crates/reporting`, `crates/storage`, `crates/workflow` are thin types/helpers; real logic lives in `lib.rs` handlers + `ops_*` stores (see RISK-REGISTER R-01 single-file-server growth, R-07 provider stub drift).
- External-gated 501s (honest, by design): `ocr/process`, `ai/chat`, `deeds/esign`, `invoices/pay-gateway` + worker `ocr.process` no-executor path. These are BLOCKED items (BL-EXT-01), not bugs.
- Stale-doc risk: MASTER-PROGRESS test count (187) vs measured (197); ACCEPTANCE 28/32=87/100 claim predates Wave 5 additions — re-baseline needed before Wave 7 re-audit.
