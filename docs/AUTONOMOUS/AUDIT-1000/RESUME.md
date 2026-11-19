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

## Done wave 48: telemetri akurasi OCR (main)
- Aggregate per-tipe + CSV + tombol + trail; F-D20-010 NOT_STARTED->PARTIAL (64.1).
- Workspace 342/0; registry 19.8 (77 IMPLEMENTED); 8-cat mean 20.8 (67 v).

## Done wave 53: matriks multi-level + substitute via delegasi (main)
- Verifikasi engine + delegasi sebagai substitute; F-D09-002 PARTIAL (69.4), F-D09-020 PARTIAL (67.5).
- Tanpa perubahan kode. Registry 20.2 (80 IMPLEMENTED); 8-cat mean 21.1 (73 v).

## Done wave 52: audit trail kepatuhan + analitik approval (main)
- Entity-scoped compliance trails verified; bottleneck snapshot live.
- F-D11-039 PARTIAL->IMPLEMENTED (76.9); F-D09-027 PLANNED->PARTIAL (68.1).
- Workspace 344/0 (no code change; evidence+rescore). Registry 20.2 (80 IMPLEMENTED); mean 21.0 (71 v).

## Done wave 138: bundle + scan queue (main)
- matter bundle (base64, capped) + image auto-OCR-queue + kartu + uji.
- F-D04-011 PLANNED->IMPLEMENTED, 025 PARTIAL->IMPLEMENTED (~72.0 verified); 026 documented BLOCKED (no mail infra). Workspace 426/0; registry ~28.8 (245 IMPLEMENTED); mean 31.5 (235 v).

## Done wave 137: quota + sync (main)
- org_quotas (0084) + sync feed + enforcement + kartu + uji.
- F-D04-029/034 PLANNED->IMPLEMENTED (~72.3 verified). Workspace 425/0; registry ~28.6 (243 IMPLEMENTED); mean 31.4 (233 v).

## Done wave 159: portal inbox + secure upload (main)
- portal inbox + ack + ownership guards on shared doc routes + kartu + uji.
- F-D19-005 PLANNED->IMPLEMENTED, F-D19-006 NOT_STARTED->IMPLEMENTED (~73.6 verified). Workspace 449/0; registry ~30.7 (287 IMPLEMENTED); mean 34.2 (277 v).
Note: closed a real tenant leak (client Download/Create had no book scope).

## Done wave 158: portal dashboard + progress (main)
- portal_client gate + dashboard/matters (identity scope) + kartu + uji.
- F-D19-003/004 NOT_STARTED->IMPLEMENTED (~73.0 verified). Workspace 448/0; registry ~30.6 (285 IMPLEMENTED); mean 34.1 (275 v).

## Done wave 157: portal login + session (main)
- cpi_get_by_token + portal login/me (client JWT, 8h, revocable) + kartu + uji.
- F-D19-001 PLANNED->IMPLEMENTED, F-D19-030 NOT_STARTED->IMPLEMENTED (~73.2 verified). Workspace 447/0; registry ~30.5 (283 IMPLEMENTED); mean 34.0 (273 v).

## Done wave 156: conflict screening — D02 CLOSED (main)
- pparty_search + /conflicts/check (3-surface, human-review note) + kartu + uji.
- F-D02-008 CONFIGURATION_REQUIRED->IMPLEMENTED (~72.8 verified; stale OCR evidence corrected). Workspace 446/0; registry ~30.4 (281 IMPLEMENTED); mean 33.8 (271 v).
D02 CRM CLOSED 40/40.

## Done wave 155: 360 view + contact roles (main)
- client overview aggregation + employee contact_role (0101) + kartu + uji.
- F-D02-005/035 PARTIAL->IMPLEMENTED (~72.8 verified). Workspace 445/0; registry ~30.4 (280 IMPLEMENTED); mean 33.8 (270 v).

## Done wave 154: prospect retention (main)
- purgeable prospect + memory/mysql arms (cascade) + uji; no migration.
- F-D02-039 NOT_STARTED->IMPLEMENTED (~72.6 verified). Workspace 444/0; registry ~30.3 (278 IMPLEMENTED); mean 33.7 (268 v).

## Done wave 153: CRM dashboard + client expiry (main)
- crm/dashboard derivation + expiry client_id filter (join) + kartu + uji; no migration.
- F-D02-037/040 PLANNED/NOT_STARTED->IMPLEMENTED (~72.5 verified). Workspace 443/0; registry ~30.2 (277 IMPLEMENTED); mean 33.7 (267 v).

## Done wave 152: risk + activity (main)
- client_risk_profiles (0100, upsert) + per-client activity scope + kartu + uji.
- F-D02-029/038 NOT_STARTED->IMPLEMENTED (~72.4 verified). Workspace 442/0; registry ~30.1 (275 IMPLEMENTED); mean 33.5 (265 v).

## Done wave 151: events + seminars (main)
- client_events + attendances (0099, held/no_show chains) + kartu + uji.
- F-D02-023 PLANNED->IMPLEMENTED (~72.4 verified). Workspace 441/0; registry ~30.0 (273 IMPLEMENTED); mean 33.4 (263 v).
Note: /api/v1/events SSE collision fixed via /api/v1/client-events prefix.

## Done wave 150: campaign tracking (main)
- marketing_campaigns + participations (0098, response chain) + kartu + uji.
- F-D02-022 PLANNED->IMPLEMENTED (~72.4 verified). Workspace 440/0; registry ~30.0 (272 IMPLEMENTED); mean 33.3 (262 v).

## Done wave 149: portal invites + surveys (main)
- client_portal_invites + client_surveys (0097, token-once + avg) + kartu + uji.
- F-D02-017/018 PLANNED->IMPLEMENTED (~72.6 verified). Workspace 439/0; registry ~29.9 (271 IMPLEMENTED); mean 33.3 (261 v).

## Done wave 148: quotations rescore + engagement letters (main)
- client_engagement_letters (0096, issue/sign lifecycle) + kartu + uji; 013 verify-rescore.
- F-D02-013 PLANNED->IMPLEMENTED (~70.1 verified, string-linked note), F-D02-014 PLANNED->IMPLEMENTED (~72.4). Workspace 438/0; registry ~29.8 (269 IMPLEMENTED); mean 33.1 (259 v).

## Done wave 147: notif routing + export (main)
- client_notif_routes (0095) + resolve plan + controlled CSV + kartu + uji.
- F-D02-034/036 NOT_STARTED/PLANNED->IMPLEMENTED (~72.5 verified). Workspace 437/0; registry ~29.7 (267 IMPLEMENTED); mean 33.0 (257 v).

## Done wave 146: reminders + feedback (main)
- client_reminders + client_feedback (0094, window + branches) + kartu + uji.
- F-D02-032/033 NOT_STARTED->IMPLEMENTED (~72.4 verified). Workspace 436/0; registry ~29.6 (265 IMPLEMENTED); mean 32.9 (255 v).

## Done wave 145: VIP + credit (main)
- client_vips + client_credit (0093, exposure math) + kartu + uji.
- F-D02-030/031 NOT_STARTED->IMPLEMENTED (~72.4 verified). Workspace 435/0; registry ~29.5 (263 IMPLEMENTED); mean 32.7 (253 v).

## Done wave 144: roster + consents (main)
- client_employees + client_consents (0092, upsert) + kartu + uji.
- F-D02-028/027 NOT_STARTED/PLANNED->IMPLEMENTED (~72.4 verified). Workspace 434/0; registry ~29.4 (261 IMPLEMENTED); mean 32.6 (251 v).

## Done wave 143: merge + corrections (main)
- CoreStore update/repoint/delete-if-empty + cmerge Tx + corrections (0091) + kartu + uji.
- F-D02-025/026 NOT_STARTED->IMPLEMENTED (~73.2 verified). Workspace 433/0; registry ~29.3 (259 IMPLEMENTED); mean 32.4 (249 v).

## Done wave 142: pipeline + win-back (main)
- client_opps + client_winbacks (0090, stage chains) + kartu + uji.
- F-D02-021/024 PLANNED/NOT_STARTED->IMPLEMENTED (~72.4 verified). Workspace 432/0; registry ~29.2 (257 IMPLEMENTED); mean 32.3 (247 v).

## Done wave 141: referrals + LTV (main)
- client_referrals (0089) + LTV derivation + kartu + uji; repaired wave-140 dead buttons (fee/doc-request JS wiring was missing, honest fix).
- F-D02-019/020 NOT_STARTED->IMPLEMENTED (~72.4 verified). Workspace 431/0; registry ~29.1 (255 IMPLEMENTED); mean 32.2 (245 v).

## Done wave 140: fees + doc requests (main)
- client_fees + client_doc_requests (0088, branch chains) + kartu + uji.
- F-D02-015/016 NOT_STARTED->IMPLEMENTED (~72.4 verified). Workspace 430/0; registry ~29.0 (253 IMPLEMENTED); mean 32.0 (243 v).

## Done wave 139: communication timeline (main)
- client timeline aggregation (no tables) + kartu + uji; 012 verify-rescore.
- F-D02-011 NOT_STARTED->IMPLEMENTED, 012 PLANNED->IMPLEMENTED (~72.1 verified). Workspace 429/0; registry ~28.9 (251 IMPLEMENTED); mean 31.9 (241 v).

## Done wave 138: client lifecycle checklists (main)
- client_checklists + items (0086, kind chains) + kartu + uji.
- F-D02-009/010 NOT_STARTED->IMPLEMENTED (~72.6 verified). Workspace 428/0; registry ~28.8 (249 IMPLEMENTED); mean 31.7 (239 v).

## Done wave 137: client hierarchy + segments (main)
- client_links + client_tags (0085, cycle DFS) + kartu + uji.
- F-D02-006/007 NOT_STARTED/PLANNED->IMPLEMENTED (~72.2 verified). Workspace 427/0; registry ~28.7 (247 IMPLEMENTED); mean 31.6 (237 v).

## Done wave 136: redaction + segregation (main)
- text masking + protocol register (no tables) + kartu + uji.
- F-D04-020/040 NOT_STARTED/PLANNED->IMPLEMENTED (~72.1 verified). Workspace 424/0; registry ~28.4 (241 IMPLEMENTED); mean 31.3 (231 v).

## Done wave 135: chunked upload + preview (main)
- upload sessions + preview card + kartu + uji.
- F-D04-035 NOT_STARTED->IMPLEMENTED, 012 PARTIAL->IMPLEMENTED (~72.2 verified). Workspace 423/0; registry ~28.3 (239 IMPLEMENTED); mean ~31.1 (229 v).

## Done wave 134: doc approval routing + flake fix (main)
- entity=document in approval engine + attachments + select; 022 credited correctly, 023 kept.
- Fixed real flake root cause: shared temp object dir across parallel tests -> unique scratch per dev_state().
- F-D04-022 corrected->IMPLEMENTED (real), 023 IMPLEMENTED. Workspace 422/0 x2. Registry ~28.5 (237 IMPLEMENTED); mean ~31.0 (227 v).

## Done wave 133: duplicates + manifest + index (main)
- sha-groups + manifest + auto-index (no tables) + kartu + uji.
- F-D04-015/032/013/036 PARTIAL->IMPLEMENTED (~72.4 verified). Workspace 421/0; registry ~28.4 (234 IMPLEMENTED); mean 31.0 (226 v).

## Done wave 132: folders + dashboard (main)
- folders + stats derivations (no tables) + kartu + uji.
- F-D04-002/038 PARTIAL->IMPLEMENTED (~72.1 verified). Workspace 420/0; registry ~28.4 (232 IMPLEMENTED); mean 30.9 (222 v).

## Done wave 131: diff + expiry (main)
- line-LCS diff + expiry feed (no tables) + kartu + uji.
- F-D04-021/022 NOT_STARTED/PLANNED->IMPLEMENTED (~72.1 verified). Workspace 419/0; registry ~28.3 (230 IMPLEMENTED); mean 30.8 (220 v).

## Done wave 130: watermark + stamp (main)
- text-layer marking (no tables) + kartu + uji.
- F-D04-018/019 NOT_STARTED/PLANNED->IMPLEMENTED (~72.1 verified). Workspace 418/0; registry ~28.2 (228 IMPLEMENTED); mean 30.7 (218 v).

## Done wave 129: locks + favorites (main)
- doc_locks + doc_favs (0084) + write gate + kartu + uji.
- F-D04-004/027 NOT_STARTED->IMPLEMENTED (~72.4 verified). Workspace 417/0; registry ~28.2 (226 IMPLEMENTED); mean 30.5 (216 v).

## Done wave 128: doc metadata (main)
- meta_fields + doc_meta (0083) + schema validation + kartu + uji.
- F-D04-005/006/037 PLANNED/NOT_STARTED->IMPLEMENTED (~72.5 verified). Workspace 416/0; registry ~28.1 (224 IMPLEMENTED); mean 30.4 (214 v).

## Done wave 127: doc lifecycle (main)
- doc_lifecycle (0082) + CoreStore::delete_doc + gates + kartu + uji.
- F-D04-007/008/009/010/031 NOT_STARTED/PLANNED->IMPLEMENTED (~73.2 verified). Workspace 415/0; registry ~28.0 (221 IMPLEMENTED); mean 30.2 (211 v).

## Done wave 126: versioning + fast-track (main)
- approval_policies (0081) + versioned resolve + fast-track lane + kartu + uji.
- F-D09-021/022 PLANNED/NOT_STARTED->IMPLEMENTED (~73.6 verified). Workspace 414/0; registry ~28.0 (216 IMPLEMENTED); mean 29.8 (206 v).

## Done wave 125: trend + mobile inbox (main)
- approvals-trend + /m/inbox (no tables) + kartu + uji.
- F-D09-027 PARTIAL->IMPLEMENTED, 006 NOT_STARTED->IMPLEMENTED (~72.5 verified). Workspace 413/0; registry ~27.9 (214 IMPLEMENTED); mean 29.7 (204 v).

## Done wave 124: token API + substitute (main)
- integration token e2e (mint/use/revoke); 020 verify-close via delegation.
- F-D09-030 NOT_STARTED->IMPLEMENTED, 020 PARTIAL->IMPLEMENTED (~72.4 verified). Workspace 412/0; registry ~27.9 (212 IMPLEMENTED); mean 29.6 (203 v).

## Done wave 123: export + matrix (main)
- approvals.csv auditor columns + filter; 002 verify-close via routing.
- F-D09-035/002 PARTIAL->IMPLEMENTED (~72.3 verified). Workspace 411/0; registry ~27.9 (210 IMPLEMENTED); mean 29.5 (202 v).

## Done wave 122: chain + redaction (main)
- approval_receipts (0080, hash chain) + redact endpoint + kartu + uji.
- F-D09-036/038 NOT_STARTED/PLANNED->IMPLEMENTED (~73.0 verified). Workspace 411/0 (earlier single failure was a bad JS-rendered needle, fixed). Registry ~27.9 (208 IMPLEMENTED); mean 29.5 (202 v).

## Done wave 121: quorum + override (main)
- quorum_rounds/votes/overrides (0079, Tx tally, SoD) + kartu + uji.
- F-D09-014/040 NOT_STARTED->IMPLEMENTED (~73.1 verified). Workspace 410/0 (1 transient api flake unreproduced x3 runs). Registry ~27.8 (206 IMPLEMENTED); mean 29.4 (200 v).

## Done wave 120: approval routing (main)
- approval_routes (0078) + resolver (amount+risk) + kartu + uji.
- F-D09-012/013 PLANNED/NOT_STARTED->IMPLEMENTED (~73.0 verified). Workspace 409/0; registry ~27.8 (204 IMPLEMENTED); mean 29.2 (198 v).

## Done wave 119: conditional + amendment (main)
- decide condition + amend endpoint (no tables) + kartu + uji.
- F-D09-009/026 PLANNED->IMPLEMENTED (~72.5 verified). Workspace 408/0; registry ~27.7 (202 IMPLEMENTED); mean 29.1 (196 v).

## Done wave 118: replay guard + simulation (main)
- idempotency_keys (0077) + simulate endpoint + kartu + uji.
- F-D09-037/034 PLANNED->IMPLEMENTED (~73.0 verified). Workspace 407/0; registry ~27.6 (200 IMPLEMENTED); mean 29.0 (194 v).

## Done wave 117: digest prefs + freeze (main)
- approval_prefs + approval_freeze (0076) + enforcement + kartu + uji.
- F-D09-028/023 PLANNED->IMPLEMENTED (~72.5 verified). Workspace 406/0; registry ~27.5 (198 IMPLEMENTED); mean 28.8 (192 v).

## Done wave 116: attachments + archive (main)
- approval attachments + archive retrieval (no tables) + kartu + uji.
- F-D09-011/031 PLANNED/NOT_STARTED->IMPLEMENTED (~72.1 verified). Workspace 405/0; registry ~27.4 (196 IMPLEMENTED); mean 28.7 (190 v).

## Done wave 115: bulk decide + receipts (main)
- appr_decide_bulk Tx + decidable inbox + kartu + uji; 039 verify-only.
- F-D09-007 NOT_STARTED->IMPLEMENTED, 039 PARTIAL->IMPLEMENTED (~72.9 verified). Workspace 404/0; registry ~27.3 (194 IMPLEMENTED); mean 28.6 (188 v).

## Done wave 114: approval SLA + conflicts (main)
- sla + conflicts derivations (no tables) + kartu + uji.
- F-D09-005/033 PLANNED->IMPLEMENTED (~72.5 verified). Workspace 403/0; registry ~27.2 (192 IMPLEMENTED); mean 28.5 (187 v).

## Done wave 113: D12 close-out (main)
- notary-monthly-pack + mpn-pack + audit actor filter + 009 rescore.
- F-D12-005/006/034 PARTIAL->IMPLEMENTED, 009 PLANNED->IMPLEMENTED (~72.4 verified). Workspace 402/0; registry ~27.1 (190 IMPLEMENTED); mean 28.4 (185 v). D12 CLOSED 40/40.

## Done wave 112: retention schedule + AML (main)
- protocol-retention + aml-scores (no tables, live derivation) + kartu + uji.
- F-D11-013/015 PLANNED/NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 401/0; registry ~26.9 (186 IMPLEMENTED); mean 28.2 (181 v).

## Done wave 111: retention + gated purge (main)
- retention_rules + purge_requests (0075, Tx execute, SoD) + kartu + uji.
- F-D11-011/012 NOT_STARTED->IMPLEMENTED (~73.9 verified). Workspace 400/0; registry ~26.8 (184 IMPLEMENTED); mean 28.1 (179 v).

## Done wave 110: authenticity verdict (main)
- ppat_authenticity 5-aspect verdict (no table) + kartu + uji.
- F-D13-003 PARTIAL->IMPLEMENTED (~73.6 verified). Workspace 399/0; registry ~26.7 (182 IMPLEMENTED); mean 28.0 (177 v). D13 CLOSED 40/40.

## Done wave 109: BPN tracker manual mode (main)
- bpn_tracks (0074, verification|queue|response) + kartu + uji; auto-sync BLOCKED on creds.
- F-D13-004/027/040 CONFIGURATION_REQUIRED->IMPLEMENTED (~72.4 verified, integration 55 = manual). Workspace 398/0; registry ~26.6 (181 IMPLEMENTED); mean 27.9 (177 v).

## Done wave 108: certificate vault links (main)
- cert_links (0073, deed<->scan) + kartu + uji.
- F-D13-039/002 PARTIAL->IMPLEMENTED (~73.2 verified). Workspace 397/0; registry ~26.5 (178 IMPLEMENTED); mean 27.8 (174 v).

## Done wave 107: AJB drafts + PPAT audit trail (main)
- ajb_drafts (0072) + ppat-audit consolidation (no table) + kartu + uji.
- F-D13-005/038 PARTIAL->IMPLEMENTED (~73.2 verified). Workspace 396/0; registry ~26.4 (176 IMPLEMENTED); mean 27.8 (173 v).

## Done wave 106: KPR packs + protocol register (main)
- kpr_packs + protocol_register (0071) + kartu + uji.
- F-D13-025/030 PLANNED->IMPLEMENTED (~72.9 verified). Workspace 395/0; registry ~26.3 (174 IMPLEMENTED); mean 27.8 (172 v).

## Done wave 105: developer batches + unit stock (main)
- dev_batches + dev_units (0070, batch<->unit link) + kartu + uji.
- F-D13-022/023 NOT_STARTED->IMPLEMENTED (~72.9 verified). Workspace 394/0; registry ~26.2 (172 IMPLEMENTED); mean 27.6 (170 v).

## Done wave 104: girik intake + lease terms (main)
- girik_intakes + lease_terms (0069) + kartu + uji.
- F-D13-021/010 NOT_STARTED->IMPLEMENTED (~72.9 verified). Workspace 393/0; registry ~26.1 (170 IMPLEMENTED); mean 27.5 (168 v).

## Done wave 103: foreign guardrail + notary handoff (main)
- fguard matrix (no table) + notary_handoffs (0068) + kartu + uji.
- F-D13-035/026 PLANNED->IMPLEMENTED (~72.9 verified). Workspace 392/0; registry ~26.0 (168 IMPLEMENTED); mean 27.4 (166 v).

## Done wave 102: PPAT monthly report + dashboard (main)
- ppat_monthly + ppat_dashboard aggregates (no migration) + kartu + uji.
- F-D13-029/037 NOT_STARTED/PLANNED->IMPLEMENTED (~72.9 verified). Workspace 391/0; registry ~25.9 (166 IMPLEMENTED); mean 27.2 (164 v).

## Done wave 101: land history + BAST handover (main)
- land_history + unit_handovers (0067) + kartu + uji.
- F-D13-013/024 PLANNED->IMPLEMENTED (~72.8 verified). Workspace 390/0; registry ~25.8 (164 IMPLEMENTED); mean 27.1 (162 v).

## Done wave 100: board + power-to-sell consent checks (main)
- Reuse deed_consents (0066, kind=board|power) + kartu + uji; no new migration.
- F-D13-017/034 PLANNED->IMPLEMENTED (~72.8 verified). Workspace 389/0; registry ~25.7 (162 IMPLEMENTED); mean 27.0 (160 v).

## Done wave 99: marital + heir consent checks (main)
- deed_consents (0066, kind=marital|heir) + verify toggle + kartu + uji.
- F-D13-015/016 NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 388/0; registry ~25.6 (160 IMPLEMENTED); mean 26.8 (158 v).

## Done wave 98: dispute flags + boundary checks (main)
- land_disputes + boundary_checks (0065) + kartu + uji.
- F-D13-032/033 PLANNED/NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 387/0; registry ~25.5 (158 IMPLEMENTED); mean 26.7 (156 v).

## Done wave 97: land split + merge trackers (main)
- land_splits + land_merges (0064) + kartu + uji.
- F-D13-011/012 PLANNED->IMPLEMENTED (~72.8 verified). Workspace 386/0; registry ~25.4 (156 IMPLEMENTED); mean 26.6 (154 v).

## Done wave 96: APHT/SKMHT charges + expiry monitor (main)
- deed_charges (0063, upsert+Tx) + monitor + kartu + uji.
- F-D13-008/009 PLANNED/NOT_STARTED->IMPLEMENTED (~72.9 verified). Workspace 385/0; registry ~25.2 (154 IMPLEMENTED); mean 26.4 (152 v).

## Done wave 95: kind-aware PPAT readiness (main)
- GET /api/v1/ppat/{id}/readiness (hibah/tukar_menukar rules) + kartu + uji (tanpa migrasi).
- F-D13-006/007 NOT_STARTED->IMPLEMENTED (~72.0 verified). Workspace 384/0; registry ~25.1 (152 IMPLEMENTED); mean 26.3 (150 v).

## Done wave 94: PBB checks + PPAT fee (main)
- pbb_checks (0062) + fee worksheet + kartu + uji.
- F-D13-036/020 NOT_STARTED->IMPLEMENTED (~72.9 verified). Workspace 383/0; registry ~25.0 (150 IMPLEMENTED); mean 26.2 (148 v).

## Done wave 93: zone table + PPh/BPHTB worksheets (main)
- zones (0061) + worksheet endpoints + kartu akta + uji (formula jujur, bukan advis).
- F-D13-031/018/019 PLANNED/NOT_STARTED->IMPLEMENTED (~72.9 verified). Workspace 382/0; registry ~24.9 (148 IMPLEMENTED); mean 26.0 (146 v).

## Done wave 92: PPAT KYC bundle + handover receipts (main, D13 opened)
- ppat_parties + cert_handovers (0060) + kartu akta + uji.
- F-D13-014/028 PLANNED->IMPLEMENTED (~72.8 verified). Workspace 381/0; registry ~24.8 (145 IMPLEMENTED); mean 25.8 (143 v).

## Done wave 91: multi-desk dispatch (main, D12 closed)
- task assignee+desk (0059 ALTER) + assign/board + kartu + uji.
- F-D12-033 PLANNED->IMPLEMENTED (~73.1 verified). Workspace 380/0; registry ~24.7 (143 IMPLEMENTED); mean 25.7 (141 v).

## Done wave 90: staff productivity + waiting analytics (main, D12: 1 left)
- GET /api/v1/reports/staff-productivity + waiting-analytics + kartu ringkas + uji (derivasi, tanpa migrasi).
- F-D12-028/029 NOT_STARTED->IMPLEMENTED (~72.0 verified). Workspace 379/0; registry ~24.5 (142 IMPLEMENTED); mean 25.6 (140 v).

## Done wave 89: self-check + year-end seals (main, D12 nearly closed)
- selfcheck_runs + year_end_seals (0058, count snapshot) + kartu + uji.
- F-D12-039/040 NOT_STARTED/PLANNED->IMPLEMENTED (~72.9 verified). Workspace 378/0; registry ~24.4 (140 IMPLEMENTED); mean 25.5 (138 v).

## Done wave 88: SOP library + backup verification (main)
- sop_docs + backup_checks (0057) + kartu dok/jobs + uji.
- F-D12-031/035 NOT_STARTED/PLANNED->IMPLEMENTED (~72.8 verified). Workspace 377/0; registry ~24.2 (138 IMPLEMENTED); mean 25.3 (136 v).

## Done wave 87: bank facilities + feedback (main)
- bank_facilities + feedbacks (0056) + kartu keu/mkt + uji.
- F-D12-025/038 PLANNED/NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 376/0; registry ~24.2 (136 IMPLEMENTED); mean 25.2 (134 v).

## Done wave 86: doc checklist + office incidents (main)
- doc_checklist_items + office_incidents (0055) + kartu dok/ringkas + uji.
- F-D12-030/036 PLANNED/NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 375/0; registry ~24.0 (134 IMPLEMENTED); mean 25.1 (132 v).

## Done wave 85: storage boxes + MPD transfers (main)
- storage_boxes + mpd_transfers (0054) + kartu akta + uji.
- F-D12-021/022 NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 374/0; registry ~23.9 (132 IMPLEMENTED); mean 24.9 (130 v).

## Done wave 84: office performance + deed volume (main)
- GET /api/v1/reports/office-performance + deed-volume + kartu akta + uji (derivasi, tanpa migrasi).
- F-D12-026/027 PLANNED->IMPLEMENTED (~72.0 verified). Workspace 373/0; registry ~23.8 (130 IMPLEMENTED); mean 24.8 (128 v).

## Done wave 83: ATK kind-tag + archive map (main)
- inventory kind tag + archive_boxes (0053 ALTER+CREATE) + kartu + uji.
- F-D12-019/020 NOT_STARTED/PLANNED->IMPLEMENTED (~72.9 verified). Workspace 372/0; registry ~23.7 (128 IMPLEMENTED); mean 24.7 (126 v).

## Done wave 82: office hours + roster + leaves (main)
- office_hours + roster_members + leaves (0052) + kartu klien + uji.
- F-D12-007/008 NOT_STARTED/PLANNED->IMPLEMENTED (~72.8 verified). Workspace 371/0; registry ~23.6 (126 IMPLEMENTED); mean 24.5 (124 v).

## Done wave 81: witness pool + re-verification (main)
- witnesses + rechecks (0051) + kartu klien + uji (re-verifikasi manual, eksternal BLOCKED).
- F-D12-024/023 NOT_STARTED/PLANNED->IMPLEMENTED (~72.9 verified). Workspace 370/0; registry ~23.5 (124 IMPLEMENTED); mean 24.4 (122 v).

## Done wave 80: down-payments + klapper (main)
- down_payments (0050) + klapper derivasi + kartu + uji.
- F-D12-013/002 NOT_STARTED/PLANNED->IMPLEMENTED (~72.9 verified). Workspace 369/0; registry ~23.4 (122 IMPLEMENTED); mean 24.3 (120 v).

## Done wave 79: visitor log + announcements (main)
- visitors + announcements (0049, nomor dipakai ulang setelah 0049 lama ditarik pre-push) + kartu + uji.
- F-D12-037/032 NOT_STARTED/PLANNED->IMPLEMENTED (~72.8 verified). Workspace 368/0; registry ~23.3 (120 IMPLEMENTED); mean 24.1 (118 v).

## Done wave 78: inventory + meterai usage (main)
- inventory_items + meterai_usage (0048, atomic decrement Tx) + kartu keu + uji.
- F-D12-017/018 NOT_STARTED/PLANNED->IMPLEMENTED (~73.4 verified). Workspace 367/0; registry ~23.2 (118 IMPLEMENTED); mean 24.0 (116 v). Recovery: 0048 initially failed live on duplicate FK fk_inv_org (success=0, no tables); fixed local-only + cleared failed row; 48 success=1 live.

## Done wave 77: cash book + advances (main)
- cash_entries + advances (0047, saldo derivasi) + kartu keu + uji.
- F-D12-015/016 NOT_STARTED/PLANNED->IMPLEMENTED (~72.8 verified). Workspace 366/0; registry ~23.1 (116 IMPLEMENTED); mean 23.9 (114 v).

## Done wave 76: price list + quotations (main)
- price_items + quotations/items (0046, snapshot Tx) + kartu keu + uji.
- F-D12-012/014 NOT_STARTED->IMPLEMENTED (~73.4 verified). Workspace 365/0; registry ~23.0 (114 IMPLEMENTED); mean 23.7 (112 v).

## Done wave 75: appointments + queue board (main)
- appointments (0045, slot ketat) + papan antrean derivasi + kartu klien + uji.
- F-D12-010/011 NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 364/0; registry ~22.9 (112 IMPLEMENTED); mean 23.6 (110 v).

## Done wave 74: protocol books + handover log (main, D12 opened)
- protocol_books + protocol_handovers (0044) + kartu akta + uji.
- F-D12-003/004 NOT_STARTED/PLANNED->IMPLEMENTED (~72.8 verified). Workspace 363/0; registry ~22.8 (110 IMPLEMENTED); mean 23.4 (108 v).

## Done wave 73: evidence vault + regulator pack (main, D11 closed)
- compliance_evidence (0043, 19-register referential check) + paket regulator + kartu + uji.
- F-D11-030/031 NOT_STARTED->IMPLEMENTED (~73.0 verified). Workspace 362/0; registry ~22.7 (108 IMPLEMENTED); mean 23.3 (106 v).

## Done wave 72: matter checklist + scorecard (main)
- matter_checklist_items (0042) + scorecard derivasi + kartu + uji.
- F-D11-032/033 PLANNED/NOT_STARTED->IMPLEMENTED (~72.9 verified). Workspace 361/0; registry ~22.6 (106 IMPLEMENTED); mean 23.2 (104 v).

## Done wave 71: control tests + audit plans (main)
- control_tests (0040) + jadwal jatuh tempo + audit_plans (0041) + kartu + uji.
- F-D11-028/025 NOT_STARTED->IMPLEMENTED (~73.0 verified). Workspace 360/0; registry ~22.5 (104 IMPLEMENTED); mean 23.0 (102 v).

## Done wave 70: compliance alert feed (main)
- GET /api/v1/compliance/alerts (6 jenis turunan) + kartu Peringatan + posture/CSV 19 register.
- F-D11-034 NOT_STARTED->IMPLEMENTED (~72.8 verified). Workspace 359/0; registry ~22.4 (102 IMPLEMENTED); mean 22.9 (100 v).

## Done wave 69: policy library + control library (main)
- policies + acks (0038) + controls (0039) + kartu kepatuhan + uji.
- F-D11-035/027 NOT_STARTED->IMPLEMENTED (~73.0 verified). Workspace 358/0; registry ~22.2 (101 IMPLEMENTED); mean 22.8 (99 v).

## Done wave 68: whistleblowing + waivers (main)
- whistle_reports (0036) + exception_waivers (0037) + kartu kepatuhan + uji.
- F-D11-019/036 NOT_STARTED->IMPLEMENTED (~72.9 verified). Workspace 357/0; registry ~22.0 (99 IMPLEMENTED); mean 22.7 (97 v).

## Done wave 67: training evidence + audit findings (main)
- training_evidence (0034) + audit_findings (0035) + kartu kepatuhan + uji.
- F-D11-024/026 NOT_STARTED/PLANNED->IMPLEMENTED (~72.8 verified). Workspace 356/0; registry ~21.8 (97 IMPLEMENTED); mean 22.5 (95 v).

## Done wave 66: compliance dashboard + exporter (main)
- GET /api/v1/compliance/posture + GET /api/v1/reports/compliance.csv + kartu dasbor kepatuhan.
- F-D11-029/040 NOT_STARTED->IMPLEMENTED (~73.2 verified). Workspace 355/0; registry ~21.6 (95 IMPLEMENTED); mean 22.4 (93 v).

## Done wave 65: ethics declarations + gift register (main)
- ethics_declarations (0032) + gift_register (0033) + kartu kepatuhan + uji lifecycle.
- F-D11-017/018 PLANNED/NOT_STARTED->IMPLEMENTED (~72.6 verified). Workspace 354/0; registry ~21.4 (93 IMPLEMENTED); mean ~22.1 (91 v).

## Done wave 64: data-subject rights DSAR (main)
- dsar_requests (migration 0031) melayani akses/penghapusan/perbaikan + kartu kepatuhan + uji.
- F-D11-003/004/005 NOT_STARTED/PLANNED->IMPLEMENTED (~72.8 verified). Workspace 353/0; registry ~21.2 (91 IMPLEMENTED); mean ~22.0 (89 v).

## Done wave 63: risk register + heatmap (main)
- risk_register (migration 0030) + state machine + heatmap 5xKxD + kartu kepatuhan + uji.
- F-D11-037/038 NOT_STARTED/PLANNED->IMPLEMENTED (~73.0 verified). Workspace 352/0; registry ~20.9 (88 IMPLEMENTED); mean ~21.9 (86 v).

## Done wave 62: training assignments (main)
- training_assignments (migration 0029) + assigned->in_progress->completed + kartu kepatuhan + uji lifecycle.
- F-D11-023 NOT_STARTED->IMPLEMENTED (~72.6 verified). Workspace 351/0; registry ~20.7 (86 IMPLEMENTED); mean ~21.8 (84 v).

## Done wave 61: approval calendar view (main)
- GET /api/v1/approvals/calendar?month + kartu kalender di tab Persetujuan (tanpa migrasi).
- F-D09-019 PLANNED->IMPLEMENTED (~71.9 verified). Workspace 350/0; registry ~20.6 (85 IMPLEMENTED); mean ~21.7 (83 v).

## Done wave 60: regulatory change tracker (main)
- regulatory_changes (migration 0028) + open->acknowledged->implemented + kartu kepatuhan + uji lifecycle.
- F-D11-022 PLANNED->IMPLEMENTED (~72.6 verified). Workspace 349/0; registry ~20.5 (84 IMPLEMENTED); mean ~21.6 (82 v).

## Done wave 59: overdue approval alerts (main)
- GET /api/v1/approvals/overdue?days=N + kartu Terlambat di tab Persetujuan (tanpa migrasi, pakai created).
- F-D09-018 PLANNED->IMPLEMENTED (~72.4 verified). Workspace 348/0; registry ~20.4 (83 IMPLEMENTED); mean ~21.6 (81 v).

## Done wave 58: register insiden + CAPA tracker (main)
- incident_logs + capa_items (migrations 0026/0027) + state machines + kartu kepatuhan + uji lifecycle.
- F-D11-020/F-D11-021 PLANNED->IMPLEMENTED (~72.5 verified). Workspace 347/0; registry 19.9->~20.3 (82 IMPLEMENTED); mean ~21.5 (80 v).

## Done wave 57: monitor kesehatan integrasi (main)
- /api/v1/integrations/health + kartu Jobs (tanpa bocor secret) + uji no-leak.
- F-D21-031 51.8->70.9 PARTIAL (verified). Workspace 346/0; registry 19.9 (80 IMPLEMENTED); mean 21.4 (78 v).

## Done wave 56: register->akta prefill + verifikasi delivery/read (main)
- PPAT "Buat akta" prefill nomor/jenis; F-D13-005 59.4->62.2 PARTIAL.
- F-D20-028 NOT_STARTED->PARTIAL (59.4); F-D09-039 PLANNED->PARTIAL (60.6).
- Workspace 345/0; registry 19.9 (80 IMPLEMENTED); 8-cat mean 21.4 (77 v).

## Done wave 55: ekspor audit approval (main)
- approvals.csv + tombol + trail; F-D09-035 NOT_STARTED->PARTIAL (68.4).
- Workspace 345/0; registry 19.9 (80 IMPLEMENTED); 8-cat mean 21.3 (75 v).

## Done wave 54: daftar pelanggaran SLA (main)
- sla-breaches.csv (hanya overdue terbuka) + tombol + trail; F-D03-039 PLANNED->PARTIAL (66.2).
- Workspace 344/0; registry 19.9 (79 IMPLEMENTED); 8-cat mean 21.2 (74 v).

## Done wave 51: ringkasan eksekutif (main)
- Snapshot 9 metrik lintas modul + kartu ringkas; F-D20-039 PLANNED->PARTIAL (68.2).
- Workspace 344/0; registry 19.9 (79 IMPLEMENTED); 8-cat mean 20.9 (69 v).

## Done wave 50: SoD conflict check (main)
- GET /api/v1/roles/sod + kartu Cek SoD (3 pasangan, advisory); F-D14-038 67.9->70.5 PARTIAL.
- Workspace 343/0; registry 19.9 (77 IMPLEMENTED); 8-cat mean 20.9 (68 v).

## Done wave 47: register vendor + DPA (main)
- CRUD + DPA workflow + kartu kepatuhan + audit; F-D11-009/010 -> IMPLEMENTED (75.6/73.1).
- /skor page removed from user UI per owner request (API + registry retained as evidence).
- Workspace 339/0; registry 20.1 (79 IMPLEMENTED); 8-cat mean 20.8 (66 v).

## Done wave 45: undangan anggota + delegasi e2e (main)
- Migration 0025; invite 72 jam; register-with-token; decide e2e via delegasi (member lawyer).
- F-D09-003/004 PARTIAL->IMPLEMENTED (78.5). Workspace 337/0; registry 19.9; mean 20.7 (66 v).

## Done wave 44b: hapus halaman /skor publik (main)
- /skor + link sidebar + JS dihapus dari tampilan pengguna (khusus legal); API katalog + data audit tetap sebagai backbone bukti.

## Done wave 44: kepatuhan tenggat + postur kepatuhan (main)
- deadline-compliance.csv + compliance-posture.csv + tombol + trail.
- F-D20-013 PLANNED->PARTIAL (68.8); F-D20-017 PLANNED->PARTIAL (68.5).
- Workspace 337/0; registry 19.9; mean 20.6 (64 v).

## Done wave 43: laporan bottleneck + profitabilitas (main)
- approvals-bottleneck.csv (stuck step/role) + profitability.csv (per-client) + audit.
- F-D20-012/014 PLANNED->PARTIAL (67.9/67.2). Workspace 336/0; registry 19.9; mean 20.5 (62 v).

## Done wave 41+42: perbandingan periode + tenggat di kalender (main)
- deeds.csv ?compare=1 summary (delta math + guards) + tombol Bandingkan.
- Task due API + kolom Tenggat + setter per baris + kalender bertitik + upcoming.
- F-D20-024 PLANNED->PARTIAL (67.2); F-D03-010 NOT_STARTED->PARTIAL (64.8).
- Workspace 335/0; registry 19.9 (75 IMPLEMENTED); 8-cat mean 20.4 (60 v).

## Done wave 40: delegasi approval pengganti (main)
- Migration 0023; decide honors active delegation + audit; guard/UI tests.
- F-D09-003 NOT_STARTED->PARTIAL (64.4); F-D09-004 PLANNED->PARTIAL (63.1). Caveat jujur: decide e2e butuh invite flow (F-D16-001 follow-up).
- Workspace 334/0; registry 19.9 (75 IMPLEMENTED); 8-cat mean 20.3 (58 verified).

## Done wave 39: audit kelengkapan navigasi (main)
- 26 halaman publik + /app + /skor wajib 200 (public_pages_all_render_200); sidebar 12 tab 1:1 + mobile nav + footer + sitemap terverifikasi.
- Semua menu penting tampil. Workspace 332/0; registry 20.0 (75 IMPLEMENTED); 8-cat mean 20.2 (56 verified).

## Done wave 38: pustaka klausa berversi (main)
- Migration 0022; clause CRUD + version bump + kartu akta + revisi/hapus.
- F-D06-002 NOT_STARTED->IMPLEMENTED (76.9). Workspace 331/0; registry 20.0 (75 IMPLEMENTED); mean 20.2 (56 v).

## Done wave 37: DPIA + breach log 72 jam (main) [dikoreksi ke F-D11-007/008]
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
