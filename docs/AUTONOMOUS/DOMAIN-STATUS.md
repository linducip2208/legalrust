# DOMAIN STATUS (2026-11-18, computed from FEATURE-REGISTRY.json, n=1000)

Live counts per domain. Gates status: acceptance 28/32 PARTIAL items
(OCR engine live, AI provider live, final self-audit only) gate D05/D07/D10/D22;
MISSING prod deploy + counsel sign-off gates all domains for production.

| Domain | Name | IMPL | PART | LIVE(IMPL+PART) | CONF_REQ | PLAN | NOT_START | Gates status |
|---|---|---|---|---|---|---|---|---|
| D01 | Identity & KYC | 0 | 6 | 6 | 9 | 14 | 11 | PARTIAL ok; 9 blocked (OCR/engine ext) |
| D02 | Client & CRM | 4 | 2 | 6 | 1 | 11 | 22 | LIVE base ok; 33 no-evidence |
| D03 | Project & Case Mgmt | 4 | 5 | 9 | 1 | 14 | 16 | LIVE base ok; 30 no-evidence |
| D04 | Document Mgmt | 7 | 7 | 14 | 2 | 9 | 15 | Strongest; 24 no-evidence |
| D05 | OCR & Doc Intel | 0 | 6 | 6 | 25 | 4 | 5 | BLOCKED: engine creds (B-OCR) |
| D06 | Deed Generation | 2 | 10 | 12 | 2 | 11 | 15 | LIVE base ok; 26 no-evidence |
| D07 | AI Legal Assistant | 1 | 1 | 2 | 28 | 4 | 6 | BLOCKED: AI provider (B-AI) |
| D08 | Workflow Automation | 0 | 3 | 3 | 4 | 14 | 19 | 33 no-evidence |
| D09 | Approval Mgmt | 0 | 0 | 0 | 1 | 24 | 15 | 39 no-evidence; earliest domain |
| D10 | Electronic Signature | 0 | 2 | 2 | 27 | 3 | 8 | BLOCKED: PSrE creds (B-ESIGN) |
| D11 | Compliance & Risk | 0 | 1 | 1 | 3 | 13 | 23 | 36 no-evidence |
| D12 | Notary Office Mgmt | 0 | 3 | 3 | 0 | 19 | 18 | 37 no-evidence; no ext block |
| D13 | PPAT & Property | 0 | 3 | 3 | 3 | 17 | 17 | 34 no-evidence |
| D14 | Finance & Accounting | 2 | 2 | 4 | 2 | 18 | 16 | LIVE base ok; gateway items blocked (B-PAY) |
| D15 | Billing & Subscription | 2 | 3 | 5 | 3 | 17 | 15 | 32 no-evidence |
| D16 | User, Role & Perm | 2 | 7 | 9 | 4 | 10 | 17 | Foundation LIVE; gate PASS |
| D17 | Multi-Office & Tenant | 0 | 4 | 4 | 0 | 22 | 14 | 36 no-evidence; no ext block |
| D18 | Comm & Notification | 2 | 4 | 6 | 7 | 12 | 15 | 7 blocked (WA/SMS/email ext) |
| D19 | Client & Public Portal | 4 | 5 | 9 | 3 | 13 | 15 | Portal LIVE; 28 no-evidence |
| D20 | Reporting & Analytics | 0 | 4 | 4 | 1 | 16 | 19 | CSV-only; 35 no-evidence |
| D21 | API & Integration | 1 | 8 | 9 | 17 | 8 | 6 | OpenAPI live; 17 ext-stub blocked |
| D22 | Security & Data Prot | 6 | 4 | 10 | 3 | 12 | 15 | Strongest impl; self-audit only (PARTIAL) |
| D23 | Search & Discovery | 0 | 9 | 9 | 8 | 7 | 16 | Keyword baseline live; no vector/tantivy |
| D24 | Mobile & Field Ops | 0 | 0 | 0 | 2 | 22 | 16 | 38 no-evidence; earliest domain |
| D25 | Enterprise Admin | 0 | 5 | 5 | 3 | 12 | 20 | 32 no-evidence |
| **Total** | 25 domains | **37** | **104** | **141** | **159** | **326** | **374** | 28/32 acceptance; prod + counsel MISSING |

Legend: IMPL=IMPLEMENTED, PART=PARTIAL, CONF_REQ=CONFIGURATION_REQUIRED,
PLAN=PLANNED, NOT_START=NOT_STARTED. Live = IMPL+PART (evidence in FEATURE-EVIDENCE.json).
