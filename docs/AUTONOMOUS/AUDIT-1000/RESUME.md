# RESUME (AUDIT-1000 — next session entry point)

Date: 2026-11-18 (wave 4 done). HEAD: (record on commit — wave ends with commit).
Workspace: 293 passed / 0 failed; fmt+clippy clean; node --check site.js clean.
Registry: IMPLEMENTED 52 / PARTIAL 102 / NOT_STARTED 369 / PLANNED 318 / CONFIG 159.
Scores: registry 17.2/100; 8-cat mean 18.7 (18 verified). Composite ≈ 45/100 — NOT 100.
Server + Apache running detached via WMI (survives shell sessions); legalrust.test 200.

## Done wave 4
- Repertorium: DeedRow.created end-to-end, GET /api/v1/reports/deeds.csv?date= (Export-gated + audit report.deeds_csv_exported), /app akta panel card + JS, needle + API tests. F-D12-001 -> IMPLEMENTED (scope-noted), F-D20-009 -> PARTIAL.
- TASK-QUEUE: B-D12-REP -> DONE.

## Done wave 3
- Webhook delivery log: migration 0014, `webhook_log/list` (memory+MySQL),
  every receipt logged (accepted/rejected/blocked), `GET /api/v1/webhooks/deliveries`
  (View-gated, tenant-scoped) + /app audit-panel card + JS + needle tests.
  F-D08-013, F-D25-017 → PARTIAL (outbound dispatch + alerting remain).
- TASK-QUEUE: B-APPR-UI + B-REPORT-UI → DONE.

## Next executable tasks (priority order)
1. **D11 compliance slice**: UU PDP obligation register + consent lifecycle (tables + API, no UI first) -> F-D11-001/002 PARTIAL.
2. **Outbound webhook dispatch** (retry queue + HMAC sign + delivery log update) -> F-D08-013 -> IMPLEMENTED, F-D25-017 up.
3. **Mobile D24**: offline capture queue UI on top of existing outbox (large; split).
4. **External blockers** (do not fake): PSrE creds, prod deploy + benchmarks + prod-DB run, counsel sign-off, external pentest, Dukcapil/BPN creds.
5. Performance benchmarks on dev machine for new endpoints (approvals, repertorium).

## Constraints to preserve
- One workstream per file set; update FILE-OWNERSHIP.md before editing.
- Forward-only migrations; RBAC server-side; transactions for multi-step
  mutations; no secrets in code/logs; fmt/clippy/test gates per AGENTS.md.
- No push without explicit user authorization (Phase 21).
- Never inflate scores: default-scored features must be individually
  re-audited before any IMPLEMENTED claim.
