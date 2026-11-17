#!/usr/bin/env python3
"""Final docs: openapi additions, acceptance recount, progress, coordination note."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")
def app(rel, text):
    p = R/rel; t = p.read_text(encoding="utf-8")
    p.write_text(t.rstrip()+"\n\n"+text.strip()+"\n", encoding="utf-8", newline="\n")

p = R/"docs/04-api-specification/openapi.yaml"; t = p.read_text(encoding="utf-8")
t += """  /api/v1/auth/tokens: { post: { responses: { '200': { description: plaintext shown once } } }, get: { responses: { '200': { description: list without secrets } } } }
  /api/v1/auth/tokens/{id}: { delete: { responses: { '200': { description: revoked } } } }
  /api/v1/auth/password: { post: { responses: { '200': { description: changed } } } }
  /api/v1/users/{uid}/roles: { post: { responses: { '200': { description: assigned } } } }
  /api/v1/notifications: { get: { responses: { '200': { description: own inbox } } } }
  /api/v1/documents/{id}/shares: { post: { responses: { '200': { description: share token+url } } }, get: { responses: { '200': { description: list } } } }
  /api/v1/shares/{id}: { delete: { responses: { '200': { description: revoked } } } }
  /api/v1/shared/{token}: { get: { responses: { '200': { description: public metadata, expiry-checked } } } }
  /api/v1/reports/matters.csv: { get: { responses: { '200': { description: csv export } } } }
  /api/v1/audit: { get: { responses: { '200': { description: auditor/owner role-gated log } } } }
  /api/v1/jobs: { post: { responses: { '200': { description: enqueued } } }, get: { responses: { '200': { description: list } } } }
  /api/v1/trust/accounts: { post: { responses: { '200': { description: escrow account } } }, get: { responses: { '200': { description: list with balances } } } }
  /api/v1/trust/accounts/{id}: { get: { responses: { '200': { description: account+entries } } } }
  /api/v1/trust/accounts/{id}/entries: { post: { responses: { '200': { description: posted }, '409': { description: insufficient balance } } } }
  /api/v1/trust/transfer: { post: { responses: { '200': { description: double-entry legs } } } }
  /api/v1/events: { get: { responses: { '200': { description: text/event-stream, opens with hello } } } }
"""
w("docs/04-api-specification/openapi.yaml", t)

w("docs/13-development-progress/ACCEPTANCE.md", """# FINAL ACCEPTANCE (Phase 15 re-audit — evidence, no inflation)
Score = DONE+TESTED / 32 checklist items (+ bonus capabilities tracked separately).
## DONE + tested (24)
PRD, architecture docs, schema, migrations 0001-0009, auth (JWT+API tokens+sessions+revocation), org mgmt (register/roles/tokens/password), RBAC enforcement, clients, identity-verify pipeline (OCR job+review), document engine (DMS+versions+shares+sha-pinning), projects/matters, tasks (guarded transitions), notarial workflow (state machine), deeds (templates/render/transitions), AI accounting+RAG keyword baseline, notifications (in-app inbox), finance (invoices/payments/overpay guard + trust ledger double-entry), jobs queue (enqueue/claim/complete), SSE events, API docs (openapi.yaml, 60+ routes), audit trail (writer+role-gated viewer), security controls (headers/rate-limit/RBAC/Argon2/JWT), reporting (CSV), automated testing (89 green), CI/CD, deployment docs.
## PARTIAL (6)
OCR engine live, AI provider live, backup restore DRILL (script exists), full frontend (SSR shell only), user manuals (ship with UI), final security audit (self-audit only).
## MISSING (2)
Performance audit with baselines, production deployment + live-DB migration run.
## Score: 24/32 = 75/100 — NOT production-ready. Bonus delivered beyond checklist: expiring share links, token auth, CSV reports, audit viewer API, durable jobs + worker, SSE, trust ledger, login rate-limit, CSP/HSTS headers.
Blockers unchanged: B-001 counsel, B-002/B-003 measurements, live DB run, DR drill, LICENSE decision.
""")
w("docs/13-development-progress/TEST-RESULTS.md", """# TEST RESULTS
## 2026-11-17 — full workspace (SQLX_OFFLINE=true, isolated target dir, no live DB)
- `cargo fmt --all -- --check`: PASS. - `cargo clippy --workspace --all-targets -- -D warnings`: PASS (zero warnings).
- `cargo test --workspace`: **89 passed, 0 failed** (server API incl. auth dual-mode, tokens, password, roles, notifications, shares+public download, reports CSV, audit viewer, jobs, trust double-entry + overdraft guard, SSE hello, rate-limit 429, webhook HMAC, OCR/deed/RAG/invoice flows; crate unit tests incl. trust double-entry atomicity).
- Ignored (CI live-DB): `api_db::live_mysql_register_and_matter`. Blocked: live migration run, engine-backed OCR/AI, DR drill.
""")
w("docs/13-development-progress/MASTER-PROGRESS.md", """# MASTER PROGRESS (computed from repo, not invented)
- Phases 0-3: IMPLEMENTED + TESTED. - Phase 4 auth: IMPLEMENTED + TESTED (JWT+sessions+API tokens+RBAC+roles+password+rate-limit; live-DB via CI job).
- Phase 5: PLANNED (SSR shell only). - Phase 6: IMPLEMENTED + TESTED (clients/matters/tasks/documents+content/shares).
- Phase 7 OCR: PARTIAL (jobs/review HTTP tested; engine BLOCKED). - Phase 8 deeds: IMPLEMENTED + TESTED.
- Phase 9 AI: PARTIAL (accounting+RAG keyword tested; provider+BLOCKED vectors). - Phase 10 finance: IMPLEMENTED + TESTED (invoices/payments/trust ledger).
- Phase 11 hardening: PARTIAL (headers/rate-limit/RBAC; lockout by parallel workstream, WAF backlog).
- Phase 12 integrations: PARTIAL (webhook verify tested; provider creds BLOCKED).
- Phase 13 QA: 89 tests green, fmt+clippy clean. - Phase 14: manifests + backup script + worker queue consumer done; live deploy pending.
- Phase 15: acceptance 24/32 = 75/100, NOT production-ready.
Multi-workstream note: parallel contributor active in workspace (esign/pay-gateway stubs, lockout, CSV export, trust unit tests). Shared files coordinated via EXECUTION-LOG; trust vocabulary unified to debit/credit.
""")
app("docs/13-development-progress/DAILY-LOG.md", """## 2026-11-17 (4) — final round
- Tokens API (create/list/revoke, Bearer dual-auth), password change (token-auth forbidden), role assignment (Assign-gated), notifications inbox+read with creation hooks, expiring document shares + public download, CSV report, role-gated audit viewer, durable jobs API + worker claim loop, SSE /events, trust ledger API (accounts/entries/transfer, no-negative guard), CSP/HSTS headers.
- Vocabulary unified (debit/credit) with parallel workstream; their stub/lockout/CSV tests kept green.
- Migration 0009 (trust). Isolated build dir (target/ws2) to avoid clobbering parallel dev-server runs.
- 89 tests green. Acceptance 24/32 = 75/100.
""")
app("docs/14-agent-workstreams/EXECUTION-LOG.md", """- 2026-11-17 coordination: parallel workstream detected (shared workspace). Convention: append-only test additions, no silent rewrites of others' handlers; shared vocabularies (trust debit/credit) agreed via code; builds isolated with CARGO_TARGET_DIR=target/ws2 to avoid locking target/debug while a dev server runs. File ownership: database store traits + server lib.rs are hot zones — announce breaking trait changes in DAILY-LOG before applying.
""")
app("CHANGELOG.md", """## [Unreleased] (final round)
- API tokens + dual-mode Bearer auth, password change, role assignment endpoint, notifications inbox, expiring document shares + public download, matters CSV report, role-gated audit viewer, durable jobs + worker consumer, SSE events, trust ledger (accounts/entries/transfer, non-negative), CSP/HSTS headers.
- Migrations 0008 (notifications/shares/jobs), 0009 (trust). 89 tests green. Acceptance 24/32 = 75/100.
""")
print("DOCS OK")
