# AGENT-RESULTS (AUDIT-1000 wave 2026-11-18 + wave 2)

## W1-backend-inventory (explore)
- 16 lib crates + 4 apps inventoried with purposes.
- 110 `.route()` registrations: 29 pages + 81 API (100 method handlers).
- Trait surface: OpsStore 42 → **47 methods** (added appr_* ×5), MarketingStore 23,
  CrmStore 10, User+Core 30. Migrations 12 → **13** (0013 approvals,
  2 tables, 46 total). Workspace tests **288 green**.

## W1-gap-scan (explore)
- Zero TODO/FIXME/PLACEHOLDER/UNIMPLEMENTED/todo!/unimplemented! in
  non-test source. Zero production mocks/fakes (only comments, CSS classes,
  HTML placeholders, landing honest-copy, test-only stubs).
- Honest-501 inventory: 6 direct `BlockedExternal` sites in server
  (readyz×2, esign, ai_chat-fallback, tripay webhook/callback, webhook
  secret) + indirect via `From` (ai, pay-gateway, tripay, ocr, cloud).
- Action taken: manifest `icons: []` placeholder → **fixed** with real
  192/512px PNGs generated from brand art via PIL (center-crop + LANCZOS).

## W2-partial-verify (explore)
- F-D08-012/015/019: VERIFIED-PARTIAL (inbound webhooks, in-app notify,
  payment collection all live as endpoints; no workflow-node binding).
- F-D20-001: OVERCLAIMED (single-CSV reality) — kept PARTIAL, score low.
- F-D20-004: VERIFIED-PARTIAL (CSV only). F-D20-020: VERIFIED-PARTIAL
  (per-endpoint RBAC). F-D20-034: was OVERCLAIMED → **FIXED this wave**
  (audit emitted on all 3 CSV exports) → IMPLEMENTED.
- F-D25-009/016/019/036: VERIFIED-PARTIAL. F-D25-038: OVERCLAIMED
  (healthz/readyz only, no SLO board) — kept PARTIAL score 20 with note.
- F-D08-020/021: endpoints IMPLEMENTED, connectors missing — PARTIAL noted.
- F-D09-029: stub-only 501, verified.

## W3-implement (main executor)
- Migration `0013_approvals.sql` (forward-only, no destructive ops).
- `OpsStore::appr_*` ×5 (memory single-lock + MySQL single-tx).
- 5 approval routes + RBAC (Create/View/Approve) + step role-match
  (admin/owner bypass, mismatch → 403).
- CSV audit events (`report.matters_exported`,
  `report.matters_csv_exported`, `report.clients_csv_exported`).
- 3 approvals integration tests; registry: 10 features updated
  (F-D09 ×9 → PARTIAL w/ backend-complete notes, F-D20-034 → IMPLEMENTED).
- Wave 2: approvals /app UI (panel+TABS+JS+needles), trust-ledger CSV
  route + UI button, job live counts; server API 107/107, workspace
  290/290, fmt+clippy clean; registry +13 → IMPLEMENTED
  (F-D09 ×8, F-D20-001/004/020, F-D25-009).
