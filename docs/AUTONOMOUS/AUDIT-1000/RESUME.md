# RESUME (AUDIT-1000— next session entry point)

Date: 2026-11-18 (wave 8 done). HEAD: (record on commit — wave ends with commit).
Workspace: 304 passed / 0 failed; fmt+clippy clean; node --check clean; live MySQL migration proof 0001-0016.
Registry: IMPLEMENTED 55 / PARTIAL 105 / NOT_STARTED 366 / PLANNED 315 / CONFIG 159.
Scores: registry 17.4/100; 8-cat mean 19.0 (24 verified). Composite ~45/100 — NOT 100.
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

## Done wave 9: durable retry queue live + mobile re-audit + benchmarks (main)
- OpsStore::job_enqueue_at (memory run_at lease + MySQL DATE_ADD); integrations::process_webhook_job (send/requeue/dead-letter+audit); worker webhook.dispatch arm; notify fan-out queues retry on failure.
- 8-cat: F-D24-003/007/015/025 -> PARTIAL (outbox core verified, 12 unit tests); F-D08-013/F-D25-017 re-scored.
- Benchmarks recorded (dev, live-DB). Workspace 304/0; registry 17.6; 8-cat mean 19.0 (24 verified).

## Done wave 38: pustaka klausa berversi (main)
- Migration 0022; clause CRUD + version bump + kartu akta + revisi/hapus.
- F-D06-002 NOT_STARTED->IMPLEMENTED (76.9). Workspace 331/0; registry 20.0 (75 IMPLEMENTED); mean 20.2 (56 v).

## Done wave 37: DPIA + breach log 72 jam (main)
- Migrations 0020/0021; CRUD + guarded transitions + kepatuhan UI cards + audit.
- F-D13-007/008 -> IMPLEMENTED (76.9). Workspace 330/0; registry 19.9 (74 IMPLEMENTED); mean 20.1 (55 v).

## Done wave 36: masa berlaku token API (main)
- expires_in_days 1..365 (default 90) + enforcement di token_owner + kolom UI + unit/API tests.
- F-D16-013 PARTIAL->IMPLEMENTED (78.8). Workspace 328/0; registry 19.7 (72 IMPLEMENTED); mean 20.0 (53 v).

## Done wave 35: katalog matriks peran (main)
- GET /api/v1/roles/matrix + tabel least-privilege di Pengaturan; 12 peran, owner 12 hak, auditor minimal.
- F-D16-007 PARTIAL->IMPLEMENTED (75.6). Workspace 326/0; registry 19.6 (71 IMPLEMENTED); mean 19.9 (52 v).

## Done wave 34: konsol sesi aktif (main)
- GET /api/v1/auth/sessions + revoke others (self->409, foreign->404) + kartu Sesi aktif.
- F-D16-019 PARTIAL->IMPLEMENTED (76.9). Workspace 325/0; registry 19.5 (70 IMPLEMENTED); mean 19.9 (51 v).

## Done wave 33: kuitansi pembayaran (main)
- GET /api/v1/invoices/{id}/receipt + tombol Kuitansi + print view; receipt_viewed audit.
- F-D15-036 NOT_STARTED->IMPLEMENTED (75.6). Workspace 324/0; registry 19.4 (69 IMPLEMENTED); mean 19.9 (50 v).

## Done wave 32: viewer + catat mutasi trust di UI (main)
- Detail rekening + tabel mutasi + form debit/credit + shortcut per baris; overdraft 409.
- F-D14-011/012 frontend+testing up (86.5). Workspace 324/0; registry 19.3 (68 IMPLEMENTED); mean 19.8 (49 v).

## Done wave 31: flood guard jalur notifikasi (main)
- Per-user cap 20/min + audit rate_limited; 25-create test (<=20, >=15, audit hit).
- F-D18-029 51.8->68.1 PARTIAL (verified). Workspace 323/0; registry 19.2; mean 19.8 (47 v).

## Done wave 30: jejak audit keuangan (main)
- trust.transfer audit + filter trust/report + trail assertions.
- F-D14-037 PARTIAL->IMPLEMENTED (75.6). Workspace 322/0; registry 19.2 (68 IMPLEMENTED); mean 19.8 (46 v).

## Done wave 29: checklist PPAT + filter audit meluas (main)
- GET /api/v1/ppat/{id}/checklist + tombol Cek inline; filter audit +notification/user/ppat/report/ropa.
- F-D13-003 NOT_STARTED->PARTIAL (64.4); F-D13-005 PLANNED->PARTIAL (59.4); F-D18-028 68.5->70.9 PARTIAL.
- Workspace 322/0; registry 19.1 (67 IMPLEMENTED); 8-cat mean 19.8 (45 verified).

## Done wave 28: pusat preferensi notifikasi (main)
- Migration 0019; opt-out per kind dihormati fan-out (+audit suppressed); checkbox di dropdown lonceng.
- F-D18-006 PLANNED->IMPLEMENTED (76.9). Workspace 322/0; registry 19.0 (67 IMPLEMENTED); mean 19.6 (43 v).

## Done wave 27: register PPAT end-to-end (main)
- Migration 0018 ppat_entries; CRUD + guarded transitions; UI register card; RBAC/audit/tests.
- F-D13-001 NOT_STARTED->IMPLEMENTED (77.9). Workspace 321/0; registry 18.9 (66 IMPLEMENTED); mean 19.6 (42 v).

## Done wave 26: intake sertifikat tanah di UI Akta (main)
- Sertifikat card (buat job land_certificate + daftar job + 404 jujur); intake flow test.
- F-D13-002 51.8->67.5 PARTIAL (verified).
- Workspace 319/0; registry 18.8 (65 IMPLEMENTED); 8-cat mean 19.5 (41 verified).

## Done wave 25: riwayat versi dokumen di UI (main)
- p-dok Detail & riwayat versi card (metadata + tabel versi + 404 jujur); version lifecycle test.
- F-D04-003 frontend+testing up (86.5, verified).
- Workspace 318/0; registry 18.8 (65 IMPLEMENTED); 8-cat mean 19.5 (37 verified).

## Done wave 24: catat pembayaran dari UI Keuangan (main)
- inv_list carries paid/balance; invoice table + per-row pay form (transfer/cash/qris) + 409 message.
- F-D15-015/019 frontend+testing up (86.5); F-D15-020 51.8->62.5 PARTIAL (credit-memo missing).
- Workspace 317/0; registry 18.8 (65 IMPLEMENTED); 8-cat mean 19.5 (37 verified).

## Done wave 23: org user directory + cross-org grant block (main)
- GET /api/v1/users (Assign) + assign org-check + user.role_assigned audit; pengaturan role dropdown.
- IDOR fix: foreign-org grant -> 404. F-D16-001 PARTIAL->IMPLEMENTED (76.9); F-D16-029 51.8->67.9 PARTIAL.
- Workspace 316/0; registry 18.7 (65 IMPLEMENTED); 8-cat mean 19.5 (36 verified).

## Done wave 22: notification filter control + manual (main)
- appr-notif "Hanya belum dibaca" checkbox + manual section 4 Notifikasi; F-D18-038 PARTIAL->IMPLEMENTED (74.4).
- Note: 1 transient api failure seen once, green on 4 reruns (name not captured; watchlist).
- Workspace 315/0; registry 18.6 (64 IMPLEMENTED); 8-cat mean 19.4 (34 verified).

## Done wave 21: notification audit trail (main)
- notify_user audits notify.{kind}.sent; notify_read audits notification.read; ?entity=notification assertions.
- F-D18-028 51.8->68.5 (stays PARTIAL: no dedicated audit-surface UI).
- Workspace 315/0; registry 18.5 (63 IMPLEMENTED); 8-cat mean 19.4 (34 verified).

## Done wave 20: public newsletter unsubscribe (main)
- POST /api/v1/public/newsletter/unsubscribe (rate bucket + email validation) + Berhenti button + lifecycle test.
- F-D18-026 51.8->69.4 (stays PARTIAL: no consent timestamp/preference center).
- Workspace 315/0; registry 18.5 (63 IMPLEMENTED); 8-cat mean 19.4 (33 verified).

## Done wave 19: notification unread/limit polish (main)
- GET /api/v1/notifications?unread_only=&limit= (clamp 1..100) + DESC order both backends; bell dropdown server-side.
- Isolation/clamp/400/401 tests. F-D18-038 51.8->67.9 (stays PARTIAL: visible filter UI + docs follow-up).
- Workspace 314/0; registry 18.5 (63 IMPLEMENTED); 8-cat mean 19.4 (32 verified).

## Done wave 18: AR aging report piutang terbuka (main)
- inv_list_open (memory aggregate + MySQL LEFT JOIN/HAVING) + ar_aging_csv + GET /api/v1/reports/ar-aging.csv (Export+audit) + kartu Keuangan.
- Partial/overpay/full-pay API test + reporting unit test. F-D14-004 PLANNED->IMPLEMENTED (8-cat 81.6).
- Workspace 313/0; registry 18.5 (63 IMPLEMENTED); 8-cat mean 19.4 (31 verified).

## Done wave 17: papan skor transparan 1000 fitur (main)
- GET /api/v1/features/scores (auth+View, filter domain/status/q, page/per_page, summary jujur) + halaman /skor (chips, filter, tabel, pager) + link sidebar.
- Skor TETAP jujur: hanya kerja nyata yang bernilai; tidak ada fitur yang dipalsukan 100.
- Workspace 311/0; registry 18.4 (62 IMPLEMENTED); 8-cat mean 19.3 (30 verified).

## Done wave 16: sidebar 1:1 per tab, menu ganda hilang (main)
- 25 snav duplikat (ocr x4, klien x4, dok x3, akta x4, ...) -> 12 link 1:1 per tab; tiap klik kini 1 item aktif.
- Tab Keuangan (p-keu) kini reachable dari sidebar (sebelumnya hanya via quick-action).
- dash_app_has_sidebar_nav_groups: label baru + uniqueness assert tiap data-tab tepat 1x.
- Workspace 309/0; registry 18.4 (62 IMPLEMENTED); 8-cat mean 19.3 (30 verified).

## Done wave 15: kind-routed approval notifications + D08-003 re-score (main)
- GET /api/v1/notifications?kind= (<=64 else 422) + appr-tab card (5 latest, unread badge, mark-read).
- notifications_kind_filter_and_read test. F-D08-015 PARTIAL->IMPLEMENTED (8-cat 77.9).
- F-D08-003 stale NOT_STARTED->IMPLEMENTED (sequential engine evidence, 8-cat 82.2, no code change).
- Workspace 309/0; registry 18.4 (62 IMPLEMENTED); 8-cat mean 19.3 (30 verified).

## Done wave 14: global search hardening + RAG ACL-trim (main)
- /api/v1/search: page/per_page/types + totals (arrays kept); rag_search ranks via rank() + org-trim at boundary.
- UI pager + totals; pagination/types/isolation tests. F-D23-001/021/031 PARTIAL->IMPLEMENTED (8-cat 74.4).
- Workspace 308/0; registry 18.1 (60 IMPLEMENTED); 8-cat mean 19.2 (28 verified).

## Done wave 13: repertorium period report from/to (main)
- deeds.csv supports inclusive ?from=&to= (YYYY-MM-DD) + from>to guard; ?date= kept; /app card now from/to with client guard; period + guard API tests.
- F-D20-009 PARTIAL->IMPLEMENTED (8-cat 76.9). Workspace 306/0; registry 17.8 (57 IMPLEMENTED); 8-cat mean 19.1 (25 verified).

## Done wave 12: /app ROPA UI in kepatuhan tab (main)
- ROPA card (tambah/muat-ulang/hapus) + ropaRefresh wiring + tab hook + boot call; needle test; frontend 0->80 (8-cat 80.0).
- Workspace 306/0; registry 17.7 (56 IMPLEMENTED); 8-cat mean 19.1 (25 verified).

## Done wave 11: ROPA register (UU PDP) + compliance test fixes (main)
- POST/GET /api/v1/compliance/ropa + DELETE /{id}; migration 0017; memory+MySQL impls; Create/View/Delete RBAC + validator + audit; api test (422/403/404) + memory tenant-isolation test.
- Bugfix nyata: memory ropa_delete menghapus baris walau org beda (check-after-remove); diperbaiki ke check-before-remove ala meeting_delete/webhook_ep_delete.
- Test-file fix: payload ROPA pakai `activity` (bukan `title`); hapus stray eprintln debug.
- Workspace 306/0; registry 17.7 (56 IMPLEMENTED); 8-cat mean 19.1 (25 verified).

## Done wave 10: dead-letter alerting v1 + benchmarks baseline (main)
- Dead-letter audit entry on max-retry exhaustion; ?status= filter on deliveries; /app Dead letters card + needles; status-filter API test.
- Workspace 304/0; registry 17.6 (55+2 re-scored); 8-cat mean 19.0.

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
