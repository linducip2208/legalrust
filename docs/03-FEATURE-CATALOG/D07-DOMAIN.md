# D07 — AI Legal Assistant

Domain 07 of 25. 40 primary features. All IDs unique. Planning-only; no implementation claimed.

## Feature index

- F-D07-001 — Conversational legal Q&A (ID)
- F-D07-002 — Matter-grounded Q&A (RAG)
- F-D07-003 — Document summarization
- F-D07-004 — Contract clause extraction
- F-D07-005 — Risk clause flagging
- F-D07-006 — Obligation & deadline extraction
- F-D07-007 — Document comparison (AI diff narrative)
- F-D07-008 — Citation & source tracking
- F-D07-009 — Legal knowledge base ingestion
- F-D07-010 — Knowledge base versioning
- F-D07-011 — Prompt template library
- F-D07-012 — Prompt versioning & rollback
- F-D07-013 — Model router (cost/quality/latency)
- F-D07-014 — Token usage metering
- F-D07-015 — Per-matter AI cost allocation
- F-D07-016 — AI confidence & disclaimer banner
- F-D07-017 — Human approval gate for AI drafts
- F-D07-018 — AI-assisted deed clause drafting
- F-D07-019 — AI-assisted engagement letter drafting
- F-D07-020 — AI-assisted email drafting
- F-D07-021 — Meeting-notes summarization
- F-D07-022 — Regulation change digest
- F-D07-023 — Case-law research helper
- F-D07-024 — Precedent retrieval
- F-D07-025 — Jurisdiction guardrail (ID-first)
- F-D07-026 — Hallucination mitigation controls
- F-D07-027 — Prompt-injection protection
- F-D07-028 — PII redaction before model call
- F-D07-029 — Local-model execution path
- F-D07-030 — Model evaluation harness
- F-D07-031 — Feedback (thumbs up/down + correction)
- F-D07-032 — AI audit trail (prompt/response/model)
- F-D07-033 — AI data retention policy
- F-D07-034 — Blocked-topic policy (validity certification)
- F-D07-035 — Multi-language response (ID/EN)
- F-D07-036 — Voice-to-text inquiry capture
- F-D07-037 — AI workspace (side panel)
- F-D07-038 — AI spend & model-mix analytics
- F-D07-039 — AI incident & correction log
- F-D07-040 — AI red-team prompt test suite

---

## F-D07-001 — Conversational legal Q&A (ID)

- **Business objective:** Reduce risk, cycle time, and rework for 'conversational legal q&a (id)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for conversational legal q&a (id) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'conversational legal q&a (id)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-001` entries in `expansions/D07-expansions.md`.

## F-D07-002 — Matter-grounded Q&A (RAG)

- **Business objective:** Reduce risk, cycle time, and rework for 'matter-grounded q&a (rag)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter-grounded q&a (rag) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'matter-grounded q&a (rag)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D07-002` entries in `expansions/D07-expansions.md`.

## F-D07-003 — Document summarization

- **Business objective:** Reduce risk, cycle time, and rework for 'document summarization' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for document summarization scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'document summarization' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-003` entries in `expansions/D07-expansions.md`.

## F-D07-004 — Contract clause extraction

- **Business objective:** Reduce risk, cycle time, and rework for 'contract clause extraction' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for contract clause extraction scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'contract clause extraction' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-004` entries in `expansions/D07-expansions.md`.

## F-D07-005 — Risk clause flagging

- **Business objective:** Reduce risk, cycle time, and rework for 'risk clause flagging' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for risk clause flagging scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'risk clause flagging' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-005` entries in `expansions/D07-expansions.md`.

## F-D07-006 — Obligation & deadline extraction

- **Business objective:** Reduce risk, cycle time, and rework for 'obligation & deadline extraction' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for obligation & deadline extraction scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'obligation & deadline extraction' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-006` entries in `expansions/D07-expansions.md`.

## F-D07-007 — Document comparison (AI diff narrative)

- **Business objective:** Reduce risk, cycle time, and rework for 'document comparison (ai diff narrative)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for document comparison (ai diff narrative) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'document comparison (ai diff narrative)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-007` entries in `expansions/D07-expansions.md`.

## F-D07-008 — Citation & source tracking

- **Business objective:** Reduce risk, cycle time, and rework for 'citation & source tracking' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for citation & source tracking scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'citation & source tracking' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-008` entries in `expansions/D07-expansions.md`.

## F-D07-009 — Legal knowledge base ingestion

- **Business objective:** Reduce risk, cycle time, and rework for 'legal knowledge base ingestion' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for legal knowledge base ingestion scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'legal knowledge base ingestion' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-009` entries in `expansions/D07-expansions.md`.

## F-D07-010 — Knowledge base versioning

- **Business objective:** Reduce risk, cycle time, and rework for 'knowledge base versioning' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for knowledge base versioning scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'knowledge base versioning' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D07-010` entries in `expansions/D07-expansions.md`.

## F-D07-011 — Prompt template library

- **Business objective:** Reduce risk, cycle time, and rework for 'prompt template library' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for prompt template library scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'prompt template library' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-011` entries in `expansions/D07-expansions.md`.

## F-D07-012 — Prompt versioning & rollback

- **Business objective:** Reduce risk, cycle time, and rework for 'prompt versioning & rollback' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for prompt versioning & rollback scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'prompt versioning & rollback' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-012` entries in `expansions/D07-expansions.md`.

## F-D07-013 — Model router (cost/quality/latency)

- **Business objective:** Reduce risk, cycle time, and rework for 'model router (cost/quality/latency)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for model router (cost/quality/latency) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'model router (cost/quality/latency)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-013` entries in `expansions/D07-expansions.md`.

## F-D07-014 — Token usage metering

- **Business objective:** Reduce risk, cycle time, and rework for 'token usage metering' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for token usage metering scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'token usage metering' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D07-014` entries in `expansions/D07-expansions.md`.

## F-D07-015 — Per-matter AI cost allocation

- **Business objective:** Reduce risk, cycle time, and rework for 'per-matter ai cost allocation' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for per-matter ai cost allocation scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'per-matter ai cost allocation' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-015` entries in `expansions/D07-expansions.md`.

## F-D07-016 — AI confidence & disclaimer banner

- **Business objective:** Reduce risk, cycle time, and rework for 'ai confidence & disclaimer banner' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai confidence & disclaimer banner scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai confidence & disclaimer banner' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-016` entries in `expansions/D07-expansions.md`.

## F-D07-017 — Human approval gate for AI drafts

- **Business objective:** Reduce risk, cycle time, and rework for 'human approval gate for ai drafts' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for human approval gate for ai drafts scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'human approval gate for ai drafts' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-017` entries in `expansions/D07-expansions.md`.

## F-D07-018 — AI-assisted deed clause drafting

- **Business objective:** Reduce risk, cycle time, and rework for 'ai-assisted deed clause drafting' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai-assisted deed clause drafting scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai-assisted deed clause drafting' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-018` entries in `expansions/D07-expansions.md`.

## F-D07-019 — AI-assisted engagement letter drafting

- **Business objective:** Reduce risk, cycle time, and rework for 'ai-assisted engagement letter drafting' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai-assisted engagement letter drafting scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai-assisted engagement letter drafting' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-019` entries in `expansions/D07-expansions.md`.

## F-D07-020 — AI-assisted email drafting

- **Business objective:** Reduce risk, cycle time, and rework for 'ai-assisted email drafting' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai-assisted email drafting scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai-assisted email drafting' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-020` entries in `expansions/D07-expansions.md`.

## F-D07-021 — Meeting-notes summarization

- **Business objective:** Reduce risk, cycle time, and rework for 'meeting-notes summarization' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for meeting-notes summarization scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'meeting-notes summarization' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D07-021` entries in `expansions/D07-expansions.md`.

## F-D07-022 — Regulation change digest

- **Business objective:** Reduce risk, cycle time, and rework for 'regulation change digest' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for regulation change digest scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'regulation change digest' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-022` entries in `expansions/D07-expansions.md`.

## F-D07-023 — Case-law research helper

- **Business objective:** Reduce risk, cycle time, and rework for 'case-law research helper' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for case-law research helper scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'case-law research helper' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-023` entries in `expansions/D07-expansions.md`.

## F-D07-024 — Precedent retrieval

- **Business objective:** Reduce risk, cycle time, and rework for 'precedent retrieval' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for precedent retrieval scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'precedent retrieval' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-024` entries in `expansions/D07-expansions.md`.

## F-D07-025 — Jurisdiction guardrail (ID-first)

- **Business objective:** Reduce risk, cycle time, and rework for 'jurisdiction guardrail (id-first)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for jurisdiction guardrail (id-first) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'jurisdiction guardrail (id-first)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-025` entries in `expansions/D07-expansions.md`.

## F-D07-026 — Hallucination mitigation controls

- **Business objective:** Reduce risk, cycle time, and rework for 'hallucination mitigation controls' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for hallucination mitigation controls scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'hallucination mitigation controls' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-026` entries in `expansions/D07-expansions.md`.

## F-D07-027 — Prompt-injection protection

- **Business objective:** Reduce risk, cycle time, and rework for 'prompt-injection protection' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for prompt-injection protection scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'prompt-injection protection' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-027` entries in `expansions/D07-expansions.md`.

## F-D07-028 — PII redaction before model call

- **Business objective:** Reduce risk, cycle time, and rework for 'pii redaction before model call' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for pii redaction before model call scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'pii redaction before model call' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D07-028` entries in `expansions/D07-expansions.md`.

## F-D07-029 — Local-model execution path

- **Business objective:** Reduce risk, cycle time, and rework for 'local-model execution path' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for local-model execution path scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'local-model execution path' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D07-029` entries in `expansions/D07-expansions.md`.

## F-D07-030 — Model evaluation harness

- **Business objective:** Reduce risk, cycle time, and rework for 'model evaluation harness' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for model evaluation harness scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'model evaluation harness' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-030` entries in `expansions/D07-expansions.md`.

## F-D07-031 — Feedback (thumbs up/down + correction)

- **Business objective:** Reduce risk, cycle time, and rework for 'feedback (thumbs up/down + correction)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for feedback (thumbs up/down + correction) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'feedback (thumbs up/down + correction)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-031` entries in `expansions/D07-expansions.md`.

## F-D07-032 — AI audit trail (prompt/response/model)

- **Business objective:** Reduce risk, cycle time, and rework for 'ai audit trail (prompt/response/model)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai audit trail (prompt/response/model) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai audit trail (prompt/response/model)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-032` entries in `expansions/D07-expansions.md`.

## F-D07-033 — AI data retention policy

- **Business objective:** Reduce risk, cycle time, and rework for 'ai data retention policy' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai data retention policy scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai data retention policy' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-033` entries in `expansions/D07-expansions.md`.

## F-D07-034 — Blocked-topic policy (validity certification)

- **Business objective:** Reduce risk, cycle time, and rework for 'blocked-topic policy (validity certification)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for blocked-topic policy (validity certification) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'blocked-topic policy (validity certification)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-034` entries in `expansions/D07-expansions.md`.

## F-D07-035 — Multi-language response (ID/EN)

- **Business objective:** Reduce risk, cycle time, and rework for 'multi-language response (id/en)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for multi-language response (id/en) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'multi-language response (id/en)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-035` entries in `expansions/D07-expansions.md`.

## F-D07-036 — Voice-to-text inquiry capture

- **Business objective:** Reduce risk, cycle time, and rework for 'voice-to-text inquiry capture' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for voice-to-text inquiry capture scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'voice-to-text inquiry capture' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D07-036` entries in `expansions/D07-expansions.md`.

## F-D07-037 — AI workspace (side panel)

- **Business objective:** Reduce risk, cycle time, and rework for 'ai workspace (side panel)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai workspace (side panel) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai workspace (side panel)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-037` entries in `expansions/D07-expansions.md`.

## F-D07-038 — AI spend & model-mix analytics

- **Business objective:** Reduce risk, cycle time, and rework for 'ai spend & model-mix analytics' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai spend & model-mix analytics scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai spend & model-mix analytics' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D07-038` entries in `expansions/D07-expansions.md`.

## F-D07-039 — AI incident & correction log

- **Business objective:** Reduce risk, cycle time, and rework for 'ai incident & correction log' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai incident & correction log scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai incident & correction log' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D07-039` entries in `expansions/D07-expansions.md`.

## F-D07-040 — AI red-team prompt test suite

- **Business objective:** Reduce risk, cycle time, and rework for 'ai red-team prompt test suite' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai red-team prompt test suite scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Lawyer / associate; supervisors; auditors.
- **User stories:** (1) As Lawyer / associate, I can complete 'ai red-team prompt test suite' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d07_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d07_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d07/items`, `GET/PUT /api/v1/d07/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d07:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D07-040` entries in `expansions/D07-expansions.md`.

---

## Cross-domain links (audit-generated, planning-level)

Candidate integration contracts. Direction OUT = this feature feeds; IN = this feature consumes.
Confirm payloads/events in detailed design (see 27-CROSS-DOMAIN-VALIDATION).

### F-D07-002 — Matter-grounded Q&A (RAG)
- OUT → F-D04-013 (Full-text content indexing) [retrieves-from]
- IN ← F-D04-013 (Full-text content indexing) [retrieved-by]

### F-D07-008 — Citation & source tracking
- OUT → F-D23-009 (Knowledge-base search) [links-evidence]

### F-D07-014 — Token usage metering
- OUT → F-D07-015 (Per-matter AI cost allocation) [aggregated-by]

### F-D07-015 — Per-matter AI cost allocation
- IN ← F-D07-014 (Token usage metering) [aggregated-by]

### F-D07-017 — Human approval gate for AI drafts
- OUT → F-D09-001 (Approval request composer) [delegates-to]

### F-D07-021 — Meeting-notes summarization
- OUT → F-D02-012 (Meeting & visit log) [summarizes]
- IN ← F-D02-012 (Meeting & visit log) [feeds-AI]

