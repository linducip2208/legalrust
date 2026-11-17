# EXECUTION BASELINE (2026-11-18)

Autonomous orchestration baseline. Docs-only snapshot; no code or migrations touched.

## VCS state
- HEAD SHA: `9ab151e9edb0bde95c5f359cbffa88924bd44e17`
  (`git log --oneline -5` head: `9ab151e PWA manifest live + CRM UI + 241 tests green`)
- Branch: `main`
- Tree state: 1 modified file — `M docs/AUTONOMOUS/TASK-QUEUE.json` (statuses updated).
  No other modified / staged / untracked files reported by `git status --short`.

## Architecture (one paragraph)
Axum monolith in `apps/server/src/lib.rs` (~100 routes + inline-HTML handlers) behind Laragon
proxy (`:80` → server `:8080`, HSTS+CSP headers); single auth choke point `auth_user()`
(JWT→session_live else SHA-256 API-token lookup) with per-query `org_id` tenant scoping and
role checks; trait-based stores (`crates/database`: UserStore/CoreStore/OpsStore) over MySQL
(migrations 0001–0010) with in-memory fallback for dev/test; jobs table as queue drained by
`apps/worker`, SSE events, audit writes; `apps/web` (Leptos) and `apps/mobile` are stubs;
external providers (OCR engine, AI, PSrE e-sign, payment gateway, Dukcapil/BPN) sit behind
Rust adapters returning 501 `BlockedExternal` until credentials exist. Full map:
`docs/AUTONOMOUS/ARCHITECTURE.md`.

## Test baseline
- HEAD commit message claims **241 tests green**.
- `docs/AUTONOMOUS/TEST-RESULTS.md` **does not exist** (verified: not in
  `docs/AUTONOMOUS/` listing) — the 241 figure could not be verified against that file.
- Recorded measured baseline in `docs/13-development-progress/ACCEPTANCE.md` and
  `docs/AUTONOMOUS/MASTER-PROGRESS.md`: **224 passed, 0 failed, 1 ignored (live-DB)**,
  measured 2026-11-18 (`cargo test --workspace`, `SQLX_OFFLINE=true`).
- `cargo fmt --check` RED (2 files need rewrapping) per MASTER-PROGRESS.
- DISCREPANCY (241 claimed vs 224 measured) must be resolved by a fresh
  `cargo test --workspace` run before any gate decision.

## Feature counts (computed live from `FEATURE-REGISTRY.json`, n=1000)
| Status | Count | % |
|---|---|---|
| IMPLEMENTED | 37 | 3.7% |
| PARTIAL | 104 | 10.4% |
| CONFIGURATION_REQUIRED | 159 | 15.9% |
| PLANNED | 326 | 32.6% |
| NOT_STARTED | 374 | 37.4% |

Per-domain breakdown: see `DOMAIN-STATUS.md`. Live/actionable (IMPLEMENTED+PARTIAL): 141.
Externally blocked: 159. No-evidence yet (PLANNED+NOT_STARTED): 700.

## Acceptance score
28/32 = 87/100 (recount 2026-11-18, unchanged — see `docs/13-development-progress/ACCEPTANCE.md`).
PARTIAL: OCR engine live, AI provider live, final security audit (self-audit only).
MISSING: production deployment + prod benchmarks + prod-DB run + counsel sign-off.

## Blockers (from `TASK-QUEUE.json` + `BLOCKERS.md`)
- B-OCR: OCR engine live — BLOCKED (native lib / cloud creds).
- B-AI: AI provider activation — BLOCKED (API credentials).
- B-ESIGN: PSrE e-sign activation — BLOCKED (PSrE credentials).
- B-PAY: Payment gateway activation — BLOCKED (gateway credentials).
- B-COUNSEL: counsel sign-off + prod deploy — BLOCKED (human/infra).
- BL-LIC-01: LICENSE intent TBD by owner.
- BL-GPU-01: heavy WAN rendering restricted (RTX 4060 8GB) — stills only.

## Timestamp
2026-11-18 (UTC+7). Next step: resolve test-count discrepancy, then dispatch waves per
`AGENT-ASSIGNMENTS.json`.
