# RESUME (AUDIT-1000 — next session entry point)

Date: 2026-11-18. HEAD: (record on commit — wave ends with commit).
Workspace: 288 passed / 0 failed; fmt+clippy clean.
Registry: IMPLEMENTED 38 / PARTIAL 112 / PLANNED 319 / NOT_STARTED 372 /
CONFIG 159. Mean overall 18.2/100. Composite quality ≈ 44/100.

## Done this wave
- Approvals engine backend (migration 0013, trait+2 impls, 5 routes+RBAC,
  3 tests, CSV audit events, manifest PWA icons, registry +10 updates,
  AUDIT-1000 artifacts incl. this file).

## Next executable tasks (priority order)
1. **Approvals UI tab in /app** (compose modal, pending list, decide buttons
   with comments, history view) → flips F-D09-001/008/010/015/017/024/025/032
   PARTIAL→IMPLEMENTED. Files: `apps/server/src/lib.rs` (app_page body),
   `apps/server/static/site.js`. Tests: needle tests for new IDs.
2. **Report center**: wire `trust_ledger_csv` + `matter_summary_csv`
   (already in `crates/reporting`) to Export-gated routes + audit; add
   `/app` export buttons. → F-D20-001/004 up.
3. **Queue-depth endpoint** for jobs (`count by status`) → F-D25-009 up;
   surface on /app jobs tab.
4. **Outbound webhook delivery log** (store attempts on webhook_in +
   list endpoint) → F-D08-013 from NOT_STARTED, F-D25-017 started.
5. **Individual 8-cat audits**: continue domain by domain (D08→D09 done
   partially; next D20, D25, D11); update FEATURE-SCORES.json entries from
   defaults to verified (set `verified_8cat` per feature).
6. **External blockers** (do not fake): PSrE creds, prod deploy + benchmarks
   + prod-DB run, counsel sign-off, external pentest, Dukcapil/BPN creds.
7. **Mobile (D24)**: offline UI on top of existing outbox core — large;
   split into capture-queue first, sync-conflict UI second.

## Constraints to preserve
- One workstream per file set; update FILE-OWNERSHIP.md before editing.
- Forward-only migrations; RBAC server-side; transactions for multi-step
  mutations; no secrets in code/logs; fmt/clippy/test gates per AGENTS.md.
- No push without explicit user authorization (Phase 21).
- Never inflate scores: default-scored features must be individually
  re-audited before any IMPLEMENTED claim.
