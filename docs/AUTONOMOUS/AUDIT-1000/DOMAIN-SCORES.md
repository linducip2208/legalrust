# DOMAIN-SCORES (AUDIT-1000 wave 2026-11-18)

Source: `FEATURE-SCORES.json` (1000 features). Mean overall = **18.2/100**.

## Rubric (explicit — no fake precision)

Category weights: backend .20, database .10, frontend .15, business_logic .15,
security .15, testing .15, integration .05, documentation .05.

- `IMPLEMENTED` default: 95/90/65/90/85/88/80/82 → ≈86. Applied to 38 features
  whose evidence (route + store + tests) was verified against code by the
  backend-inventory sweep. Frontend defaults to 65 (API-verified; UI varies).
- `PARTIAL` default: 60/60/25/55/60/55/40/50 → ≈52.
- `PLANNED`: 10 flat. `NOT_STARTED`: 0. `CONFIGURATION_REQUIRED`: adapter-level
  45/30/15/40/40/45/20/40 → ≈36 (wiring exists, activation needs creds/TLS).
- 10 features carry full 8-category verified scores (`verified_8cat: true`):
  F-D09-001/008/010/015/016/017/024/025/032 (approvals engine, implemented +
  tested 2026-11-18, no UI yet → frontend 20) and F-D20-034 (report audit
  trail, now emitted on all CSV exports).

Individual deep audits continue per `RESUME.md` — default scores are
placeholders pending per-feature code reads, NOT final grades.

## Per-domain means

| Domain | Mean | Note |
|---|---|---|
| D01 Identity & KYC | 19.4 | OCR live dev-host; Dukcapil/BPN blocked |
| D02 Client & CRM | 14.8 | master records implemented+tested |
| D03 Project & Case | 19.4 | matters/tasks/audit history live |
| D04 Document Mgmt | 28.0 | strongest: DMS+shares+SHA live |
| D05 OCR & Doc Intel | 31.3 | tesseract live, batch/PDF pending |
| D06 Deed Generation | 21.8 | templates+transitions live, esign 501 |
| D07 AI Legal | 29.6 | Ollama live, embeddings pending |
| D08 Workflow | 11.0 | jobs queue + triggers only |
| D09 Approvals | 22.1 | generic engine added this wave (API) |
| D10 E-signature | 27.6 | PSrE adapter, provider blocked |
| D11 Compliance | 7.2 | almost entirely planned |
| D12 Notary Office | 8.6 | almost entirely planned |
| D13 PPAT | 10.8 | almost entirely planned |
| D14 Finance | 13.2 | invoices/trust live, quotes pending |
| D15 Billing | 15.1 | manual+Tripay live, plans pending |
| D16 Roles | 19.4 | RBAC catalog live |
| D17 Multi-office | 10.7 | tenant model only |
| D18 Notify | 18.7 | in-app+SSE live, email/SMS pending |
| D19 Portal | 21.0 | /app + marketing site live |
| D20 Reporting | 10.8 | CSV live + audit now; builder pending |
| D21 API/Integration | 29.8 | 110 routes, keys, webhooks live |
| D22 Security | 23.7 | Argon2/RBAC/CORS/rate-limit, no pentest |
| D23 Search | 20.6 | global+RAG keyword live, vectors pending |
| D24 Mobile | 7.3 | outbox core only, offline UI pending |
| D25 Enterprise Admin | 12.2 | jobs/audit viewers, console pending |

## Quality-gate mapping (Phase 20, honest)

Product completeness ≈ 25 (narrow), Backend ≈ 55, Database ≈ 70, Frontend ≈ 30,
Security ≈ 60 (no critical issues found in self-audit; pentest outstanding),
Testing ≈ 70 (288 green, coverage of new code, gaps in unstarted domains),
Integrations ≈ 35, Performance ≈ 50 (dev benchmarks only), Documentation ≈ 65,
Deployment ≈ 40 (manifests ready, no prod run). **Composite ≈ 44/100 —
NOT 100. Production readiness remains blocked (prod deploy + counsel).**
