# FINAL REPORT — LegalRust autonomous waves (implementation)

Date: 2026-11-18. Scope: working tree at `D:\project laravel\legalrust`
(no code/migration edits made for this report; read-only recount + docs).
Companion re-audit: `docs/AUTONOMOUS/REAUDIT-REPORT.md` (Wave 7, 0 mismatch / 40 sampled).

Verdict up front: **28/32 = 87/100 — NOT production-ready** (unchanged; see §15).
Reason list unchanged: OCR/AI engines need credentials, e-sign PSrE needs
credentials, payment gateway needs credentials, no independent security audit,
no production deploy / prod benchmarks / prod-DB run, no counsel sign-off.

## 1. Repo audit summary (from `docs/AUTONOMOUS/INITIAL-AUDIT.md`)

- HEAD `d4c4c5d`, branch `main`, toolchain rustc/cargo 1.99.0. Workspace:
  4 apps (`server`, `worker`, `web` Leptos SSR stub, `mobile` sync-core stub)
  + 16 crates. `apps/server` depends on 13 internal crates + axum/tokio/
  tower-http/serde/sqlx-mysql/uuid/validator/chrono/anyhow/tracing.
- Router: ~100 `.route(` call sites at audit time; current grep: **93**
  (64 unique `/api/*` + ~24 public/site + `/static`, fallback 404).
  Layers: RequestBodyLimit, RequestId/Trace, permissive CORS, security
  headers (nosniff, DENY, referrer-policy, CSP `default-src 'self'`, HSTS).
- Stores: `UserStore` + `CoreStore` + `OpsStore` traits; dual backend
  `memory.rs` (dev/test/demo) and `mysql.rs`/`ops_mysql.rs` (live pool).
  `FullState { version, pool: Option<MySqlPool>, stores }`; server runs with
  or without DB (`pool: None` → memory path; `/readyz` reflects DB state).
- Hygiene: zero `TODO/FIXME/todo!/unimplemented!` in apps/crates; only
  `.env.example` + `.env.prod.example` tracked (no secret `.env` committed);
  `target-*/scratch` not tracked; migrations 0001–0010 unique, no dupes.
- Working-tree drift at report time (NOT committed, NOT counted in §5/§6):
  `M apps/server/src/lib.rs, site.css, site.js, tests/api.rs`,
  `M crates/database/src/{lib,ops,ops_memory}.rs`,
  `M crates/domain/src/lib.rs`, plus untracked `migrations/0011_crm.sql`
  (CRM: `client_meetings` + client tags — parallel-workstream work, never
  applied to any DB, excluded from all counts).
- `cargo fmt --all -- --check` FAILS today (exit 1): 2 files need rewrapping
  only (`crates/database/src/lib.rs:33` import list,
  `crates/database/src/ops.rs:472` tag_attach/detach signatures). No code
  edits made for this report, so left as-is — run `cargo fmt --all` before
  the next test gate.

## 2. Architecture changes (from `ARCHITECTURE.md` + `DECISIONS.md`)

- Request flow: Browser → Laragon proxy (:80) → Axum `:8080` → `auth_user`
  (JWT + `session_live` or SHA-256 API-token fallback) → handler → stores
  (org-scoped queries on both backends) → JSON API / inline-HTML pages +
  external `site.css`/`site.js` (CSP-safe, no inline style/script except
  JSON-LD + 2 external scripts). Side channels: SSE `/api/v1/events`,
  `audit_events` writes, `jobs` table polled by worker.
- Worker (`apps/worker`, ~50 lines): 5 s tick → `job_claim()` →
  `notify.digest` done / `ocr.process` explicit fail / else "no executor";
  no broker (the `jobs` table IS the queue); idles honestly with zero pool.
- Decisions recorded: server-rendered Axum pages (not Leptos SSR) for SEO;
  Tabler self-hosted (CSS+JS+49 woff2, local fonts, zero CDN); CSP
  `default-src 'self'` kept; MySQL TIMESTAMP read via
  `DATE_FORMAT`/`UNIX_TIMESTAMP` (never raw `NaiveDateTime` decode);
  password min 12 / session 8 h / JWT secret min 32 B; honest-label rule for
  mockups/illustrations; ComfyUI/SDXL stills primary, WAN video optional;
  migrations strictly sequential forward-only.

## 3. 25-domain status table (from `FEATURE-IMPLEMENTATION-MATRIX.md`; n=40 each)

| Domain | Name | IMPL | PART | CFG | PLAN | NOT |
|---|---|---|---|---|---|---|
| D01 | Identity & KYC | 0 | 6 | 9 | 14 | 11 |
| D02 | Client & CRM | 4 | 2 | 1 | 11 | 22 |
| D03 | Project & Case Mgmt | 4 | 5 | 1 | 14 | 16 |
| D04 | Document Mgmt | 7 | 7 | 2 | 9 | 15 |
| D05 | OCR & Doc Intelligence | 0 | 6 | 25 | 4 | 5 |
| D06 | Deed Generation | 2 | 10 | 2 | 11 | 15 |
| D07 | AI Legal Assistant | 1 | 1 | 28 | 4 | 6 |
| D08 | Workflow Automation | 0 | 3 | 4 | 14 | 19 |
| D09 | Approval Mgmt | 0 | 0 | 1 | 24 | 15 |
| D10 | Electronic Signature | 0 | 2 | 27 | 3 | 8 |
| D11 | Compliance & Risk | 0 | 1 | 3 | 13 | 23 |
| D12 | Notary Office Mgmt | 0 | 3 | 0 | 19 | 18 |
| D13 | PPAT & Property Legal | 0 | 3 | 3 | 17 | 17 |
| D14 | Finance & Accounting | 2 | 2 | 2 | 18 | 16 |
| D15 | Billing & Subscription | 2 | 3 | 3 | 17 | 15 |
| D16 | User, Role & Permission | 2 | 7 | 4 | 10 | 17 |
| D17 | Multi-Office & Tenant | 0 | 4 | 0 | 22 | 14 |
| D18 | Communication & Notify | 2 | 4 | 7 | 12 | 15 |
| D19 | Client & Public Portal | 4 | 5 | 3 | 13 | 15 |
| D20 | Reporting & Analytics | 0 | 4 | 1 | 16 | 19 |
| D21 | API & Integration | 1 | 8 | 17 | 8 | 6 |
| D22 | Security & Data Protection | 6 | 4 | 3 | 12 | 15 |
| D23 | Search & Discovery | 0 | 9 | 8 | 7 | 16 |
| D24 | Mobile & Field Ops | 0 | 0 | 2 | 22 | 16 |
| D25 | Enterprise Admin & Expansion | 0 | 5 | 3 | 12 | 20 |

Top live-or-partial: D04 (14), D06 (12), D22/D03/D16/D19/D21/D23 (9–10).
Most provider-gated: D07 (28), D10 (27), D05 (25), D21 (17).

## 4. 1000-feature breakdown counts (from `FEATURE-REGISTRY.json`, recounted)

- IMPLEMENTED **37** (3.7%), PARTIAL **104** (10.4%),
  CONFIGURATION_REQUIRED **159** (15.9%), PLANNED **326** (32.6%),
  NOT_STARTED **374** (37.4%). Total **1000**. Matches matrix §3.
- Wave 7 re-audit (fresh sample 40/1000, offset 12, every 25th):
  0 mismatches; sample distribution consistent with population.
- Rule used: IMPLEMENTED = live 200-path route + store op + table, non-501;
  PARTIAL = usable subset; CONFIG = delivery path exists but needs external
  provider; otherwise PLANNED/NOT_STARTED (conservative default).

## 5. DB changes 0001–0010 (40 tables, forward-only)

| File | Content |
|---|---|
| 0001_foundation | orgs, users, roles, permissions, role_permissions, user_roles, sessions, api_tokens, audit_events (9) |
| 0002_core | clients, matters, documents, document_versions, tasks (5) |
| 0003_deeds_finance | deeds, deed_parties, invoices, payments (4) |
| 0004_client_optional | ALTER `matters.client_id` NULL (intake-first) |
| 0005_ocr | ocr_jobs, ocr_fields, ocr_reviews (3) |
| 0006_deeds | deed_templates (1) |
| 0007_ai_finance | ai_conversations, ai_messages, ai_usage, rag_chunks (4) |
| 0008_ops | notifications, document_shares, jobs (3) |
| 0009_trust | trust_accounts, trust_entries (2) |
| 0010_marketing | leads, demo_requests, contact_messages, newsletter_subscribers, lead_notes, follow_ups, pricing_plans, faqs, articles (9) |

Live-DB drill (Laragon MySQL 8.4.9): migrations applied to `legalrust_drill`;
`live_mysql_register_and_matter` PASS (1.28 s); row counts
organizations=1 users=1 roles=12 permissions=12. DR restore drill to
`legalrust_staging` OK (counts match; dump deleted post-drill).
Untracked `migrations/0011_crm.sql` (client_meetings + tags) is NOT part of
this sequence — unreviewed, unapplied, excluded from counts.
Follow-up: `.env` DATABASE_URL password does not match local server
(root/empty) — align before shared runs.

## 6. API changes (count incl.)

- Current grep: **93** `.route(` sites → **64 unique `/api/*` paths**
  (+ ~24 public/site routes, `/static`, fallback). INITIAL-AUDIT counted
  ~71–72 API/admin routes (incl. method variants/public-marketing/admin).
  `docs/04-api-specification/openapi.yaml` present (covers 60+ routes).
- New since 28/32 baseline: `GET /api/v1/search` (tenant-scoped keyword
  search, `global_search` lib.rs:1107, route :3486, wired to /app search box
  site.js:98-108); admin marketing pipeline (~23 public+admin endpoints:
  lead CRUD/status/assign/notes/follow-ups, demo/contact/subscribe/pricing/
  faq/article CMS) with `admin_auth` + `admin_lead_scope` + audit writes.
- Honest 501s (by design, CONFIGURATION_REQUIRED): `ocr/process`,
  `ai/chat`, `deeds/esign`, `invoices/pay-gateway` (+ worker
  `ocr.process` no-executor path). RAG keyword search + token metering live;
  binary/binary-parse RAG input rejected with clear error.

## 7. UI/UX changes

- Tabler self-hosted (vendor CSS/JS, 49 woff2 icons, local fonts, zero CDN);
  CSP-safe shells (no inline style/script; JSON-LD only).
- Landing: hero with `hero-loop.mp4` + `hero-office.png` poster, honest-status
  banner (28/32, 4 named 501s), dashboard mockup labeled illustration-only,
  mega-menu (Fitur×7, Solusi×5), mobile details-menu, footer (privasi/syarat/
  FAQ/sitemap/status). 22 public pages + `/sitemap.xml` + `/robots.txt` +
  canonical/OG/JSON-LD (16 public/SEO tests green; live 200s incl. pricing/
  faqs/articles seeded).
- `/login`: Masuk/Daftar tabs, brute-force note, **demo-account button**
  (site.js:39-42) + **roles/rights matrix table** (org_owner, office_admin,
  notary/ppat, lawyer, finance_officer, …) (lib.rs:2125).
- `/app` dashboard: sidebar nav (Dashboard, KYC/OCR, Klien+MKT, Perkara,
  Kalender, Dokumen, Akta, Keuangan, Marketing), 7 tabs incl. `mkt` +
  `ringkas`; ringkas fetches clients/matters/documents/deeds/trust/ocr live
  (site.js:220-222) and renders trust totals + invoice outstanding
  (site.js:299); global search box → `/api/v1/search` grouped results;
  marketing tab: leads pipeline (filter/advance/assign/notes), demo, kontak,
  newsletter, pricing toggle, FAQ add, articles publish, CSV export
  (401 → /login).
- Assets: 12 PNG stills + `hero-loop.mp4` in `apps/server/static/`
  (ComfyUI RealVisXL reviewed/approved; WAN 2.1 T2V confirmed present but
  unused per owner choice — stills primary).

## 8. Security findings (self-audit + re-audit R-01)

- `docs/14-SECURITY/SELF-AUDIT.md` (2026-11-17, scope read server+crates,
  fixes limited to `crates/auth` + `crates/application`): all `/api` routes
  carry `auth_user` + `authorize` except documented self-scoped/HMAC/
  capability-URL exceptions; 100+ sqlx `.bind` sites, zero
  `format!("SELECT|INSERT|UPDATE|DELETE` interpolations; upload allowlist +
  50 MiB cap + SHA-256 pin; `AppError::public_message()` prevents DB-detail
  leaks; login lockout 5/300 s → 900 s + 20/min global limiter; secrets
  env-only + redacted Debug + 32 B minimum. Fixed then: password 8→12,
  session 24 h→8 h, JWT-secret min 32 B, token-error redaction.
- Progress since self-audit: residual §6.6 (axum DTO `length(min=8)` vs auth
  12) FIXED — `RegisterReq`/`LoginReq`/`PasswordReq` all `length(min=12)`
  (lib.rs:219,227,1431-1434; re-audit §6.1 PASS). Residual §6.7
  (`legalrust-ai` hyper/tokio mismatch) FIXED — ai 16 tests green again.
- REAUDIT R-01 (MEDIUM, still open): `notify_list` decoded raw
  `read_at TIMESTAMP NULL → Option<NaiveDateTime>` (`ops_mysql.rs` old :550).
  **Code fix present**: query now uses `UNIX_TIMESTAMP(read_at) AS read_unix`
  (`ops_mysql.rs:542`); `preferred_at` reads a DATETIME column (supported).
  BUT the MySQL notify path was never executed live (all API tests use
  `MemoryStores`, `pool: None`); if the integer-decode assumption is wrong,
  `GET /notifications` 500s on live MySQL. FIX-before-prod: run notify
  create+list+read against live MySQL (same drill pattern as
  `live_mysql_register_and_matter`).
  (Note: numbering differs from `RISK-REGISTER.md` R-01 = single-file-server
  growth; both tracked.)
- Re-audit risk points: 5/6 PASS (auth min-length, CSP inline, .env hygiene,
  migration dupes, target/scratch hygiene); 2 LOW accepted (registry line
  drift, CSP without explicit `script-src`); 1 INFO (uncommitted test file
  hygiene). Full external audit (SAST/DAST/WAF/pentest, hash-chained audit,
  KMS) still NOT_STARTED/PLANNED → checklist item stays PARTIAL.

## 9. Perf numbers (dev-machine table — NOT production claims)

Source: `docs/30-BENCHMARK-PROTOCOLS/BENCHMARKS.md` §31 (2026-11-17).
Machine: Win 11 Home, Ultra 7 155H, 63.7 GiB RAM. Debug build, ephemeral
in-memory store (dead DATABASE_URL → memory fallback), concurrency 1,
port 8091. Do NOT quote as SLAs.

| Endpoint | n | p50 | p95 |
|---|---|---|---|
| `GET /healthz` | 200 | 0.45 ms | 4.61 ms |
| `GET /api/v1/version` | 20 | 0.30 ms | 0.49 ms |
| `GET /nope-404` (fallback) | 20 | 0.29 ms | 0.51 ms |
| `POST /api/v1/auth/register` | 1 | 486 ms (argon2) | — |
| `POST /api/v1/auth/login` | 1 | 587 ms (argon2 verify) | — |
| `POST /api/v1/clients` | 1 | 2 ms | — |

Prod benchmarks + prod-DB runs: BLOCKED (no prod env). Thresholds defined
in BENCHMARKS.md (healthz p95 < 500 ms @c=10; list p95 < 800 ms @c=10
seeded 1k rows) — unmeasured.

## 10. Test results

- Recorded baseline (`docs/13-development-progress/TEST-RESULTS.md`,
  2026-11-17): `cargo fmt --check` PASS; `clippy --workspace --all-targets
  -D warnings` PASS (zero warnings); `cargo test --workspace`
  **203 passed, 0 failed** (server API 65 incl. search + marketing admin +
  16 public/SEO; store 11; ai 16; ocr 16; mobile 12; auth 12; app pages).
- Live (same day, :8080 + Laragon proxy): landing + 22 public pages +
  sitemap/robots 200; Tabler + ComfyUI assets 200; pricing/faqs/articles
  seeded live. Live-DB: migrations 0001–0009 → `legalrust_drill`,
  `live_mysql_register_and_matter` PASS (1.28 s). Ignored: live-DB test in
  CI (run manually). Blocked: engine-backed OCR/AI, prod deploy.
- Static recount today: 189 `#[test]` attrs in server tests + crate
  src/tests (74 in `apps/server/tests/api.rs`, 1 ignored live-DB).
- LIVE RE-RUN for this report (2026-11-18, `SQLX_OFFLINE=true`,
  `CARGO_TARGET_DIR=target-finalverify`): `cargo test --workspace` →
  **224 passed, 0 failed, 1 ignored** (exit 0; ignored = live-DB
  `api_db`, run manually per drill). Delta vs recorded 203: +21 from
  uncommitted working-tree tests (server API suite now 80). Full log:
  per-suite `test result: ok` lines, zero FAILED.
- `cargo fmt --all -- --check` today: FAIL (exit 1, 2 files need
  rewrapping — §1). Recorded PASS (2026-11-17) predates the uncommitted
  edits; clippy not re-run (no code edits in scope for this report).

## 11. Screenshots inventory (list only — nothing moved)

`C:/Users/lindu cipta/Documents/legalrust-qa/` (12 PNGs + `check/` harness):
- `home-desktop.png` (3.2 MB), `home-mobile.png` (1.1 MB),
  `app-unauth-desktop.png`, `demo-desktop.png`, `faq-desktop.png`,
  `fitur-ai-desktop.png`, `fitur-desktop.png`, `fitur-ocr-desktop.png`,
  `harga-desktop.png`, `kontak-desktop.png`, `login-desktop.png`,
  `solusi-notaris-desktop.png`
- `check/check.js`, `check/check2.js`, `check/check3.js`,
  `check/package.json` (headless-Chromium QA scripts, not images)

`D:/` root (12 PNGs, ad-hoc captures):
- `shot_app2.png`, `shot_app_auth.png`, `shot_dash.png`,
  `shot_dash_auth.png`, `shot_fit.png`, `shot_fitur.png`, `shot_harga.png`,
  `shot_hero.png`, `shot_login.png`, `shot_mnav.png`, `shot_mob.png`,
  `shot_nav.png`

Coverage: landing desktop/mobile, nav (desktop dropdown + mobile),
hero, fitur (+OCR/AI subpages), harga, solusi-notaris, FAQ, kontak, login,
demo, app unauth + authed dashboard. No prod screenshots (no prod env).

## 12. Blockers

- BL-DB-01: CLEARED (live MySQL drill + DR restore verified; `.env`
  credential mismatch follow-up remains).
- BL-EXT-01: OCR engine / AI provider / PSrE e-sign / payment gateway /
  Dukcapil/BPN/WhatsApp-SMS-email/S3/SSO — adapters + stubs live, activation
  needs credentials. Maps to registry CONFIGURATION_REQUIRED (159).
- BL-LIC-01: LICENSE file withheld — owner licensing intent undecided
  (MIT vs proprietary).
- BL-HUMAN-01: counsel sign-off + prod deploy — needs human/infra.
- BL-GPU-01: WAN render heavy (RTX 4060 8 GB) — stills only unless asked.
- New from re-audit: REAUDIT R-01 (notify `read_at` live path unverified) —
  must clear before prod-DB acceptance.
- `docs/13-development-progress/BLOCKERS.md` adds: live benchmarks, live
  DR drill (GPG/S3 path), `.env`/Laragon password alignment.

## 13. Deploy instructions (k8s / compose / prod-runbook + keep-alive note)

- Descriptors: `deploy/k8s.yaml` (resource requests/limits 250m/256Mi →
  1 Gi; liveness `/healthz`, readiness `/readyz`; no secrets),
  `deploy/Dockerfile.server` + `deploy/Dockerfile.worker`
  (rust:1.99-bookworm builder, debian slim runtime, non-root appuser),
  `docker-compose.yml` (dev, mysql healthcheck + backup one-shot note),
  `docker-compose.prod.yml` (additive overlay; topology server + worker +
  mysql:8.4 + redis:7-alpine), `scripts/backup.sh` (mysqldump + sha256
  sidecars, 7-day retention + S3 lifecycle), `scripts/dr-drill.sh`
  (backup → restore staging → counts), `scripts/bench.sh` (curl p50/p95 +
  wrk hint), `scripts/migrate-live.sh` (env-only creds helper).
- Runbook: `docs/11-deployment/PROD-RUNBOOK.md` — `.env.prod` from
  `.env.prod.example` + vault (never commit); build/push/run; first-boot
  auto-migrate on connect (`main.rs` → `db::migrate`, forward-only; verify
  via `/readyz` `{"ready":true,"db":true}`); backups via host cron +
  `docs/16-disaster-recovery/DR-RUNBOOK.md` (RPO 24 h / RTO 4 h).
- **Server keep-alive note (dev/local):** the :8080 server is a manually
  started `cargo build` binary behind Laragon Apache proxy
  (`auto.legalrust.test` → `127.0.0.1:8080`). After ANY rebuild the old
  binary holds :8080 (cf. PID 37512 incident serving stale no-route pages)
  — kill it before starting the new binary, then re-check
  `http://legalrust.test/healthz`. JWT/INTEGRATION secrets come from `.env`;
  local DB is Laragon MySQL root/empty (NOT the `.env` password).

## 14. Next actions (ordered, no code edits by this report)

1. Credentials/config-gated: OCR engine, AI provider, PSrE, pay-gateway,
   Dukcapil/BPN → flip 501s only with per-provider `supported/configured`
   probes + startup report (RISK R-02) + counsel where required.
2. REAUDIT R-01: live-DB notify create+list+read drill (extends
   `migrate-live.sh`); keep `UNIX_TIMESTAMP` integer pattern everywhere.
3. Independent security audit (external): SAST/DAST/WAF baselines, pen-test
   tracker, hash-chained audit log, KMS hooks, `UserRow::pw_hash` Debug
   redaction, token-endpoint `authorize` comments, distributed lockout design.
4. Prod: provision env → compose/k8s deploy → prod benchmarks (protocol in
   BENCHMARKS.md) → prod DR drill (GPG/S3) → counsel sign-off (COUNSEL-
   CHECKLIST: UU ITE/PDP/notarial validity/PSrE/residency).
5. Hygiene: split `lib.rs` (~3.6k lines) by domain (RISK R-01); fix registry
   `:line` drift (cite symbols + commit hash); align `.env` ↔ Laragon creds;
   refresh user manuals for new search/admin routes (50/50 claim predates
   them); commit/pick up uncommitted test-file changes (re-audit INFO).
6. Owner decisions: LICENSE intent (BL-LIC-01); WAN video vs stills (done:
   stills); mobile/web stub labeling (done: PLANNED).

## 15. Acceptance recount (honest, denominator stays 32)

Method: each of the 32 checklist items re-checked against current code +
test/file evidence listed in this report. New work since 28/32: global
search API + /app wiring, Tabler dashboard + live KPIs, marketing admin UI
(/app mkt tab + 23 endpoints), 22 public pages + SEO, WAN hero asset
(+poster), TIMESTAMP fixes (incl. `UNIX_TIMESTAMP(read_at)`), demo login +
roles matrix, migration 0010, DTO min-12 fix, ai-crate compile fix,
Wave 7 re-audit (0 mismatch).

- PARTIAL→DONE candidates: **none qualify.** "OCR engine live" and
  "AI provider live" are still honest 501s (BL-EXT-01, credential-gated —
  no evidence can move them). "Final security audit" gained self-audit
  residual fixes + a clean re-audit, but external/independent audit,
  SAST/DAST/WAF/pentest remain NOT_STARTED/PLANNED and REAUDIT R-01 is
  open — stays PARTIAL by the same standard. MISSING (prod deploy +
  prod benchmarks + prod-DB run + counsel sign-off) unchanged.
- DONE items verified intact (spot-checked): auth/RBAC/lockout, DMS +
  shares + SHA pin, matters/tasks/deeds state machines, finance + trust
  ledger, jobs queue + worker, SSE, audit trail, CSV reports, API tokens,
  CSP/HSTS headers, portal UI, backup drill, manuals, dev benchmarks.
- **New score: 28/32 = 87/100 — NOT production-ready.** Delta: 0 moves;
  +10 bonus capabilities tracked separately (§16). Bonus do not inflate
  the score per the recount rule. Evidence freshness: workspace re-run
  2026-11-18 = 224 passed / 0 failed / 1 ignored; fmt gate currently RED
  (2 files, formatting-only, §1) — must be green before next acceptance.

## 16. Bonus list (delivered beyond the 32, tracked separately)

1. Global search API + /app grouped-results UI (lib.rs:1107/:3486, site.js:98-108).
2. Tabler ops dashboard + live KPIs (counts/trust totals from live API).
3. Marketing admin UI: leads pipeline, assign/notes/follow-ups, demo/
   contact/newsletter/pricing/FAQ/article CMS, CSV export.
4. 22 public pages + SEO (sitemap/robots/canonical/OG/JSON-LD, seeded CMS).
5. WAN hero asset (`hero-loop.mp4` + poster; stills-primary decision kept).
6. TIMESTAMP decode fixes (`DATE_FORMAT`/`UNIX_TIMESTAMP`, incl. notify).
7. Demo login button + roles/rights matrix on `/login`.
8. Migration 0010 (9 marketing tables; strengthens migrations item).
9. Self-audit residual fixes (DTO min-12, ai crate compiles).
10. Wave 7 re-audit evidence pack (registry verified accurate, 0 mismatch).

## 17. Top risks (from `RISK-REGISTER.md`, still current)

R-01 single-file `lib.rs` growth (H/M) · R-02 provider-stub drift (M/H) ·
R-03 license intent (M/M) · R-04 secret-length mismatch recurrence (M/H) ·
R-05 TIMESTAMP/string-date skew (L/M) · R-06 permissive CORS + public
state-changing endpoints (M/M) · R-07 worker executor gap (M/L) ·
R-08 audit-read fragility (L/H) · R-09 stale-progress truth (H/M —
this report + re-baselined counts address it) · R-10 unintegrated
web/mobile stubs (M/M — labeled PLANNED, honesty banners kept).

## 18. Sign-off

No production claim. No fake data. External-dependent features return 501
with docs (AGENTS.md rule 5) and are listed as BLOCKED, not IMPLEMENTED.
Next acceptance gate: R-01 live-DB drill + credentials for ≥1 provider +
independent audit scheduled + prod env provisioned. Owner decisions pending:
LICENSE intent, counsel sign-off.
