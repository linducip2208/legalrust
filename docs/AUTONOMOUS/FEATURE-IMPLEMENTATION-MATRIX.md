# Feature Implementation Matrix (1000 features, 25 domains)

Source: `docs/03-FEATURE-CATALOG/feature-catalog.json` (1000 rows) mapped against
`apps/server/src/lib.rs` routes/handlers, `crates/database/src/ops*.rs` + entities,
`migrations/*.sql` (10 files), workspace `Cargo.toml`. Read-only static analysis;
no code or migrations touched. For verification builds use `$env:CARGO_TARGET_DIR='target-reg'`;
no compilation was needed for this static-evidence pass.

## Status definitions
- **IMPLEMENTED**: live 200-path route + store op + table verified (non-501).
- **PARTIAL**: usable subset live (e.g. intake/queue/list/audit), remainder missing or 501.
- **CONFIGURATION_REQUIRED**: delivery path exists (queue/route/stub) but needs an external provider: OCR engine, AI provider, PSrE e-sign, payment gateway, or other named integration (Dukcapil/BPN/WhatsApp/SMS/e-mail/S3/SSO). 501 + docs per AGENTS rule 5.
- **PLANNED**: acknowledged need (P0/P1), no implementation evidence found.
- **NOT_STARTED**: no evidence found (default for P2/P3 without evidence).
- Rule is conservative: default PLANNED/NOT_STARTED unless evidence found.

## Method
- Parsed the 1000-row catalog with Python; sampled 2-3 D*-DOMAIN.md files for schema context (domain/priority/complexity/effort fields).
- Curated ~130 title anchors (IMPLEMENTED/PARTIAL) and ~120 external-dependency patterns (CONFIGURATION_REQUIRED) against the verified evidence inventory (routes, ops traits, migration tables, four known 501s: `ocr_process` lib.rs:747, `ai_chat` lib.rs:995, `deed_esign` lib.rs:970, `pay_gateway` lib.rs:1268).
- Automated verification in generator: every IMPLEMENTED/PARTIAL/CONFIG anchor requires its code token to exist (loud fail otherwise); 19 absent-capability tokens asserted missing (`kanban,gantt,timesheet,liveness,webauthn,passkey,totp,scim,nats,tantivy,qdrant,minio,openapi,biometric,fingerprint,oauth,...`); sampled rows with unexpected code hits auto-upgraded (max PARTIAL) and flagged.
- Sample: 96 features (3-4 per domain, spanning statuses present) verified with file:line evidence; remaining 904 rule-classified. See `verify_report.txt` (temp) for the per-sample log.

## Status totals (n=1000)
| Status | Count | % |
|---|---|---|
| IMPLEMENTED | 37 | 3.7% |
| PARTIAL | 104 | 10.4% |
| CONFIGURATION_REQUIRED | 159 | 15.9% |
| PLANNED | 326 | 32.6% |
| NOT_STARTED | 374 | 37.4% |

## Per-domain counts (each domain n=40)
| Domain | Name | IMPLEMENTED | PARTIAL | CONFIG_REQ | PLANNED | NOT_STARTED |
|---|---|---|---|---|---|---|
| D01 | Identity & KYC | 0 | 6 | 9 | 14 | 11 |
| D02 | Client & CRM | 4 | 2 | 1 | 11 | 22 |
| D03 | Project & Case Management | 4 | 5 | 1 | 14 | 16 |
| D04 | Document Management | 7 | 7 | 2 | 9 | 15 |
| D05 | OCR & Document Intelligence | 0 | 6 | 25 | 4 | 5 |
| D06 | Deed Generation | 2 | 10 | 2 | 11 | 15 |
| D07 | AI Legal Assistant | 1 | 1 | 28 | 4 | 6 |
| D08 | Workflow Automation | 0 | 3 | 4 | 14 | 19 |
| D09 | Approval Management | 0 | 0 | 1 | 24 | 15 |
| D10 | Electronic Signature | 0 | 2 | 27 | 3 | 8 |
| D11 | Compliance & Risk | 0 | 1 | 3 | 13 | 23 |
| D12 | Notary Office Management | 0 | 3 | 0 | 19 | 18 |
| D13 | PPAT & Property Legal | 0 | 3 | 3 | 17 | 17 |
| D14 | Finance & Accounting | 2 | 2 | 2 | 18 | 16 |
| D15 | Billing & Subscription | 2 | 3 | 3 | 17 | 15 |
| D16 | User, Role & Permission | 2 | 7 | 4 | 10 | 17 |
| D17 | Multi-Office & Tenant | 0 | 4 | 0 | 22 | 14 |
| D18 | Communication & Notification | 2 | 4 | 7 | 12 | 15 |
| D19 | Client & Public Portal | 4 | 5 | 3 | 13 | 15 |
| D20 | Reporting & Analytics | 0 | 4 | 1 | 16 | 19 |
| D21 | API & Integration | 1 | 8 | 17 | 8 | 6 |
| D22 | Security & Data Protection | 6 | 4 | 3 | 12 | 15 |
| D23 | Search & Discovery | 0 | 9 | 8 | 7 | 16 |
| D24 | Mobile & Field Operations | 0 | 0 | 2 | 22 | 16 |
| D25 | Enterprise Administration & Global Expansion | 0 | 5 | 3 | 12 | 20 |

## Top implemented groups (IMPLEMENTED + PARTIAL)
| Domain | Name | Live or partial |
|---|---|---|
| D04 | Document Management | 14 / 40 |
| D06 | Deed Generation | 12 / 40 |
| D22 | Security & Data Protection | 10 / 40 |
| D03 | Project & Case Management | 9 / 40 |
| D16 | User, Role & Permission | 9 / 40 |
| D19 | Client & Public Portal | 9 / 40 |
| D21 | API & Integration | 9 / 40 |
| D23 | Search & Discovery | 9 / 40 |
| D01 | Identity & KYC | 6 / 40 |
| D02 | Client & CRM | 6 / 40 |
| D05 | OCR & Document Intelligence | 6 / 40 |
| D18 | Communication & Notification | 6 / 40 |
| D15 | Billing & Subscription | 5 / 40 |
| D25 | Enterprise Administration & Global Expansion | 5 / 40 |
| D14 | Finance & Accounting | 4 / 40 |
| D17 | Multi-Office & Tenant | 4 / 40 |
| D20 | Reporting & Analytics | 4 / 40 |
| D08 | Workflow Automation | 3 / 40 |
| D12 | Notary Office Management | 3 / 40 |
| D13 | PPAT & Property Legal | 3 / 40 |
| D07 | AI Legal Assistant | 2 / 40 |
| D10 | Electronic Signature | 2 / 40 |
| D11 | Compliance & Risk | 1 / 40 |

## Most provider-gated domains (CONFIGURATION_REQUIRED)
| Domain | Name | Gated |
|---|---|---|
| D07 | AI Legal Assistant | 28 / 40 |
| D10 | Electronic Signature | 27 / 40 |
| D05 | OCR & Document Intelligence | 25 / 40 |
| D21 | API & Integration | 17 / 40 |
| D01 | Identity & KYC | 9 / 40 |
| D23 | Search & Discovery | 8 / 40 |
| D18 | Communication & Notification | 7 / 40 |
| D08 | Workflow Automation | 4 / 40 |
| D16 | User, Role & Permission | 4 / 40 |
| D11 | Compliance & Risk | 3 / 40 |

## Notes / limitations
- Area-level evidence: a row cites the verified capability area (route/handler/table), not a 1:1 per-feature test. Statuses for non-sampled rows rest on curated title-to-area mapping.
- UI beyond landing/marketing pages and the `/app` shell was not individually verified; API-live rows do not imply a dedicated screen.
- Four 501s bound CONFIGURATION_REQUIRED for their families; other externals (Dukcapil, BPN, sanctions, WhatsApp/SMS, S3, SSO) map to CONFIGURATION_REQUIRED via the named-provider stub (lib.rs:101) or documented absence + external dependency.
