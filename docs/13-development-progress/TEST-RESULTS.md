# TEST RESULTS
## 2026-11-17 — full workspace (SQLX_OFFLINE=true, isolated target dir, no live DB)
- `cargo fmt --all -- --check`: PASS. - `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **262 passed, 0 failed** (server API 98 incl. ops UI + CRM + search + marketing; store 17 incl. deadlines; AI 16; OCR 16; mobile 12; auth 12; app pages; full crate matrix).
- Live (same day, :8080 + Laragon proxy): landing + 22 public pages + sitemap/robots 200; Tabler + ComfyUI assets 200; pricing/faqs/articles seeded live.
- Live (same day, :8080 + Laragon proxy): Tabler landing + vendor CSS + ComfyUI hero/CTA PNGs 200; /public/pricing + /public/faqs 200; H1 verified in served HTML.
- Live-DB (same day, Laragon MySQL 8.4.9): migrations 0001–0009 applied to `legalrust_drill`; `live_mysql_register_and_matter` PASS (1.28s); row counts organizations=1 users=1 roles=12 permissions=12.
- DR restore drill (same day, local): mysqldump→`legalrust_staging` restore OK, counts match, dump deleted post-drill.
- Benchmarks (same day, dev machine i.e. Ultra 7 155H, debug, ephemeral store, n=200/20, concurrency 1): /healthz p50 0.45ms p95 4.61ms; /version p50 0.30ms; 404 p50 0.29ms; register 486ms / login 587ms (argon2); full table in `docs/30-BENCHMARK-PROTOCOLS/BENCHMARKS.md`.
- Ignored (CI live-DB): `api_db::live_mysql_register_and_matter` (run manually instead — PASS, see above). Blocked: engine-backed OCR/AI, production deploy.
## 2026-11-18 — OCR native live (SQLX_OFFLINE=true, tesseract 5.4.0 + ind/eng/osd on PATH, TESSDATA_PREFIX=%LOCALAPPDATA%\tessdata)
- `cargo fmt --all -- --check`: PASS. `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **268 passed, 0 failed** (server API 99 incl. `ocr_process_image_live_or_blocked` + `ocr_process_requires_content`; OCR crate 21 incl. TSV parser, magic sniff, blank-PNG live no-fabrication; full crate matrix).
- Live engine proof: `tesseract hero-office.png stdout -l ind+eng tsv` runs on host (TSV header + word rows parsed; photo yields 0 text lines — honestly empty, nothing fabricated).
- Host setup (per-host requirement, persisted to user env): winget `UB-Mannheim.TesseractOCR` + `ind.traineddata` (tessdata_fast) + `configs`/`tessconfigs` copied beside traineddata + PATH/TESSDATA_PREFIX.
- Blocked (tersisa jujur): PSrE e-sign (tanpa provider), production deploy.
## 2026-11-18 — AI live + hardening + manual (SQLX_OFFLINE=true, Ollama lokal qwen2.5:1.5b)
- `cargo fmt --all -- --check`: PASS. `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **285 passed, 0 failed** (ai 19 incl. live-stub + unreachable-blocked; server API 102 incl. stub-200, gated-ollama-live, 501-unreachable; OCR 21; Tripay stub E2E).
- Live proof: `POST /api/v1/ai/chat` → `{"reply":"4","model":"qwen2.5:1.5b","input_tokens":40,"output_tokens":2}` (Ollama lokal via endpoint).
- Self-audit: grep secrets/TODO/unsafe/PII-log + CORS permissive→allowlist + .env ignored — temuan ditutup kecuali pentest eksternal (rekomendasi).
## 2026-11-18 — AUDIT-1000 wave 2: approvals UI + report center (SQLX_OFFLINE=true, node --check clean)
- Gates: fmt PASS, clippy `-D warnings` PASS.
- `cargo test --workspace`: **290 passed, 0 failed** (server API 107 incl. approvals UI needles + trust CSV; OCR 21; AI 19; Tripay stub E2E).
- Registry: IMPLEMENTED 38 → **51** (F-D09 ×8, F-D20-001/004/020, F-D25-009); skor 16.3 → **16.9/100**; 8-cat mean 18.4 (14 verified).
## 2026-11-18 — AUDIT-1000 wave 1: backend slice + gap scan (SQLX_OFFLINE=true; 3 subagent read-only + eksekutor)
- Gates: fmt PASS, clippy `-D warnings` PASS.
- `cargo test --workspace`: **288 passed, 0 failed** (approvals 3 baru; sisanya hijau).
- Gap scan: nol TODO/mock di non-test source; semua 501 jujur dan terdokumentasi.
- Registry: 10 fitur update (F-D09 ×9 → PARTIAL backend-lengkap, F-D20-034 → IMPLEMENTED); skor 15.8 → 16.3/100; artefak AUDIT-1000 + RESUME.md lengkap.
## 2026-11-18 — Tripay gateway wired (SQLX_OFFLINE=true, stub HTTP lokal, tanpa kredensial asli)
- `cargo fmt --all -- --check`: PASS. `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **280 passed, 0 failed** (integrations 30 incl. HMAC vector + callback + stub create/reject; server API 100 incl. 501-tanpa-env + stub E2E checkout→PAID→duplikat→401; OCR 21; full matrix).
- Kredensial: TIDAK ada hardcode — semua via `TRIPAY_*` env; rahasia tidak muncul di Debug/log/audit. Live sandbox menunggu kredensial merchant dari user.
