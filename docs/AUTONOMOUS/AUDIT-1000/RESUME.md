# RESUME (AUDIT-1000 — next session entry point)

Date: 2026-11-18 (wave 2 done). HEAD: (record on commit — wave ends with commit).
Workspace: 290 passed / 0 failed; fmt+clippy clean; node --check site.js clean.
Registry: IMPLEMENTED 51 / PARTIAL 99 / PLANNED 319 / NOT_STARTED 372 / CONFIG 159.
Scores: registry 16.9/100; 8-cat mean 18.4 (14 verified). Composite ≈ 45/100 — NOT 100.

## Done wave 2
- Approvals /app tab UI: p-appr panel (compose/list/detail/decide/revoke),
  Persetujuan top-tab + Alur Kerja sidenav rewired, TABS/TABLBL, JS wired,
  node --check + needle test green. F-D09 ×8 → IMPLEMENTED.
- Report center: trust-ledger CSV route (Export-gated + audit) + /app button;
  F-D20-001/004/020 → IMPLEMENTED. Jobs queue live counts; F-D25-009 → IMPLEMENTED.
- Server API 107/107; workspace 290/290.

## Next executable tasks (priority order)
1. **Outbound webhook delivery log** (store attempts on webhook_in + list endpoint + /app panel) → F-D08-013 from NOT_STARTED, F-D25-017 started.
2. **Individual 8-cat audits**: D20/D25/D09 touched items verified; next D08 connectors, D11 ROPA/consent slice, D12 repertorium; set `verified_8cat` per feature.
3. **D12 notary slice**: repertorium (deed daily log view + CSV) — small, real value.
4. **Mobile D24**: offline capture queue UI on top of existing outbox (large; split).
5. **External blockers** (do not fake): PSrE creds, prod deploy + benchmarks + prod-DB run, counsel sign-off, external pentest, Dukcapil/BPN creds.
6. Performance benchmarks on dev machine for new endpoints (approvals p95).

## Constraints to preserve
- One workstream per file set; update FILE-OWNERSHIP.md before editing.
- Forward-only migrations; RBAC server-side; transactions for multi-step
  mutations; no secrets in code/logs; fmt/clippy/test gates per AGENTS.md.
- No push without explicit user authorization (Phase 21).
- Never inflate scores: default-scored features must be individually
  re-audited before any IMPLEMENTED claim.
