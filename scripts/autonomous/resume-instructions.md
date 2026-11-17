# Resume Instructions (next autonomous session)

Read first, in order:
1. `docs/AUTONOMOUS/EXECUTION-BASELINE.md` — HEAD, counts, blockers, known discrepancies.
2. `docs/AUTONOMOUS/MASTER-PROGRESS.md` + `docs/AUTONOMOUS/TASK-QUEUE.json` — queue + owners.
3. `docs/AUTONOMOUS/AGENT-MANIFEST.md` + `docs/AUTONOMOUS/AGENT-ASSIGNMENTS.json` — who works on what.
4. `docs/AUTONOMOUS/DOMAIN-STATUS.md` + `docs/AUTONOMOUS/FEATURE-EVIDENCE.json` — evidence so far.
5. `docs/13-development-progress/ACCEPTANCE.md` — 28/32 checklist + PARTIAL/MISSING.
6. `AGENTS.md` — working rules (gates, RBAC, tx, no destructive migrations).

First action — resolve open discrepancy:
- `TEST-RESULTS.md` is missing but HEAD claims 241 green while ACCEPTANCE records
  224 passed / 0 failed / 1 ignored. Run `cargo test --workspace` (with
  `SQLX_OFFLINE=true` when no DB) and record the true count before gate decisions.

Standard commands:
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (`SQLX_OFFLINE=true` when no DB)

Loop: pick highest-priority unblocked TODO per TASK-QUEUE.json dependencies →
respect file-ownership + bounded concurrency (max 8 active, one writer per file set) →
fmt/check/clippy/test → commit milestone → update TASK-QUEUE.json, MASTER-PROGRESS.md,
`docs/13-development-progress/ACCEPTANCE.md`, and `docs/13-development-progress` package doc.

Stop conditions (halt wave, record in BLOCKERS.md, do not force through):
- Any gate red (fmt / check / clippy / test).
- Evidence mismatch vs FEATURE-REGISTRY.json.
- New external dependency without credentials → mark BLOCKED (501 + docs), take next TODO.
- Counsel sign-off / prod deploy / LICENSE decision reached → BLOCKED, needs human.
- Never force-push / reset-hard / delete migrations.
