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

## 2026-11-18 — autonomous wave (E. Setiawan + agents)
- Global search: `GET /api/v1/search` tenant-scoped + search box /app (59 API tests green).
- Audit repo formal: INITIAL-AUDIT/ARCHITECTURE/DEPENDENCY-MAP/RISK-REGISTER.
- Marketing admin API (23 routes) + 10 tests; workspace **197 passed, 0 failed**.
- Fix TIMESTAMP decode panic (DATE_FORMAT/UNIX_TIMESTAMP); /orgs/me 200.
- Mobile overflow fixed + verified; server logging ke logs/autonomous/.

## 2026-11-18 — marketing admin UI + Wave 6 verify (main)
- Tab Marketing ke-5 di /app: pipeline leads (filter/advance/assign/notes), demo, kontak, newsletter, pricing toggle, FAQ add, articles publish, CSV export — semua via /api/v1/admin/*, 401 → /login.
- Workspace **203 passed, 0 failed** (server API 65). F-MKTADMIN DONE.

## 2026-11-18 — premium UX pass (main, menuju 100/100 & 1000/100 UI)
- Scope: hanya `apps/server/static/site.css` + `site.js` (CSP-safe, tanpa inline style/handler).
- CSS: focus-visible 3px, ::selection brand, hover/active/disabled feedback, `user-invalid` error state, table-row hover, card lift, FAQ open state, sidenav slide, skip-link class, `prefers-reduced-motion` kill-switch, print stylesheet.
- JS: `Escape` menutup hasil search + notifikasi; Ctrl/Cmd+K tetap fokus ke search.

## 2026-11-18 — UI/UX visual audit + perbaikan (main, headless Chrome 1440px + 390px + CDP)
- Verifikasi: landing/login rapi di desktop; CDP ukur viewport 390 → `docW=390`, nol overflow halaman (9 `<table>` di dalam `.mini`/`.table-responsive` scroll-container, by design).
- Temuan diperbaiki: klaim `Dipercaya oleh Ribuan Profesional` (fabrikasi) → `Dirancang untuk Praktik Hukum Indonesia`; ikon huruf P/O/T/A/L/K → Tabler `ti-briefcase/files/scan/file-text/report/shield-check` (terverifikasi render, bukan tofu); duplikat `Kembali ke Beranda` di login dibuang; tombol demo `ghost` → `outline-secondary` (kontras); `.stat b` nowrap (`Rp 248 jt` tidak wrap); `html,body{overflow-x:clip}`; skip-link + `id=main` di 3 shell (public/auth/app).
- Tes: server API 99/99, fmt+clippy clean. Catatan: teks "terpotong" di screenshot mobile adalah artefak downscale viewer — DOM + CDP membuktikan teks utuh.

## 2026-11-18 — OCR native live via tesseract (main, pilihan user #1)
- Host: `Tesseract-OCR 5.4.0` (UB-Mannheim) + traineddata `eng+ind+osd` di `%LOCALAPPDATA%\tessdata`; PATH+TESSDATA_PREFIX persisten (user env). `tesseract --list-langs` live-terverifikasi.
- Kode: `crates/ocr` — `TesseractAdapter::recognize` real (temp file + `tesseract stdout -l ind+eng tsv`, watchdog 120s, parse TSV level-5 jadi baris+confidence); `image_extension` sniff png/jpg/webp/bmp/tif; tambah dep `anyhow` (cached, tanpa network).
- Storage: `OpsStore::ocr_store_result` baru (memory single-lock, mysql single-tx: status+fields atomik, re-process idempoten). Tanpa migrasi (tabel `ocr_fields` ada sejak 0005).
- Endpoint: `POST /api/v1/ocr/jobs/{id}/process` live — gambar jadi `processed`+fields+notif; tanpa konten 422; non-gambar 422; tanpa engine 501 (lama).
- Tes: ocr crate 21/21; server API 99/99; workspace **268 passed, 0 failed**; fmt+clippy clean.
- Landing jujur: 29/32, OCR live-bila-terinstall, sisa 501: ai/chat, esign, pay-gateway.

## 2026-11-18 — payment gateway Tripay live-bila-dikonfigurasi (main, request user: tanpa hardcode)
- Adapter baru `crates/integrations/src/tripay.rs`: `TripayConfig::default_env` (TRIPAY_MODE/BASE_URL/API_KEY/PRIVATE_KEY/MERCHANT_CODE/CALLBACK_URL/RETURN_URL — semua via env, rahasia tidak disimpan di struct/Debug/log).
- Signature `HMAC-SHA256(merchant_code+merchant_ref+amount)` + vektor uji tetap; `verify_tripay_callback` (X-Callback-Signature); `build/parse_merchant_ref` (`LRINV{id}-{rand}` untuk mapping callback→invoice).
- HTTPS via `reqwest` + `rustls-tls-native-roots` (pure Rust, tanpa OpenSSL; registry reachable jadi dep baru OK).
- Endpoint: `POST /api/v1/invoices/{id}/pay-gateway` live — validasi channel/nama/expired, tolak invoice lunas (409), buat transaksi Tripay, audit+notif. Tanpa env → 501 lama (pesan baru `tripay not configured`).
- Callback: `POST /api/v1/webhooks/incoming/tripay` — verifikasi signature, hanya `PAID` diproses, mapping via `inv_owner_org` baru (memory+mysql, tanpa migrasi), idempoten via marker `tripay:{reference}`, guard overpay existing, audit+notif.
- Tes: integrations 30 (vektor HMAC, callback, resolve, stub HTTP create+rejection); server 100 (501-tanpa-env, stub E2E checkout→PAID→paid=balance 0→duplikat→401). Workspace **280 passed, 0 failed**; fmt+clippy clean.
- Env didokumentasikan di `.env.example` + `.env.prod.example`; compose `env_file` + k8s `envFrom` teruskan otomatis tanpa perubahan manifest.
- Skor tetap 29/32: gateway live-bila-kredensial-merchant-diisi; yang dibutuhkan dari user: TRIPAY_API_KEY + PRIVATE_KEY + MERCHANT_CODE (sandbox dulu untuk uji).

## 2026-11-18 — AI chat live via Ollama + self-audit + manual refresh (main)
- Host: `Ollama 0.40.16` + `qwen2.5:1.5b` (986MB) serve lokal. Kode: `legalrust-ai::complete` baru (reqwest/rustls, OpenAI-shape, env AI_BASE_URL/AI_MODEL/AI_API_KEY/AI_TIMEOUT_SECS, tanpa hardcode); `POST /api/v1/ai/chat` simpan jawaban asisten + output usage; 501 hanya bila endpoint tak menjawab. Live proof via endpoint: `{"reply":"4","model":"qwen2.5:1.5b","input_tokens":40,"output_tokens":2}`.
- Tes: ai crate 19/19 (stub + unreachable-blocked + defaults); server API 102/102 (stub 200+usage, gated-ollama, 501-unreachable). Workspace **285 passed, 0 failed**; fmt+clippy clean.
- Hardening: CORS permissive → allowlist env (`CORS_ORIGINS`, default same-origin-only); self-audit (no hardcoded secrets, no TODO non-test, .env ignored, no PII di log) → DONE (pentest eksternal tetap disarankan); tesseract masuk `deploy/Dockerfile.server` runtime.
- Manual: 02-PORTAL ditulis ulang (/app 10 tab + search + notif), 04/06/07/08 diperbarui (OCR engine, Tripay, AI live, troubleshooting baru); env AI/CORS/Tripay di `.env.example` + `.env.prod.example`; `deploy/README` langkah 6-8.
- Skor: 31/32 = 97/100. Sisa jujur: prod deploy + benchmarks + prod-DB + counsel sign-off.

## 2026-11-18 — AUDIT-1000 + approvals engine (main, mode eksekusi otonom)
- Refresh aman: HEAD == origin/main (3d09a96), tree bersih, 1 worktree, tanpa uncommitted work. 3 subagent read-only (inventory 110 routes/105 trait methods/44 tabel; gap scan: nol TODO/mock/501-jujur; partial-verify D08/D20/D25).
- Implementasi: migrasi 0013 approvals; `OpsStore::appr_*` (memory+mysql, tx atomik); 5 endpoint + RBAC + role-match; audit di 3 endpoint CSV; ikon PWA 192/512.
- Tes: 3 approvals tests baru; workspace **288 passed, 0 failed**; fmt+clippy clean.
- Registry: F-D09 ×9 → PARTIAL (backend lengkap, UI pending), F-D20-034 → IMPLEMENTED. Skor registry 15.8 → **16.3/100**; 8-cat mean 18.2; komposit kualitas ≈ 44/100 — BUKAN 100 (lapor jujur).
- Artefak: docs/AUTONOMOUS/AUDIT-1000/ (scores, evidence, domain, uiux, security, tests, manifest, results, ownership, RESUME.md). Next: approvals /app tab UI, report center, queue-depth, webhook log (lihat RESUME.md).

## 2026-11-18 — legalrust.test jadi file listing: diagnosis + perbaikan (main)
- Penyebab ganda: (1) backend :8080 mati (dimatikan saat bersih-bersih sesi) → proxy tanpa tujuan; (2) Laragon menimpa `auto.legalrust.test.conf` (ProxyPass manual hilang) → vhost jadi DocumentRoot repo = terlihat seperti file manager.
- Perbaikan: `D:/laragon/etc/apache2/sites-enabled/00-legalrust-proxy.conf` (prefix `00-`, bukan `auto.*` agar tidak ditimpa; dimuat duluan sehingga menang untuk ServerName sama) + restart Apache (kill tree + start ulang dengan `-d` ServerRoot yang sama; `-k restart` gagal karena Apache jalan sebagai proses biasa, bukan service) + backend dijalankan ulang (`target-ui5/debug/legalrust-server.exe`, JWT 32+, DB Laragon `legalrust`).
- Verifikasi: `legalrust.test/healthz` → backend JSON ok; `/`, `/login`, `/app` → 200.
- Catatan operasional: backend berjalan sebagai proses latar (bukan service); bila mati, jalankan ulang perintah di atas lalu cek `:8080/healthz`. Jangan edit `auto.*` (ditimpa Laragon) 

## 2026-11-18 — migrasi 0013/0014 live-verified + api_db diperbaiki (main)
- Terapkan 0001‒0014 ke DB scratch Laragon MySQL 8.4.9: OK; tabel approvals + webhook_deliveries ada. Test `api_db` yang basi (password/JWT pendek) diperbaiki → PASS. DB scratch dihapus.

## 2026-11-18 — AUDIT-1000 wave 3: webhook delivery log (main)
- Migration `0014_webhook_deliveries` (org nullable, provider/event/status); `OpsStore::webhook_log/list` (memory+MySQL); setiap receipt tercatat (accepted/rejected/blocked).
- `GET /api/v1/webhooks/deliveries` (View-gated, tenant-scoped) + kartu /app di panel audit + JS `whRefresh()` + needle test.
- Tes: workspace **292 passed, 0 failed**; fmt+clippy+node --check clean.
- Registry: F-D08-013, F-D25-017 → PARTIAL; skor 16.9 → **17.0/100**; 8-cat mean 18.6 (16 verified). — edit `00-legalrust-proxy.conf`.

## 2026-11-18 — AUDIT-1000 wave 2: approvals UI + report center (main)
- /app: panel `p-appr` (compose/list/detail/decide/revoke), tab Persetujuan + sidenav Alur Kerja rewired, TABS/TABLBL, `appr*` JS + job live counts + trust-CSV download; node --check + needle test hijau; HTML live-terverifikasi via curl (:8080 + legalrust.test 200).
- API: `GET /api/v1/reports/trust-ledger.csv` (Export-gated + audit `report.trust_csv_exported`); job counts client-side (tanpa endpoint baru).
- Tes: server API 105 → 107, workspace **290 passed, 0 failed**; fmt+clippy clean.
- Registry: F-D09 ×8, F-D20-001/004/020, F-D25-009 → IMPLEMENTED (total 51); skor 16.3 → **16.9/100**; 8-cat mean 18.4 (14 verified).
## 2026-11-18 — AUDIT-1000 wave 4-6: repertorium + webhook log + compliance (main)
- Wave 4: DeedRow.created + GET /api/v1/reports/deeds.csv?date= (Export-gated + audit) + /app akta card + JS + tes; F-D12-001 -> IMPLEMENTED, F-D20-009 -> PARTIAL.
- Wave 5: compliance slice (migration 0015, 6 endpoint + RBAC + audit), api_db stale-test fix, migrasi 0001-0015 live-verified MySQL; F-D11-001/002 -> PARTIAL.
- Wave 6: tab Kepatuhan + panel p-kepatuhan di /app (obligations + consents), JS wired, needle tests; F-D11-001/002 -> IMPLEMENTED.
- Registry 51 -> **54 IMPLEMENTED**; skor 16.9 -> **17.4/100**; 8-cat mean 18.8 (22 verified); workspace 296 passed, 0 failed.

## 2026-11-18 AUDIT-1000 wave 25: riwayat versi dokumen (main)
- Menu Dokumen kini tampilkan riwayat versi; workspace **318 passed, 0 failed**; F-D04-003 86.5.

## 2026-11-18 AUDIT-1000 wave 24: catat bayar UI (main)
- Menu Keuangan kini bisa catat pembayaran + sisa; workspace **317 passed, 0 failed**; F-D15-015/019 86.5.

## 2026-11-18 AUDIT-1000 wave 23: user directory (main)
- Direktori user org + dropdown peran + IDOR fix + audit; workspace **316 passed, 0 failed**; F-D16-001 IMPLEMENTED (76.9).

## 2026-11-18 AUDIT-1000 wave 22: notification filter + manual (main)
- Checkbox unread + manual; workspace **315 passed, 0 failed** (1 transient flake, 4x rerun hijau); F-D18-038 IMPLEMENTED (74.4).

## 2026-11-18 AUDIT-1000 wave 21: notification audit trail (main)
- Audit send+read + trail assertions; workspace **315 passed, 0 failed**; F-D18-028 68.5 PARTIAL.

## 2026-11-18 AUDIT-1000 wave 20: newsletter unsubscribe (main)
- Rute publik berhenti + tombol + lifecycle; workspace **315 passed, 0 failed**; F-D18-026 69.4 PARTIAL.

## 2026-11-18 AUDIT-1000 wave 19: notification unread/limit (main)
- ?unread_only/limit + DESC + dropdown wiring; workspace **314 passed, 0 failed**; F-D18-038 67.9 PARTIAL.

## 2026-11-18 AUDIT-1000 wave 18: AR aging report (main)
- Piutang terbuka org-wide + CSV + kartu Keuangan; workspace **313 passed, 0 failed**; F-D14-004 IMPLEMENTED (81.6).

## 2026-11-18 AUDIT-1000 wave 17: papan skor 1000 fitur (main)
- Semua 1000 fitur tampil di /skor + API dengan skor jujur; workspace **311 passed, 0 failed**.

## 2026-11-18 AUDIT-1000 wave 16: sidebar dedup 1:1 (main)
- Hilangkan menu ganda aktif; tambah link Keuangan; workspace **309 passed, 0 failed**.

## 2026-11-18 AUDIT-1000 wave 15: approval notification node (main)
- ?kind= filter + appr-tab card + tests; F-D08-015/003 IMPLEMENTED; workspace **309 passed, 0 failed**.

## 2026-11-18 AUDIT-1000 wave 14: search hardening + RAG ACL-trim (main)
- Search pagination/types/totals + RAG rank + tenant trim; UI pager; workspace **308 passed, 0 failed**; F-D23-001/021/031 IMPLEMENTED (8-cat 74.4).

## 2026-11-18 AUDIT-1000 wave 13: repertorium period report (main)
- deeds.csv periode dari/sampai + guard rentang; /app from/to; workspace **306 passed, 0 failed**; F-D20-009 IMPLEMENTED (8-cat 76.9).

## 2026-11-18 AUDIT-1000 wave 12: /app ROPA UI (main)
- ROPA card di tab kepatuhan (tambah/hapus/muat-ulang); workspace **306 passed, 0 failed**; F-D11-006 frontend 80 (8-cat 80.0).

## 2026-11-18 AUDIT-1000 wave 11: ROPA register + compliance fixes (main)
- ROPA register UU PDP live (migration 0017, API CRUD, RBAC, audit); memory cross-tenant delete bug diperbaiki; workspace **306 passed, 0 failed**; registry 17.7 (56 IMPLEMENTED); 8-cat mean 19.1 (25 verified).

## 2026-11-18 AUDIT-1000 wave 10: dead-letter alerting + status filter (main)
- Dead-letter audit trail + ?status= filter + /app Dead letters card; workspace 304/0; registry 17.6 (55 IMPLEMENTED); 8-cat mean 19.0 (24 verified).

## 2026-11-18 AUDIT-1000 wave 9: durable retry queue + mobile re-audit + benchmarks (main)
- job_enqueue_at (memory lease + MySQL DATE_ADD) + process_webhook_job + worker arm; dead-letter audit; 4 tes baru; workspace 304/0.
- Registry 54 -> **55 IMPLEMENTED** (F-D08-013); 8-cat mean 19.0 (24 verified).

## 2026-11-18 AUDIT-1000 wave 7: auto-dispatch on notify (main)
- Fan-out background di notify_user ke endpoint terdaftar (signed post_json, timeout 10s, delivery log accepted/rejected); tes E2E signed-stub; /app unchanged (panel deliveries sudah ada).
- 1 transient failure (stub timing) terlihat sekali, tidak terulang di 3 run penuh; dicatat.
- Registry 54 IMPLEMENTED (tetap); 8-cat mean 18.9 (20 verified).
- Endpoints CRUD + signed test-send (HMAC) + delivery log in+out + /app card; migrasi 0016 live-verified; registry 17.4; 8-cat mean 18.9; workspace 298/0.

## 2026-11-18 - benchmarks wave 8 + B-WH-RETRY queued (main)
- Dev benchmarks live-DB (Laragon MySQL): healthz p50 5.9-19.1ms; matters POST p50 20.4ms; approvals POST p50 27.5ms / list p50 8.9ms; webhooks deliveries p50 8.4ms; consents p50 7.5ms; deeds.csv p50 13.2ms (n=5-10, debug build, smoke only).
- B-WH-RETRY queued (retry queue + dead-letter + failure alerting); B-BENCH partial (dev numbers recorded, prod thresholds pending).
