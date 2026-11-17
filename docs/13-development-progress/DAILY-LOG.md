# DAILY LOG
## 2026-11-17 (4) — legalrust.test proxy + compile fix
- DB: fixed `crates/database/src/mysql.rs:50` broken `sqlx::query(" for rn in [` -> real `for rn in [...]` loop; `ops_memory.rs` Inner added `bg_jobs/notifs/shares/audits`, job_* moved to `bg_jobs`; `memory.rs:133` `id.clone()` fix. `cargo check --workspace` green.
- Run: `cargo run -p legalrust-server` OK on `:8080` (`/healthz` ok, `/readyz` BLOCKED tanpa DB - ephemeral store aktif).
- Laragon: `D:/laragon/etc/apache2/sites-enabled/auto.legalrust.test.conf` -> `ProxyPass / http://127.0.0.1:8080/`; Apache restart manual; `http://legalrust.test/healthz` OK. Status: TESTED.
## 2026-11-17 (3) — final push Phases 7-12
- DB: migrations 0005 (OCR), 0006 (deed templates+rendered), 0007 (AI/RAG); OpsStore trait + memory + MySQL impls.
- Server: OCR jobs/review/process-501, deed templates/from-template/transition, AI chat-501+usage, RAG index/search, invoices/payments+overpay guard, webhooks HMAC, integrations list, rate-limit 20/min auth, security headers, Stores::new.
- Workflow DeedState parse/as_str; AppError::RateLimited (429).
- Tests: 8 API flows green (OCR, deeds, RAG+invoices, AI-blocked, webhooks+rate-limit). Workspace total 31 green.
- Deploy: k8s.yaml, scripts/backup.sh. Acceptance 21/31 = 68/100.
## 2026-11-17 (4) — deploy/DR/benchmark/docs workstream (docs+deploy+scripts only, no Rust edits)
- Deploy: k8s.yaml adds resource requests/limits (250m/256Mi → 1Gi) + tuned liveness /healthz + readiness /readyz probes (no secrets); docker-compose.yml adds mysql healthcheck + backup one-shot note (forward-only migrations, no destructive rollback).
- Scripts: backup.sh now mysqldump + sha256 sidecars + 7-day retention (local prune + S3 lifecycle); new dr-drill.sh (backup → restore staging → SELECT 1 + row counts → report) and bench.sh (curl latency loop p50/p95 via awk + wrk hint), bash only, no secrets.
- Docs: DR-RUNBOOK.md (RPO 24h/RTO 4h, roles, restore steps, BLOCKED until first live drill), BENCHMARKS.md (honest: no measured numbers yet; matrix + c=1/10/50, p95 <500ms healthz / <800ms list), COUNSEL-CHECKLIST.md (UU ITE, PDP, notarial validity, PSrE e-sign, residency — all unchecked, counsel sign-off pending).

## 2026-11-17 (4) — final round
- Tokens API (create/list/revoke, Bearer dual-auth), password change (token-auth forbidden), role assignment (Assign-gated), notifications inbox+read with creation hooks, expiring document shares + public download, CSV report, role-gated audit viewer, durable jobs API + worker claim loop, SSE /events, trust ledger API (accounts/entries/transfer, no-negative guard), CSP/HSTS headers.
- Vocabulary unified (debit/credit) with parallel workstream; their stub/lockout/CSV tests kept green.
- Migration 0009 (trust). Isolated build dir (target/ws2) to avoid clobbering parallel dev-server runs.
- 89 tests green. Acceptance 24/32 = 75/100.

## 2026-11-17 (5) — six-agent wave closure (main-agent verification)
- Trust ledger, OCR/RAG/AI helpers, hardening (LoginGuard/skew-check/CSV builders), portal UI + design system, server hardening (lockout/CSP/HSTS/esign-501/billing-501/CSV endpoint/ocr-supported), deploy/DR/docs — all 6 workstreams complete.
- Deduplicated migrations (removed stale `0009_trust_ledger.sql`, kept code-matching `0009_trust.sql`; no live DB ever ran, BL-DB-01 unchanged) + removed merged scratch files (`api_new_tests.rs.txt`, `trust_body.rs.txt`). Sequence now 0001–0009.
- Independently re-verified: `cargo fmt --check` PASS, `cargo check --workspace` PASS, `cargo clippy --workspace --all-targets -- -D warnings` PASS, `cargo test --workspace` **89 passed, 0 failed** (1 ignored: live-DB). Full suite ran under `target-test/` (main `target/debug` exe lock held by foreign PID, not ours — left alone).
- Portal UI moves PARTIAL → DONE (12 tests). Acceptance 25/32 = 78/100, still NOT production-ready.

## 2026-11-17 (6) — live-DB migration drill (BL-DB-01) — LIVE-RUN OK
- Detection: port 3306 TCP open; `docker` not installed (no compose MySQL); Laragon MySQL 8.4.9 alive (`mysqladmin ping` OK). `.env` DATABASE_URL password rejected (`Access denied for user 'root'@'localhost' (using password: YES)`); drill used Laragon default auth (root, empty password via `MYSQL_PWD`, never echoed). `.env` creds do NOT match the local MySQL server — follow-up: align `.env`/docs or Laragon password before any shared live run.
- DB: `CREATE DATABASE IF NOT EXISTS legalrust_drill` (utf8mb4/utf8mb4_unicode_ci) on 127.0.0.1:3306.
- Run (isolated `$env:CARGO_TARGET_DIR='target-dblive'`): `cargo test -p legalrust-server --test api_db -- --ignored --nocapture` with `DATABASE_URL=mysql://root@127.0.0.1:3306/legalrust_drill` → `test live_mysql_register_and_matter ... ok` (1 passed, 0 failed; build ~53s, test 1.28s). No server started (no port needed).
- Evidence: `_sqlx_migrations` versions 1–9 all applied; 31 user tables present; row counts — organizations=1, users=1, roles=12, permissions=12, user_roles=1, sessions=1 (one live register flow).
- Added: `scripts/migrate-live.sh` (env-only creds helper: ping → create DB → ignored live test → row-count sanity; no hardcoded secrets). No edits to migrations/crates/apps/docker-compose.

## 2026-11-17 (7) — live DR restore drill (local) — OK
- `mysqldump --routines --events legalrust_drill` (DUMP_EXIT 0) → restored into fresh `legalrust_staging` (RESTORE_EXIT 0).
- Verified: organizations 1=1, users 1=1, staging tables 32 (31 + `_sqlx_migrations`). Dump file deleted post-drill (contained test password hash).
- Backup-restore DRILL moves PARTIAL → DONE (local evidence; production GPG/S3 path in `dr-drill.sh` still awaits prod run).

## 2026-11-17 (8) — wave closure: benchmarks + full re-verification
- Bench agent: /healthz p50 0.45ms p95 4.61ms (n=200), version p50 0.30ms, 404 p50 0.29ms, register 486ms/login 587ms (argon2); table in `docs/30-BENCHMARK-PROTOCOLS/BENCHMARKS.md`; server killed, port 8091 free.
- Final workspace verification (main): fmt PASS, clippy `-D warnings` PASS, `cargo test --workspace` **118 passed, 0 failed** (+29: mobile 12, integrations 14, ocr 3).
- Acceptance 28/32 = 87/100. Remaining: OCR/AI providers live, final security audit, production deployment + counsel sign-off.

## 2026-11-17 (12) — wave 5: public website + marketing CMS + QA visual (main)
- 22 halaman publik (fitur x7, solusi x5, harga server-rendered, tentang, kontak/demo form urlencoded, artikel + slug, FAQ accordion, privasi, syarat, sitemap.xml, robots.txt) + meta/OG/canonical/JSON-LD + mega menu CSS-only + mobile details-menu. Status jujur per fitur (TERSEDIA/TERBATAS/BUTUH KONFIGURASI).
- CMS terisi live: 4 paket harga (1 aktif), 8 FAQ, 2 artikel — semua endpoint publik 200.
- 6 gambar seksi ComfyUI (approved) + manifest; WAN T2V terkonfirmasi ada tapi tidak dipakai (keputusan: foto saja).
- TIMESTAMP decode panic diperbaiki (DATE_FORMAT/UNIX_TIMESTAMP); /orgs/me 200 kembali.
- QA visual headless Chromium: landing + fitur + harga terkonfirmasi render benar.
- fmt/clippy/test: **187 passed, 0 failed**. Pushed `5c42dc5`.

## 2026-11-17 (12) — wave 4: marketing backend + Tabler + WAN (main)
- Migration 0010 marketing (9 tabel) + MarketingStore (memory+MySQL) + lead status state-machine; 23 endpoint public/admin (lead pipeline, notes, assign, follow-up, pricing/faq/article CMS) — RBAC Approve + audit.
- Landing/login/dasbor dirombak ke Tabler self-host (CSS+JS+49 ikon woff2 lokal, font PlusJakarta lokal, nol CDN). Foto hero/CTA + 4 aset fitur dari ComfyUI RealVisXL (reviewed, approved). WAN 2.1 T2V 1.3B terkonfimasi ada + render 9 frame jalan (user pilih: foto saja — render video dibatalkan, stills diekstrak dari frame bila perlu).
- Workspace: fmt PASS, clippy -D warnings PASS, **171 passed, 0 failed**. Binary :8080 di-swap; legalrust.test 200 semua (landing, vendor, hero, pricing, faqs).

## 2026-11-17 (10) — swap binary :8080, legalrust.test live (main)
- Penyebab NOT_FOUND: PID 37512 (binary lama, tanpa route halaman) masih memegang :8080. Dihentikan, `cargo build`, start binary baru dengan JWT_SECRET/INTEGRATION_SECRET dari `.env` + DATABASE_URL root-tanpa-password ke DB `legalrust` (baru dibuat).
- Hasil: `/` 200 landing, `/healthz` ok, `/readyz` db:true (migrasi 0001–0009 jalan), register+login 200, `http://legalrust.test/` 200 HTML landing via proxy Laragon.

## 2026-11-17 (9) — public pages: landing + login + dasbor (main)
- `GET /` (landing + menu Masuk), `GET /login` (form login + daftar org, fetch JSON ke API, token → localStorage → `/app`), `GET /app` (dasbor: hitung klien/perkara/dokumen/akta + daftar perkara, redirect ke /login bila 401/tanpa token). JS seperlunya (fetch + render), logika bisnis tetap di server.
- Live check di :8093: `/`, `/login`, `/app` semua 200 + HTML benar. `legalrust.test` (proxy Laragon → :8080) otomatis ikut tampil setelah server :8080 di-restart dengan binary baru.
- Tes `public_pages_serve_landing_login_app` hijau. Workspace kini **119 passed, 0 failed**.

## 2026-11-17 (11) — redesign total UI publik (main, keluhan user valid)- Gaya editorial-luxury: paper hangat + serif Fraunces + Plus Jakarta Sans, floating island nav, hero masif, strip statistik jujur (32/119/9/100%), bento asimetris 6 fitur sesuai kemampuan nyata (DMS, akta, OCR, keuangan, trust ledger, AI/RAG/audit), reveal-on-scroll + hover spring physics.
- Login: kartu double-bezel, tab Masuk/Daftar organisasi, pesan error + loading state. Dasbor: 6 kartu stat (termasuk total saldo trust Rp) + tabel perkara ber-badge status.
- Binary :8080 di-swap; `legalrust.test/` + `/login` + `/app` 200 terverifikasi live.
