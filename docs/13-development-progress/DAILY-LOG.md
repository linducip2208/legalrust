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
