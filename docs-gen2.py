#!/usr/bin/env python3
"""LEGAL OS phase-2 docs: architecture, engines, governance. Planning only."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust\docs")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8")

w("README.md", """
# LEGAL OS — Documentation Index (Planning Phase)

> PLANNING ONLY. No application source code has been implemented.
> Rust 100% rule applies to future application code, not to external infra/models/OS.

| # | Folder | Document |
|---|--------|----------|
| 00 | 00-EXECUTIVE | Vision, scope, decisions, glossary |
| 01 | 01-PRODUCT-VISION | Product vision, personas, positioning |
| 02 | 02-MASTER-PRD | Master PRD + functional scope |
| 03 | 03-FEATURE-CATALOG | 25 domains x 40 features = 1,000 specs + 10,000 expansions + JSON/CSV |
| 04 | 04-USER-STORIES | Epic/story map + acceptance map |
| 05 | 05-BUSINESS-RULES | Business rules register BR-001.. |
| 06 | 06-SYSTEM-ARCHITECTURE | Modular monolith, DDD, Mermaid diagrams |
| 07 | 07-DATABASE-ARCHITECTURE | MySQL 8.4 logical design, DDL examples, index strategy |
| 08 | 08-API-ARCHITECTURE | REST conventions, auth, idempotency, events |
| 09 | 09-OCR-ENGINE | OCR PRD, engine benchmark, CER/WER methodology |
| 10 | 10-AI-ENGINE | Provider abstraction, RAG, safety, audit |
| 11 | 11-DOCUMENT-ENGINE | Lifecycle, versioning, retention, storage |
| 12 | 12-LEGAL-WORKFLOWS | Notary/PPAT/litigation/corporate workflows |
| 13 | 13-UI-UX | Design system, screens, journeys, tokens |
| 14 | 14-SECURITY | Auth, crypto, OWASP ASVS mapping, IR |
| 15 | 15-COMPLIANCE | UUJN/UU PDP/UU ITE/PPAT matrix + gaps |
| 16 | 16-TESTING-QA | Test strategy + traceability |
| 17 | 17-DEVOPS | CI/CD, envs, observability, backup |
| 18 | 18-ROADMAP | Phases, milestones, critical path |
| 19 | 19-AGENT-ORCHESTRATION | Agent registry, task queue, quality gates |
| 20 | 20-PROGRESS | Progress tracker + doc completeness |
| 21 | 21-OPERATIONS | Runbooks, SLOs, support |
| 22 | 22-RISK-REGISTER | Risks with mitigation |
| 23 | 23-RESEARCH | Crate research (verified 2026-11), OCR/AI/mobile notes |

Machine-readable catalog: `03-FEATURE-CATALOG/feature-catalog.json`, `feature-catalog.csv`,
`expansions/expansion-catalog.json`, `expansions/expansion-catalog.csv`.
Validation: see `20-PROGRESS/VALIDATION-REPORT.md`.
""")

w("00-EXECUTIVE/EXECUTIVE-SUMMARY.md", """
# Executive Summary — LEGAL OS

## Vision
Enterprise Legal Operations & Intelligence Platform for Indonesia first, architected for
international expansion, with a 10-20 year lifecycle.

## Scope (planning phase)
Documentation and architecture only. Explicitly out of scope: production code,
migrations, deployments, vendor procurement.

## Key decisions (with rationale)
1. **Modular monolith first (Rust/Axum)** — single deployable, DDD bounded contexts,
   event-driven internally via NATS; extract services only on measured need.
2. **100% Rust application code** — backend Axum, frontend Leptos WASM SSR;
   external systems (MySQL, Redis, NATS, S3/MinIO, AI models) are not Rust and are out of scope of the rule.
3. **MySQL 8.4 LTS + InnoDB** — relational integrity first; no polymorphic associations;
   Tantivy for full-text, optional Qdrant for vectors.
4. **Provider-independent AI/OCR** — trait abstractions; OpenAI/Anthropic/local models
   behind router; OCR engines benchmarked (CER/WER/field accuracy), human-in-the-loop mandatory.
5. **AI never certifies legal validity** — AI drafts/flags only; notary/lawyer approves.
   Electronic signature uses certified PSrE for qualifying acts; notarial authentic deeds
   keep physical-presence guardrail until UUJN/UU ITE harmonization (see compliance gaps).

## Production baselines (verified 2026-11-17, stable only, no RC)
- axum **0.8.9** (MSRV 1.80) | tower-http **0.7** | tokio **1.54.x** | sqlx **0.9**
  (MSRV 1.94; pin `=0.8.6` only if toolchain frozen) | serde **1.x**
- leptos stable **0.8.23** (0.9.x is RC — NOT baseline) | MySQL **8.4 LTS** | Rust stable (current).
- Re-verify versions at implementation kickoff; record in ADR.

## Risks (top)
Regulatory disharmony (UUJN physical presence vs UU ITE e-docs; Pasal 5(4) UU ITE exclusion
for notarial deeds) caps e-sign/cyber-notary scope. UU PDP full enforcement since Oct 2024
makes notary-as-controller obligations (ROPA/DPIA/breach 72h) mandatory. OCR/AI accuracy on
Indonesian documents requires measured benchmarks before SLA claims.

## Readiness
Planning foundation complete; implementation readiness gated (see 20-PROGRESS). No readiness
claimed without evidence.
""")

w("00-EXECUTIVE/DECISIONS.md", """
# Architecture Decision Records (index)
- ADR-001 Modular monolith first; service extraction on SLO/coupling triggers.
- ADR-002 Axum + Tokio + Tower + SQLx + Serde backend baseline (versions above).
- ADR-003 Leptos 0.8.x stable baseline; SSR + WASM; no 0.9 RC in production path.
- ADR-004 MySQL 8.4 LTS, InnoDB, strict FK, no polymorphic associations.
- ADR-005 NATS (events) + Redis (cache/locks) + S3/MinIO (objects) + Tantivy (search), Qdrant optional.
- ADR-006 Dioxus Mobile evaluated, documented; no Dart; native shell if webview insufficient.
- ADR-007 OCR/AI behind traits; human review mandatory; AI never certifies validity.
- ADR-008 Tenant/office scoping in every query; legal-hold overrides deletion.
Full log lives in 06-SYSTEM-ARCHITECTURE/ADR.md.
""")

w("00-EXECUTIVE/GLOSSARY.md", """
# Glossary (ID/EN)
Akta/Deed, Minuta (original), Salinan (copy), Kutipan (excerpt), Repertorium (daily deed log),
Klapper (alphabetical index), Protokol (notary protocol archive), Penghadap (appearing party),
Saksi (witness), Renvoi (amendment initialling), PPAT, AJB, APHT, SKMHT, BPN, SHM/SHGB,
PSrE (certified e-sign provider), UUJN, UU PDP (27/2022), UU ITE (1/2024), PSE/PP PSTE 71/2019,
CDD/EDD, ROPA, DPIA, CER/WER, RAG, LTV (long-term validation), TSA.
""")

w("01-PRODUCT-VISION/VISION.md", """
# Product Vision
Indonesia-first legal operations platform for notary offices, PPAT, law firms, corporate legal,
developers, and enterprise legal ops. 25 domains / 1,000 specified features / 10,000 expansions.
Design horizon 10-20 years: modular monolith, locale packs, multi-region readiness, locale-first
compliance (UUJN, UU PDP, UU ITE, PPAT regs) with internationalization seams.

Personas: Notaris, PPAT, Partner/Associate, Paralegal, Corporate counsel, Developer ops,
Front-desk/KYC officer, Finance/billing, Auditor/regulator (read-only packs), Client (portal).
Non-goals (planning): no implementation, no vendor selection, no data migration.
""")

w("02-MASTER-PRD/MASTER-PRD.md", """
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
""")

w("04-USER-STORIES/STORY-MAP.md", """
# Story Map (epics -> representative stories)
E1 Onboard client (D01/D02/D19): register -> KYC capture -> verify -> approve -> portal invite.
E2 Open matter (D03): create -> staff -> budget -> plan tasks -> client engagement letter.
E3 Build deed (D04/D05/D06): ingest docs -> OCR+review -> draft from template -> approve -> ceremony -> protocol.
E4 Property transfer (D13): certificate intake -> BPN check -> AJB/APHT -> tax worksheet -> handover.
E5 Bill & collect (D14/D15): time/expense -> invoice -> approve -> deliver -> remind -> reconcile.
E6 Govern (D08/D09/D10): request -> route -> approve -> sign (guardrailed) -> archive.
E7 Assure (D11/D22): risk screen -> control test -> finding -> CAPA -> evidence pack.
E8 Operate (D17/D25): provision tenant/office -> configure -> monitor SLO -> backup/restore drill.
Each story maps to feature IDs in 03-FEATURE-CATALOG; acceptance criteria live per feature.
""")

w("05-BUSINESS-RULES/RULES.md", """
# Business Rules Register (extract; full rule text per feature in catalog)
- BR-G01 Tenant/office scoping mandatory on every query. - BR-G02 Legal hold blocks deletion/purge.
- BR-G03 PII display masking by role; NIK never in logs. - BR-G04 Money/deed/sign actions need approval.
- BR-G05 Consent (UU PDP) required before processing; withdrawal honored prospectively.
- BR-ID01 NIK 16-digit + province/regency sanity; expiry tracked. - BR-DEED01 Deed numbers immutable once issued;
  corrections via renvoi/akta perbaikan. - BR-PROTO01 Protocol retention 25 years overrides standard purge.
- BR-SIGN01 Certified PSrE only for qualifying signatures; akta exclusion warning enforced.
- BR-FIN01 Trust ledger never negative; segregation of duties on journals.
- BR-WF01 Idempotent transitions; illegal transitions rejected (409).
Numbering: full BR-### per feature; conflicts resolved by G-rules first, then domain rules.
""")

w("06-SYSTEM-ARCHITECTURE/ARCHITECTURE.md", """
# System Architecture (planning)
## Style
Modular monolith (single Rust binary + Leptos SSR), DDD bounded contexts per domain (D01-D25),
clean-architecture layering inside modules (handler -> service -> repo), ports/traits for
OCR/AI/storage/events. Event-driven via NATS subjects (`legal.<tenant>.<domain>.<event>.v1`);
Redis for cache/locks/rate-limit; S3/MinIO objects; Tantivy search; Qdrant optional vectors.

```mermaid
flowchart LR
  UI[Leptos SSR/WASM] --> API[Axum API + Tower]
  API --> SVC[Domain services D01-D25]
  SVC --> DB[(MySQL 8.4)]
  SVC --> OBJ[(S3/MinIO)]
  SVC --> Q[NATS]
  SVC --> C[(Redis)]
  SVC --> OCR[OCR router]
  SVC --> AI[AI router]
  Q --> W[Workers: OCR/AI/index/notify/billing]
  W --> DB
  W --> OBJ
```

## Request path
```mermaid
sequenceDiagram
  participant U as User
  participant A as Axum+Tower
  participant S as Service
  participant D as MySQL
  U->>A: HTTPS + JWT/OIDC, idempotency-key
  A->>S: validated DTO (serde)
  S->>D: tx + FK-checked writes
  S-->>Q: domain event (NATS)
```

## Workers/idempotency/retries
At-least-once delivery; consumer idempotency keys + dedupe table; exponential backoff w/ jitter;
poison messages to DLQ + error inbox; saga/compensation for cross-module ops (no 2PC).
## Extraction triggers
p95 breach sustained, team contention, blast-radius need, or regulatory isolation — then extract
behind versioned API with dual-run.
## ADR log
See 00-EXECUTIVE/DECISIONS.md. Crate baselines: axum 0.8.9, tower-http 0.7, tokio 1.54.x,
sqlx 0.9 (MSRV 1.94), serde 1.x, leptos stable 0.8.23 (no RC).
""")

w("07-DATABASE-ARCHITECTURE/DATABASE.md", """
# MySQL 8.4 LTS Logical Architecture (planning; example DDL only, NOT migrations)
## Principles
InnoDB everywhere; utf8mb4/unicode_ci; BIGINT UNSIGNED PKs (UUIDv7 external refs where needed);
every tenant row carries tenant_id (+office_id); FKs enforced; no polymorphic associations
(use explicit link tables); append-only event/audit tables; JSON only for schemaless payload
with generated-column indexes; partitioning for hot tables (events, ocr_results, audit).

## Core tables (extract)
tenants, offices, users, roles, permissions, role_permissions, user_roles, clients,
client_contacts, matters, matter_team, tasks, documents, document_versions, ocr_jobs,
ocr_results, deed_records, deed_parties, workflows, workflow_instances, approvals,
esign_envelopes, invoices, payments, trust_ledger, notifications, audit_events, retention_policies.
All tables: created_at/updated_at, created_by, row_version (optimistic locking), soft-delete
(deleted_at) except append-only logs.

## Index strategy
PK + (tenant_id, ...) composite first; covering indexes for list screens; FULLTEXT via Tantivy
side-index (MySQL FULLTEXT not primary); FK indexes; partial indexes via generated columns.

## Example DDL (illustrative only)
```sql
CREATE TABLE tenants (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(200) NOT NULL, status ENUM('active','suspended') NOT NULL DEFAULT 'active',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE matters (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id BIGINT UNSIGNED NOT NULL, office_id BIGINT UNSIGNED NOT NULL,
  number VARCHAR(40) NOT NULL, title VARCHAR(300) NOT NULL,
  status VARCHAR(40) NOT NULL DEFAULT 'open',
  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  created_by BIGINT UNSIGNED NOT NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_matters_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id),
  UNIQUE KEY uq_matters_tenant_number (tenant_id, number),
  KEY ix_matters_office_status (office_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE audit_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  tenant_id BIGINT UNSIGNED NOT NULL, actor_id BIGINT UNSIGNED NULL,
  action VARCHAR(120) NOT NULL, entity VARCHAR(120) NOT NULL, entity_id BIGINT UNSIGNED NOT NULL,
  before_hash CHAR(64) NULL, after_hash CHAR(64) NULL,
  ip VARCHAR(64) NULL, created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_audit_entity (tenant_id, entity, entity_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```
Tenant isolation: mandatory `tenant_id` predicate (reviewed), separate KMS data keys per tenant
optional; backups per-tenant restorable; retention vs legal-hold matrix in 11/15.
""")

w("08-API-ARCHITECTURE/API.md", """
# API Architecture (planning)
REST v1 (`/api/v1/<domain>/...`), JSON (serde), cursor pagination, problem-details errors,
idempotency-key on POST/PATCH, optimistic locking (If-Match: row_version), RBAC scopes
(`perm:<domain>:<action>`), tenant/office claim enforcement server-side.
Auth: OIDC SSO primary, JWT access (short) + rotating refresh, MFA step-up for sensitive ops,
service accounts via OAuth2 client-credentials, HMAC request signing for webhooks.
Events: NATS subjects `legal.<tenant>.<domain>.<event>.v1`, schema-registry validated,
webhook dispatcher with retry/DLQ. OpenAPI published from code annotations at implementation;
sandbox + changelog + deprecation policy. Rate plans per tenant; abuse detection.
""")

w("09-OCR-ENGINE/OCR-PRD.md", """
# OCR & Document Intelligence PRD (planning)
## Scope
KTP/KK/NPWP/passport/property-cert/deed/contract/general-PDF; multi-page; tables; layout;
classification; enhancement (deskew/denoise/perspective/blur gate); confidence; field validation;
HITL review; versioning; archival. PII masked in logs.
## Engine strategy (trait-routed; benchmark before commit)
- **ocrs** (pure Rust, ONNX runtime dep): best for line-level Latin text; needs detection tuning for ID cards.
- **oar-ocr**: archival-oriented pipeline pieces; evaluate vs build cost.
- **Tesseract bindings** (tesseral/teeseract crates): strongest ID-doc accuracy baseline but
  brings **native libtesseract + traineddata** (NOT pure Rust) — document as external dependency.
- Cloud/local vision models via AI router for low-confidence fallback only (cost/PDP controlled).
Decision: router interface `OcrEngine::recognize(image)->lines+conf`; per-doc-type engine profiles;
winner chosen per doc type on measured CER/WER/field-accuracy, not assumption.
## Accuracy methodology
CER = edit-dist/ground-truth chars; WER word-level; field-level F1 on NIK/names/dates/amounts;
classification accuracy; p95 latency; human-correction rate. Corpus: stratified Indonesian samples
(good/flat-lay/glare/rotated/low-res), ground-truth double-keyed, versioned (see 23-RESEARCH).
Gates (planning targets, to confirm): printed KTP field-acc >= 97%, CER <= 2% clean scans;
contracts clause-seg F1 >= 0.85; correction-rate dashboard tracked per engine/profile.
## Pipeline
ingest -> QC gate -> enhance -> classify -> route engine -> extract -> validate -> confidence gate
(auto-pass >= T_high, review band, reject <= T_low with guidance) -> HITL -> version -> archive.
Retry/DLQ; cost metering per job.
""")

w("10-AI-ENGINE/AI-PRD.md", """
# AI Engine PRD (planning)
Provider-independent core: `AiProvider` trait (chat/extract/embed), model router
(cost/quality/latency/data-class), token metering + per-matter allocation, prompt template
library with versioning/rollback, structured extraction (serde-validated JSON), RAG over
matter-scoped knowledge base (Tantivy + optional Qdrant vectors), citation/source tracking,
summarization/comparison/clause-risk drafting aids. **AI never certifies legal validity**;
every AI draft/flag requires human approval with disclaimer banner; blocked-topic policy
refuses validity-certification requests. Safety: prompt-injection defenses (instruction hierarchy,
untrusted-content delimiting, tool-output validation), PII redaction pre-call, tenant-scoped
retrieval, data-retention limits, full audit (prompt hash/response/model/version/cost).
Providers: OpenAI + Anthropic + local models behind same trait; keys in vault; eval harness
(grounded-QA accuracy, citation precision, refusal correctness, regression on prompt change).
""")

w("11-DOCUMENT-ENGINE/DOCUMENT.md", """
# Document Engine (planning)
Lifecycle: ingest (upload/scan/email/mobile) -> AV-scan -> hash (SHA-256) -> classify ->
OCR-queue -> metadata -> version -> workflow/approval -> sign (guardrailed) -> seal ->
retain -> legal-hold aware purge. Versioning major/minor with diff; check-in/out locks;
watermark/stamp/redaction; matter-bundle export with manifest; S3/MinIO lifecycle tiers;
trash-with-restore; integrity re-hash jobs; protocol segregation for notarial originals.
Metadata schema + custom fields; retention engine vs holds; storage quotas; large-file chunking.
""")

w("12-LEGAL-WORKFLOWS/WORKFLOWS.md", """
# Legal Workflows (planning)
W1 Notarial deed: intake -> KYC -> draft -> review -> repertorium number -> reading/signing
ceremony (physical) -> minuta/protocol filing -> copies. W2 PPAT transfer: cert intake ->
BPN check -> AJB/APHT/SKMHT -> tax worksheets -> signing -> BPN submission -> handover.
W3 Litigation support: deadlines -> filings -> evidence bundle -> hearing prep. W4 Corporate:
engagement -> KYC/COI -> drafting -> approval matrix -> e-sign (non-akta) -> closing binder.
W5 Compliance: screen -> assess -> approve -> monitor -> report. Each workflow = reusable
template in D08 with SLA, escalation, evidence pack, and guardrails (akta physical-presence,
e-sign exclusion, PDP consent).
""")

w("13-UI-UX/DESIGN-SYSTEM.md", """
# UI/UX — Premium Enterprise LegalTech (planning)
White/light workspace, sophisticated navy/blue accents; no generic dark admin look.
Design tokens: --bg #FFFFFF, --surface #F6F8FB, --ink #0F1F3D, --navy #1B3A6B, --accent #2563EB,
--success #15803D, --warn #B45309, --danger #B91C1C; Inter-like Latin stack + Noto Sans fallback
for ID diacritics; 8pt spacing; radius 10/14; accessible focus rings; WCAG 2.2 AA contrast.
Navigation: global command bar + office switcher, matter-centric sidebar, role dashboards
(notary/PPAT/partner/finance/auditor/client-portal). Components: data tables (sticky header,
column prefs), document preview + redaction, OCR review split-view, AI side-panel with citations,
workflow visualizer, approval inbox, signing ceremony checklist. Screen inventory: 25 domains x
5 screens minimum (SCR1-5 per feature) + dashboards; user journeys mapped in STORY-MAP;
responsive breakpoints 360/768/1280/1600; Indonesian + English strings from day one.
""")

w("14-SECURITY/SECURITY.md", """
# Security Architecture (planning)
AuthN: OIDC SSO, Argon2id passwords, TOTP + WebAuthn/passkeys, step-up MFA for money/deed/sign.
AuthZ: RBAC + office/matter scopes + ABAC policies + break-glass (logged, time-boxed).
Crypto: AES-256 at rest, TLS 1.2+, KMS/HSM keys, rotation schedule, field-level encryption for
NIK/PII. Sessions: short JWT + rotating refresh, anomaly detection, concurrent control.
App: CSRF/XSS/SQLi guardrails (SQLx parameterized), security headers, WAF baseline, rate limits,
secure upload + AV gate, secrets vault, dependency/container/SAST/DAST scans. Audit: hash-chained
append-only logs, tamper-evident evidence store. IR: runbooks + breach-notification (UU PDP 72h)
workflow. OWASP ASVS mapping table + threat models (STRIDE per domain) maintained in 22-RISK.
""")

w("15-COMPLIANCE/COMPLIANCE-MATRIX.md", """
# Compliance Matrix (Indonesia-first; items marked *VERIFY require legal counsel)
| Requirement | Source | Control | Evidence | Status |
|---|---|---|---|---|
| Notary authority & authentic deed form; physical presence | UUJN 30/2004 jo 2/2014; KUHPerdata 1868 | Physical-presence guardrail; ceremony checklist; repertorium/klapper/protokol | Ceremony logs, protocol registers | Implemented-by-design* |
| Cyber-notary / virtual presence | UUJN 15(3) open clause, no implementing rule | Design seam only; no virtual authentic deed until harmonized | ADR + guardrail log | GAP — law pending* |
| E-doc/e-signature validity; **exclusion for notarial/PPAT deeds** | UU ITE 1/2024 (Pasal 5(4)); PP PSTE 71/2019 | Akta-exclusion warning; certified-PSrE path only for qualifying docs; TSA/LTV | Signing logs, provider certs | Guardrailed* |
| Personal data protection (controller duties, consent, ROPA, DPIA, breach 72h, rights) | UU PDP 27/2022 (enforced Oct 2024) | Consent lifecycle, DSAR/deletion/rectification, ROPA, DPIA, retention engine | ROPA, DPIA reports, breach log | Must-implement |
| Protocol retention (25 thn) vs deletion rights | UUJN + UU PDP | Legal-hold overrides purge; protocol scheduler | Hold register, purge approvals | Designed* |
| PPAT duties & land registration | PP 24/1997, Perkaban/BPN rules | PPAT register, BPN tracker, tax worksheets | PPAT reports, receipts | Must-implement* |
| Confidentiality of deeds | UUJN 16(1)f | Classification, masking, access scopes | Access audit | Designed |
| Financial/tax records (faktur, PPh, PPN) | Tax regs | Invoice/tax logs, e-faktur export | Ledgers, exports | Must-implement* |
Gaps needing counsel sign-off flagged *VERIFY throughout.
""")

w("16-TESTING-QA/STRATEGY.md", """
# QA Strategy (planning)
Levels: unit (services/rules), integration (API+DB tx), contract (OpenAPI), DB (FK/migration),
UI (Leptos component + a11y axe), security (ASVS/pen-test), perf/load (k6-style SLOs),
OCR benchmarks (CER/WER/field-acc, corpus-versioned), AI evals (groundedness/citation/refusal),
DR drills. Every feature: AC1-5 + TC-P/N + EC mapped in catalog = traceability matrix source.
Gates: coverage thresholds set at implementation; no gate pass without evidence artifacts.
""")

w("17-DEVOPS/DEVOPS.md", """
# DevOps (planning)
Envs: dev/staging/prod parity; IaC for MySQL/Redis/NATS/MinIO; SQLx offline-check in CI;
cargo deny/audit, container scan; migration dry-run; blue/green or rolling deploys; feature flags;
OTel (traces/metrics/logs) + SLO dashboards; backup (full+binlog, encrypted, per-tenant restore
tested) + restore drills; secrets via vault; maintenance-mode + status page; runbooks in 21.
""")

w("18-ROADMAP/ROADMAP.md", """
# Roadmap (planning phases; implementation starts only after explicit approval)
P0 Foundation (arch, DB core, auth/tenant, audit, docs validation). P1 Matter+documents+OCR
pipeline with HITL. P2 Deed/PPAT pack + approvals + certified e-sign (guardrailed). P3 AI engine
(RAG, evals) + billing/finance. P4 Portal/mobile-field + analytics. P5 Hardening (pen-test,
load, DR) + locale pack v2 + multi-region readiness. Milestones M1 catalog validated, M2 arch
frozen, M3 OCR benchmark published, M4 AI eval green, M5 pilot-ready. Critical path:
DB core -> auth/scope -> documents -> OCR/HITL -> deed -> approvals/sign -> billing.
Dependencies in 19-AGENT-ORCHESTRATION/DEPENDENCY-GRAPH.md.
""")

w("19-AGENT-ORCHESTRATION/AGENTS.md", """
# Agent Orchestration (planning; no agents running)
## Registry
Chief-architect, product-manager, domain-architect x25 (one scope each), rust-backend,
leptos-frontend, mysql-architect, ocr-researcher, ai-researcher, security-researcher,
qa-engineer, documentation-engineer, integration-reviewer. Max realistic concurrency: 4-8
agents; 1,000 work packages are QUEUED, not parallel (see TASK-QUEUE).
## Rules (AGENT-RULES)
Isolated scopes (one domain file per agent); never edit shared index/catalog generator outputs
directly — propose via patch files; every contribution needs validation + human/architect review.
## Merge policy / quality gates
See QUALITY-GATES.md: unique IDs, 40/domain, AC+tests present, FK discipline, no RC baselines,
evidence-required states. Concurrency limits + resource notes documented; pretending 1,000
parallel agents is forbidden.
""")

w("19-AGENT-ORCHESTRATION/AGENT_REGISTRY.md", """
# AGENT_REGISTRY
| Agent | Scope | Concurrency |
|---|---|---|
| chief-architect | ADR + cross-domain coherence | 1 |
| product-manager | PRD + story map + rules | 1 |
| domain-architect-D01..D25 | single domain file only | up to 4 parallel |
| rust-backend | module contracts, NATS schemas | 1 |
| leptos-frontend | design system conformance | 1 |
| mysql-architect | DDL review, index strategy | 1 |
| ocr-researcher | benchmarks, corpus | 1 |
| ai-researcher | evals, safety | 1 |
| security-researcher | ASVS, threat models | 1 |
| qa-engineer | traceability, gates | 1 |
| documentation-engineer | index, validation | 1 |
| integration-reviewer | merge review | 1 |
Actual parallelism capped at 4-8; queue holds 1,000 packages.
""")

w("19-AGENT-ORCHESTRATION/TASK_QUEUE.md", """
# TASK_QUEUE (1,000 packages queued; states per 20-PROGRESS)
Packages: PKG-D01-001..PKG-D25-040 map 1:1 to feature IDs. Default state: READY_FOR_IMPLEMENTATION
is FORBIDDEN until planning gates pass; current state DRAFT (documented, not implemented).
No package may enter IN_PROGRESS without explicit implementation approval.
""")

w("19-AGENT-ORCHESTRATION/DEPENDENCY-GRAPH.md", """
# DEPENDENCY_GRAPH (extract)
D16/D17 (identity/tenant) -> all. D02 -> D01. D03 -> D02. D04 -> D03. D05 -> D04.
D06 -> D04+D05. D08/D09 -> D03. D10 -> D09 (guardrailed). D13 -> D06. D14/D15 -> D03.
D07 -> D04+D10(AI never certifies). D21 -> D16. D22/D11 cross-cutting. D23 -> D04.
Critical path: D16/D17 -> D02 -> D03 -> D04 -> D05 -> D06 -> D09/D10.
""")

w("19-AGENT-ORCHESTRATION/AGENT_RULES.md", """
# AGENT_RULES
1. One scope per agent per turn. 2. Read before write. 3. Never hand-edit generated catalog
files; change the generator + rerun. 4. No shared-file concurrent edits. 5. Evidence with every
claim. 6. Mark *VERIFY items for counsel. 7. Respect 100%-Rust app-code rule in designs.
""")

w("19-AGENT-ORCHESTRATION/MERGE_POLICY.md", """
# MERGE_POLICY
Patches need: scope check, validation script green, architect review, no RC versions,
FK discipline, audit coverage. Direct pushes to frozen planning baseline forbidden.
""")

w("19-AGENT-ORCHESTRATION/QUALITY_GATES.md", """
# QUALITY_GATES (planning)
QG1 25 domains exist. QG2 >=40 features/domain. QG3 >=1000 features. QG4 >=10 opps/feature.
QG5 IDs unique. QG6 AC+tests per feature. QG7 DB covers required domains. QG8 OCR PRD complete.
QG9 AI safety documented. QG10 UI inventory exists. QG11 Security+compliance matrices exist.
QG12 API conventions exist. QG13 Roadmap deps exist. QG14 Agent docs exist. QG15 Index current.
QG16 Assumptions listed. All gates evaluated by script + human review in VALIDATION-REPORT.
""")

w("20-PROGRESS/TRACKER.md", """
# Progress Tracker
States: NOT_STARTED RESEARCH DRAFT IN_REVIEW APPROVED READY_FOR_IMPLEMENTATION IN_PROGRESS
TESTING BLOCKED COMPLETED.
| Area | State | Evidence |
|---|---|---|
| Product vision / PRD / rules | DRAFT | docs 00/01/02/05 |
| 25 domains / 1,000 features | DRAFT (generated, script-verified counts) | catalog + validation |
| 10,000 expansions | DRAFT (generated) | expansions + validation |
| DB/API/OCR/AI/UI/Security/Compliance/QA/DevOps/Roadmap/Agents | DRAFT | docs 06-19 |
| Implementation | NOT_STARTED | awaiting explicit approval |
Nothing marked COMPLETED/APPROVED without review evidence.
""")

w("21-OPERATIONS/RUNBOOKS.md", """
# Operations (planning)
SLOs (targets): API p95 < 400ms (non-OCR/AI), ingestion success >= 99.5%, backup restore tested
monthly, search p95 < 800ms. Runbooks: deploy/rollback, queue drain, OCR DLQ replay, AI cost
spike, cert rotation, breach 72h, BPN/provider outage fallback, support tiers L1-L3 with
matter-aware SLAs. On-call, maintenance windows, status comms defined at implementation.
""")

w("22-RISK-REGISTER/RISKS.md", """
# Risk Register (top; full per-feature risks in catalog)
R1 Regulatory disharmony (UUJN vs UU ITE 5(4)) limits e-akta — Mit: guardrails + counsel track; Sev High.
R2 UU PDP breach exposure (notary as controller) — Mit: consent/ROPA/DPIA/72h workflow; High.
R3 OCR accuracy below deed-grade — Mit: benchmark gates + HITL; High.
R4 AI hallucination / validity-certification misuse — Mit: refusal policy + approval gates; High.
R5 Scope creep (1,000 feats) — Mit: phased roadmap, packages queued; Medium.
R6 Secret/cost sprawl (AI/OCR/providers) — Mit: vault + metering + budgets; Medium.
R7 Data loss — Mit: encrypted backups + restore drills; High.
R8 Mobile offline conflicts — Mit: CRDT-ish rules + review; Medium.
Each risk: owner, trigger metric, mitigation, contingency. STRIDE models per domain at implementation.
""")

w("23-RESEARCH/CRATES.md", """
# Crate & Platform Research (verified 2026-11-17 via crates.io; re-verify at kickoff)
- axum **0.8.9** stable (MSRV 1.80). tower-http **0.7**. tokio **1.54.x** (MSRV 1.85).
- sqlx **0.9** (MSRV 1.94; fallback `=0.8.6` if toolchain frozen). serde **1.x**. serde_json 1.x.
- leptos stable **0.8.23** REQUIRED baseline; 0.9.x is RC — excluded from production path.
- tailwindcss (via leptos/tailwind integration) + SSR; toolchain: Rust stable current.
- MySQL **8.4 LTS**, InnoDB. Redis 7/8, NATS 2.x, MinIO (S3), Tantivy 0.2x, Qdrant optional.
- Dioxus Mobile: evaluated — Rust codebase, mobile targets experimental, store/API friction;
  decision: document only; choose web-first responsive + PWA, revisit native per market; NO Dart.
- OCR: ocrs (pure Rust + ONNX dep), oar-ocr (pipeline parts), Tesseract bindings = **native
  libtesseract external dep (not pure Rust)** — allowed as infra, must be declared; cloud/local
  vision fallback via AI router. Benchmarks required before engine commitment.
- AI: OpenAI + Anthropic + local (ollama/llama.cpp-class) behind trait; no training on client data
  without consent; eval harness mandatory.
Risks: Leptos 0.9 churn, sqlx MSRV jump, Tesseract native packaging, Qdrant ops cost.
""")

w("03-FEATURE-CATALOG/README.md", """
# Feature Catalog — 25 domains / 1,000 primary features / 10,000 expansions
Per-feature spec contains: ID, name, objective, description, users, stories, FR/NFR, rules,
subfeatures, entities, API, screens, permissions, security, audit, dependencies, acceptance,
positive/negative/edge tests, priority, complexity, effort, future pointers.
Machine-readable: feature-catalog.json/.csv; expansions in expansions/ (per-domain .md +
expansion-catalog.json/.csv with parent links). Generated by docs-gen.py (planning only).
""")

print("PHASE2 OK")
