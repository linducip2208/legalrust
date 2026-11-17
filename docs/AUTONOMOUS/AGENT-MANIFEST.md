# AGENT MANIFEST — 100 logical workers (25 domains x 4 roles)

Docs-only orchestration plan. No code or migrations touched by this manifest.

## Structure
100 logical workers: `agent-Dxx-R` where `Dxx` = domain D01–D25, `R` = role A/B/C/D.
Concrete mapping (agent, scopes, wave): `AGENT-ASSIGNMENTS.json` (100 entries, validated).

## Domains (25)
D01 Identity & KYC · D02 Client & CRM · D03 Project & Case Management ·
D04 Document Management · D05 OCR & Document Intelligence · D06 Deed Generation ·
D07 AI Legal Assistant · D08 Workflow Automation · D09 Approval Management ·
D10 Electronic Signature · D11 Compliance & Risk · D12 Notary Office Management ·
D13 PPAT & Property Legal · D14 Finance & Accounting · D15 Billing & Subscription ·
D16 User, Role & Permission · D17 Multi-Office & Tenant ·
D18 Communication & Notification · D19 Client & Public Portal ·
D20 Reporting & Analytics · D21 API & Integration · D22 Security & Data Protection ·
D23 Search & Discovery · D24 Mobile & Field Operations ·
D25 Enterprise Administration & Global Expansion.

## Roles (4 per domain)
- **A — Builder.** Implements PLANNED/NOT_STARTED items inside the domain's `scope_files`.
  Sole code writer while its wave is active.
- **B — Verifier.** Reproduces evidence, writes/updates tests, maintains TEST-RESULTS.
  Read-only on A's files mid-wave; reports mismatches, never silently fixes A's code.
- **C — Scribe.** Docs-only: domain docs, ACCEPTANCE deltas, evidence packs.
- **D — Gatekeeper.** Enforces gates (RBAC server-side, DB transactions, fmt/check/clippy/test
  green, no fake data, BLOCKED=501+docs for externals). Reviews diffs, marks BLOCKED.
  Writes no feature code.

## File-ownership rules
1. One workstream owns one file set at a time (AGENTS.md rule). A's `scope_files` are
   exclusive per active wave; no two active waves share a scope file.
2. B owns `tests/` (domain-scoped) + `docs/AUTONOMOUS/TEST-RESULTS.md`.
3. C owns `docs/13-development-progress/` + `docs/AUTONOMOUS/Dxx-*.md`.
4. D owns gates + `docs/AUTONOMOUS/BLOCKERS.md` entries; D never writes feature code.
5. Migrations: forward-only, one new migration per change, reviewed — owned by the
   domain-A of the active wave, approved by its D before commit.
6. Cross-domain files (`apps/server/src/lib.rs`, `openapi.yaml`) are edited by only one
   active A at a time; others queue via TASK-QUEUE.json dependencies.
7. Every multi-step mutation runs in a DB transaction; every endpoint enforces RBAC
   server-side; no secrets in code/logs; no TODO in critical paths.

## Bounded concurrency policy
- Max **4 active agents per wave** (one domain's A+B pair plus one C and one D shared),
  max **2 concurrent waves**, max **8 active agents** globally.
- Waves execute in order 1→10 (mapping in AGENT-ASSIGNMENTS.json); a wave's D must sign
  gates (fmt, check, clippy, test, docs update, evidence recorded) before the next wave's
  A starts on shared files.
- Externally blocked items (CONFIGURATION_REQUIRED) are marked BLOCKED (501 + docs) and
  skipped — workers pick the next unblocked TODO per TASK-QUEUE.json dependencies, never
  idle-wait on credentials.
- Stop conditions: gate red (fmt/check/clippy/test), evidence mismatch vs registry,
  new BLOCKED external, or counsel/prod step reached → halt wave, record in BLOCKERS.md,
  resume per `scripts/autonomous/resume-instructions.md`.

## Wave map (domain → wave)
- W1 Foundation: D16, D17, D22 · W2 Core CRM/Matters: D02, D03 ·
  W3 Docs/Deeds: D04, D06, D12 · W4 Identity/OCR: D01, D05 ·
  W5 AI/Esign: D07, D10 · W6 Flow/Approval: D08, D09 ·
  W7 Business: D14, D15, D11 · W8 Comms/Portal: D18, D19, D13 ·
  W9 Data/API: D20, D23, D21 · W10 Edge/Sweep: D24, D25.
