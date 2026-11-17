# Master PRD (planning)
## Objectives
O1 Single matter truth (documents, tasks, deadlines, money, comms). O2 Deed-grade accuracy
(OCR + human review + protocol discipline). O3 Governed automation (workflows/approvals/e-sign
within regulatory guardrails). O4 Auditability (append-only evidence everywhere).
## Scope by domain
See 03-FEATURE-CATALOG (D01-D25). Regulatory guardrails: authentic deeds require physical
presence + wet process until law harmonized; e-sign excluded for akta notariil/PPAT per
Pasal 5(4) UU ITE (compliance matrix 15-COMPLIANCE). Notary is UU PDP controller+processor:
consent, ROPA, DPIA, breach notification, retention vs protocol 25-year duties.
## Cross-cutting requirements
Tenant/office scoping on all reads/writes; RBAC+ABAC; idempotent mutations; NATS domain events;
OTel traces; PII masking in logs; S3/MinIO integrity hashes; MySQL FK integrity; WCAG 2.2 AA.
## Acceptance (program level)
Quality gates in 19-AGENT-ORCHESTRATION/QUALITY-GATES.md must pass; validation report green;
no 100% readiness claim without evidence.
