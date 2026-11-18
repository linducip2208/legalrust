# RESUME (AUDIT-1000— next session entry point)

Date: 2026-11-18 (wave 8 done). HEAD: (record on commit — wave ends with commit).
Workspace: 299 passed / 0 failed; fmt+clippy clean; node --check clean; live MySQL migration proof 0001-0016.
Registry: IMPLEMENTED 55 / PARTIAL 101 / NOT_STARTED 368 / PLANNED 317 / CONFIG 159.
Scores: registry 17.4/100; 8-cat mean 18.9 (20 verified). Composite ~45/100 — NOT 100.
Server + Apache running detached via WMI (survives shell sessions); legalrust.test 200.

## Done wave 8: durable webhook retry queue (main)
- OpsStore::job_enqueue_at (memory run_at lease + MySQL DATE_ADD); integrations::process_webhook_job (send/requeue/dead-letter); worker webhook.dispatch arm; notify fan-out queues retry on failure.
- 4 new tests (enqueue_at lease + 3 dispatch paths); workspace 299/0; registry 17.4/100 (55 IMPLEMENTED); 8-cat mean 18.9 (20 verified).

## Done wave 5: compliance slice (UU PDP obligations + consent API) + repertorium UI verification
- Migration 0015, 6 API endpoints + RBAC + audit, tests (API + store); live MySQL migration proof 0001-0015; api_db stale-test fixed.
- Registry: F-D11-001/002 -> PARTIAL; verified_8cat 18 -> 20.

## Done wave 4
- Repertorium: DeedRow.created end-to-end, GET /api/v1/reports/deeds.csv?date= (Export-gated + audit report.deeds_csv_exported), /app akta panel card + JS, needle + API tests. F-D12-001 -> IMPLEMENTED (scope-noted), F-D20-009 -> PARTIAL.
- TASK-QUEUE: B-D12-REP -> DONE.

## Done wave 3
- Webhook delivery log: migration 0014, `webhook_log/list` (memory+MySQL),
  every receipt logged (accepted/rejected/blocked), `GET /api/v1/webhooks/deliveries`
  (View-gated, tenant-scoped) + /app audit-panel card + JS + needle tests.
  F-D08-013, F-D25-017 → PARTIAL (outbound dispatch + alerting remain).
- TASK-QUEUE: B-APPR-UI + B-REPORT-UI → DONE.

## Done wave 6: compliance /app tab (panel kepatuhan + JS + needles)
- F-D11-001/002 PARTIAL -> IMPLEMENTED. Scores: registry 17.4/100.

## Done wave 6: outbound webhook endpoints + migration proof (main)
- Migration 0016 (webhook_endpoints + direction); endpoints CRUD + signed test-send + delivery log + /app card + needles; delivery log covers in+out.
- Migration 0001-0016 live-verified on Laragon MySQL; api_db stale-test fixed.
- Registry 17.4 (54 impl); 8-cat mean 18.9 (20 verified). Workspace 298/0.

## Done wave 7: auto-dispatch on notify events (main)
- Fan-out background di notify_user ke endpoint terdaftar (signed post_json, timeout 10s, delivery log accepted/rejected).
- Tes signed-stub E2E (matter create -> dispatch -> log); 1 transient stub-timing failure seen once, unreproduced in 3 full reruns.
- Registry tetap 54 IMPLEMENTED; 8-cat mean 18.9 (20 verified, F-D08-013/F-D25-017 re-scored). Workspace 299/0.

## Next executable tasks (priority order)
1. **Outbound auto-dispatch** -> DONE wave 7 (signed fan-out; retry remains queued).
2. **Webhook retry queue + dead-letter + failure alerts** (B-WH-RETRY) -> F-D08-013/F-D25-017 up.
3. **Mobile D24**: offline capture queue UI on top of existing outbox (large; split).
4. **External blockers** (do not fake): PSrE creds, prod deploy + benchmarks + prod-DB run, counsel sign-off, external pentest.
5. Performance benchmarks on dev machine for new endpoints.

## Constraints to preserve
- One workstream per file set; update FILE-OWNERSHIP.md before editing.
- Forward-only migrations; RBAC server-side; transactions for multi-step
  mutations; no secrets in code/logs; fmt/clippy/test gates per AGENTS.md.
- No push without explicit user authorization (Phase 21).
- Never inflate scores: default-scored features must be individually
  re-audited before any IMPLEMENTED claim.
