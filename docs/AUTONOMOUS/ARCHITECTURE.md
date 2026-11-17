# ARCHITECTURE — LegalRust (as of 2026-11-18, working tree)

## 1. Request flow (ASCII)

```
Browser/Client
  │  HTTPS (Laragon proxy :80 → server :8080; HSTS+CSP+nosniff headers)
  ▼
Axum Router (apps/server/src/lib.rs :: build_router, ~100 routes + fallback)
  │  layers: RequestBodyLimit · RequestId/Trace · permissive CORS · security headers
  ├─ GET  site pages ──► inline-HTML handlers ──► /static/* (site.css, site.js, PNGs, fonts, vendor)
  ├─ GET  /api/v1/public/*, /api/v1/admin/* (marketing pipeline)
  └─ ANY  /api/v1/* ──► auth_user(headers, state) ──► 401 on failure
        │  1. Bearer JWT → validate_token(jwt_secret) → session_live(sid)?
        │  2. else SHA-256(token) → token_owner → user_roles
        ▼  AuthUser { claims { sub, org, roles, sid, exp } }
Handler (same file) ──► stores.users | stores.ops (traits)
        │  every query binds org_id / tenant_id  ◄── tenant isolation
        ├─ pool: Some(MySqlPool) → mysql.rs / ops_mysql.rs → MySQL (migrations 0001–0010)
        └─ pool: None → memory.rs / ops_memory.rs (dev/test/demo)
        ▼  All multi-step mutations run in DB transactions (AGENTS.md rule)
Response: JSON (API) · HTML shell + external site.css/site.js (pages; no inline style/script per CSP)
Side channels: SSE /api/v1/events · audit_events writes · jobs table → worker polls
```

## 2. Crate dependency graph

```
apps/server ─┬─► crates/auth ────────► crates/database ──► sqlx(mysql) ──► MySQL 8 (Laragon)
             │        │                      ▲  ▲
             │        └─► crates/application  │  │
             ├─► crates/authorization ─► crates/domain
             ├─► crates/workflow / rag / integrations / reporting / document / storage / audit
             ├─► crates/database (UserStore/CoreStore/OpsStore traits)
             └─► axum · tokio · tower-http · validator · uuid · chrono
apps/worker ─► crates/infrastructure (tracing) + crates/database (OpsStore::job_claim/job_done)
apps/web ────► leptos 0.8 SSR (STUB — not wired into server binary)
apps/mobile ─► serde/serde_json/uuid/chrono only (sync-core STUB)
External infra (behind Rust adapters, BLOCKED until creds): libtesseract/cloud OCR,
  AI provider (hyper client stub-tested), PSrE e-sign, payment gateway, Dukcapil/BPN.
```

## 3. Store traits (crates/database)
- `UserStore`: org/owner create, find_by_email, session create/revoke/live, roles assign/list, API tokens (create/list/revoke/owner-by-hash), password set.
- `CoreStore`: clients, matters (+tasks), documents (+versions, SHA-256 pinned object keys via `LocalStore`/storage adapters).
- `OpsStore` (ops_mysql/ops_memory): OCR jobs/fields/reviews, deed templates + transitions, AI conversations/messages/usage metering, RAG chunks (keyword search live), invoices + anti-overpay payments, webhooks, notifications, shares, audit reads, jobs queue, trust accounts/entries/transfers, marketing pipeline (leads/notes/follow-ups/contacts/demo/pricing/faqs/articles).
- `StoreError { NotFound, Conflict(String), Db(String) }` → mapped to typed `AppError` (no panics on expected paths; 501 via `AppError::BlockedExternal`).

## 4. RBAC enforcement points
1. `auth_user()` (lib.rs:134) — single choke point: JWT+`session_live` or token-hash lookup; every protected handler calls it first.
2. Org scoping — `claims.org` bound as `org_id`/`tenant_id` parameter in ALL store queries (both backends).
3. Role checks — `claims.roles` consulted on privileged actions (admin marketing via `admin_auth`+`admin_lead_scope`, audit viewer, role assignment, deed sign/approve transitions).
4. Brute-force — `LoginGuard` 5/300 s → 900 s lock in front of login.
5. Secrets — `check_secret` (≥32 B) gates register/login/token issue; JWT secret never logged (`Session` Debug redacts token; token errors are generic "token error").

## 5. Background worker (apps/worker, 50 lines)
- Loop: 5 s tick → `OpsStore::job_claim()` → match `job.kind`: `notify.digest` (marks done), `ocr.process` (explicit fail: "no OCR engine available"), else "no executor". `job_done(id, ok, note)` persists outcome. No broker: the `jobs` table IS the queue; worker idles honestly with zero pool (`DATABASE_URL` unset/unreachable).

## 6. Static assets (apps/server/static, served at /static/{*path})
- `site.css` (139 lines, single external stylesheet; CSP-safe, no inline styles), `site.js` (degradation-proof reveal + app logic), PNGs: hero-office, office-interior, meeting-room, library, courthouse, doc-scan, ocr-scan, signature, security, server-room, ai-assist, cta-building; `fonts/`, `vendor/` (Tabler/self-hosted).
- Pages reference assets by URL; reveal animations degrade gracefully without JS (verified via headless screenshot per Wave 5 log).
