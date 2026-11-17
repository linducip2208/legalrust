# D03 — Project & Case Management

Domain 03 of 25. 40 primary features. All IDs unique. Planning-only; no implementation claimed.

## Feature index

- F-D03-001 — Matter / project creation wizard
- F-D03-002 — Matter numbering scheme
- F-D03-003 — Matter classification & practice area
- F-D03-004 — Matter stage pipeline
- F-D03-005 — Task breakdown & assignment
- F-D03-006 — Task dependencies
- F-D03-007 — Milestone tracking
- F-D03-008 — Deadline & SLA engine
- F-D03-009 — Court / agency deadline calculator
- F-D03-010 — Calendar integration (matter deadlines)
- F-D03-011 — Kanban matter board
- F-D03-012 — Gantt timeline view
- F-D03-013 — Time entry (manual)
- F-D03-014 — Timer-based time capture
- F-D03-015 — Timesheet approval
- F-D03-016 — Matter budget vs actual
- F-D03-017 — Cost advance (panjar) tracking
- F-D03-018 — Expense attachment to matter
- F-D03-019 — Matter team & staffing
- F-D03-020 — Workload & capacity view
- F-D03-021 — Matter priority & escalation
- F-D03-022 — Sub-matter / workstream grouping
- F-D03-023 — Related-matter linking
- F-D03-024 — Matter templates library
- F-D03-025 — Recurring matter scheduler
- F-D03-026 — Matter notes & activity feed
- F-D03-027 — Internal memo on matter
- F-D03-028 — Matter hold / suspension
- F-D03-029 — Matter reopen workflow
- F-D03-030 — Matter closure checklist
- F-D03-031 — Post-matter review & lessons learned
- F-D03-032 — Matter profitability report
- F-D03-033 — Client matter statement
- F-D03-034 — Matter access control
- F-D03-035 — Matter audit history
- F-D03-036 — Bulk matter import
- F-D03-037 — Matter archive & retrieval
- F-D03-038 — Matter dashboard (firm-wide)
- F-D03-039 — Matter SLA breach alerts
- F-D03-040 — Matter inbox triage & auto-assignment

---

## F-D03-001 — Matter / project creation wizard

- **Business objective:** Reduce risk, cycle time, and rework for 'matter / project creation wizard' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter / project creation wizard scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter / project creation wizard' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-001` entries in `expansions/D03-expansions.md`.

## F-D03-002 — Matter numbering scheme

- **Business objective:** Reduce risk, cycle time, and rework for 'matter numbering scheme' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter numbering scheme scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter numbering scheme' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-002` entries in `expansions/D03-expansions.md`.

## F-D03-003 — Matter classification & practice area

- **Business objective:** Reduce risk, cycle time, and rework for 'matter classification & practice area' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter classification & practice area scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter classification & practice area' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-003` entries in `expansions/D03-expansions.md`.

## F-D03-004 — Matter stage pipeline

- **Business objective:** Reduce risk, cycle time, and rework for 'matter stage pipeline' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter stage pipeline scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter stage pipeline' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-004` entries in `expansions/D03-expansions.md`.

## F-D03-005 — Task breakdown & assignment

- **Business objective:** Reduce risk, cycle time, and rework for 'task breakdown & assignment' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for task breakdown & assignment scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'task breakdown & assignment' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-005` entries in `expansions/D03-expansions.md`.

## F-D03-006 — Task dependencies

- **Business objective:** Reduce risk, cycle time, and rework for 'task dependencies' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for task dependencies scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'task dependencies' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-006` entries in `expansions/D03-expansions.md`.

## F-D03-007 — Milestone tracking

- **Business objective:** Reduce risk, cycle time, and rework for 'milestone tracking' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for milestone tracking scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'milestone tracking' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-007` entries in `expansions/D03-expansions.md`.

## F-D03-008 — Deadline & SLA engine

- **Business objective:** Reduce risk, cycle time, and rework for 'deadline & sla engine' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for deadline & sla engine scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'deadline & sla engine' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-008` entries in `expansions/D03-expansions.md`.

## F-D03-009 — Court / agency deadline calculator

- **Business objective:** Reduce risk, cycle time, and rework for 'court / agency deadline calculator' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for court / agency deadline calculator scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'court / agency deadline calculator' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-009` entries in `expansions/D03-expansions.md`.

## F-D03-010 — Calendar integration (matter deadlines)

- **Business objective:** Reduce risk, cycle time, and rework for 'calendar integration (matter deadlines)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for calendar integration (matter deadlines) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'calendar integration (matter deadlines)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-010` entries in `expansions/D03-expansions.md`.

## F-D03-011 — Kanban matter board

- **Business objective:** Reduce risk, cycle time, and rework for 'kanban matter board' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for kanban matter board scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'kanban matter board' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-011` entries in `expansions/D03-expansions.md`.

## F-D03-012 — Gantt timeline view

- **Business objective:** Reduce risk, cycle time, and rework for 'gantt timeline view' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for gantt timeline view scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'gantt timeline view' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-012` entries in `expansions/D03-expansions.md`.

## F-D03-013 — Time entry (manual)

- **Business objective:** Reduce risk, cycle time, and rework for 'time entry (manual)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for time entry (manual) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'time entry (manual)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D03-013` entries in `expansions/D03-expansions.md`.

## F-D03-014 — Timer-based time capture

- **Business objective:** Reduce risk, cycle time, and rework for 'timer-based time capture' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for timer-based time capture scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'timer-based time capture' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-014` entries in `expansions/D03-expansions.md`.

## F-D03-015 — Timesheet approval

- **Business objective:** Reduce risk, cycle time, and rework for 'timesheet approval' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for timesheet approval scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'timesheet approval' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-015` entries in `expansions/D03-expansions.md`.

## F-D03-016 — Matter budget vs actual

- **Business objective:** Reduce risk, cycle time, and rework for 'matter budget vs actual' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter budget vs actual scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter budget vs actual' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-016` entries in `expansions/D03-expansions.md`.

## F-D03-017 — Cost advance (panjar) tracking

- **Business objective:** Reduce risk, cycle time, and rework for 'cost advance (panjar) tracking' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for cost advance (panjar) tracking scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'cost advance (panjar) tracking' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-017` entries in `expansions/D03-expansions.md`.

## F-D03-018 — Expense attachment to matter

- **Business objective:** Reduce risk, cycle time, and rework for 'expense attachment to matter' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for expense attachment to matter scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'expense attachment to matter' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-018` entries in `expansions/D03-expansions.md`.

## F-D03-019 — Matter team & staffing

- **Business objective:** Reduce risk, cycle time, and rework for 'matter team & staffing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter team & staffing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter team & staffing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-019` entries in `expansions/D03-expansions.md`.

## F-D03-020 — Workload & capacity view

- **Business objective:** Reduce risk, cycle time, and rework for 'workload & capacity view' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for workload & capacity view scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'workload & capacity view' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-020` entries in `expansions/D03-expansions.md`.

## F-D03-021 — Matter priority & escalation

- **Business objective:** Reduce risk, cycle time, and rework for 'matter priority & escalation' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter priority & escalation scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter priority & escalation' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-021` entries in `expansions/D03-expansions.md`.

## F-D03-022 — Sub-matter / workstream grouping

- **Business objective:** Reduce risk, cycle time, and rework for 'sub-matter / workstream grouping' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for sub-matter / workstream grouping scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'sub-matter / workstream grouping' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-022` entries in `expansions/D03-expansions.md`.

## F-D03-023 — Related-matter linking

- **Business objective:** Reduce risk, cycle time, and rework for 'related-matter linking' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for related-matter linking scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'related-matter linking' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-023` entries in `expansions/D03-expansions.md`.

## F-D03-024 — Matter templates library

- **Business objective:** Reduce risk, cycle time, and rework for 'matter templates library' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter templates library scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter templates library' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-024` entries in `expansions/D03-expansions.md`.

## F-D03-025 — Recurring matter scheduler

- **Business objective:** Reduce risk, cycle time, and rework for 'recurring matter scheduler' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for recurring matter scheduler scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'recurring matter scheduler' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D03-025` entries in `expansions/D03-expansions.md`.

## F-D03-026 — Matter notes & activity feed

- **Business objective:** Reduce risk, cycle time, and rework for 'matter notes & activity feed' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter notes & activity feed scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter notes & activity feed' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-026` entries in `expansions/D03-expansions.md`.

## F-D03-027 — Internal memo on matter

- **Business objective:** Reduce risk, cycle time, and rework for 'internal memo on matter' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for internal memo on matter scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'internal memo on matter' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-027` entries in `expansions/D03-expansions.md`.

## F-D03-028 — Matter hold / suspension

- **Business objective:** Reduce risk, cycle time, and rework for 'matter hold / suspension' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter hold / suspension scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter hold / suspension' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-028` entries in `expansions/D03-expansions.md`.

## F-D03-029 — Matter reopen workflow

- **Business objective:** Reduce risk, cycle time, and rework for 'matter reopen workflow' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter reopen workflow scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter reopen workflow' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-029` entries in `expansions/D03-expansions.md`.

## F-D03-030 — Matter closure checklist

- **Business objective:** Reduce risk, cycle time, and rework for 'matter closure checklist' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter closure checklist scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter closure checklist' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D03-030` entries in `expansions/D03-expansions.md`.

## F-D03-031 — Post-matter review & lessons learned

- **Business objective:** Reduce risk, cycle time, and rework for 'post-matter review & lessons learned' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for post-matter review & lessons learned scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'post-matter review & lessons learned' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-031` entries in `expansions/D03-expansions.md`.

## F-D03-032 — Matter profitability report

- **Business objective:** Reduce risk, cycle time, and rework for 'matter profitability report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter profitability report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter profitability report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D03-032` entries in `expansions/D03-expansions.md`.

## F-D03-033 — Client matter statement

- **Business objective:** Reduce risk, cycle time, and rework for 'client matter statement' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for client matter statement scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'client matter statement' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-033` entries in `expansions/D03-expansions.md`.

## F-D03-034 — Matter access control

- **Business objective:** Reduce risk, cycle time, and rework for 'matter access control' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter access control scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter access control' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D03-034` entries in `expansions/D03-expansions.md`.

## F-D03-035 — Matter audit history

- **Business objective:** Reduce risk, cycle time, and rework for 'matter audit history' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter audit history scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter audit history' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-035` entries in `expansions/D03-expansions.md`.

## F-D03-036 — Bulk matter import

- **Business objective:** Reduce risk, cycle time, and rework for 'bulk matter import' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for bulk matter import scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'bulk matter import' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-036` entries in `expansions/D03-expansions.md`.

## F-D03-037 — Matter archive & retrieval

- **Business objective:** Reduce risk, cycle time, and rework for 'matter archive & retrieval' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter archive & retrieval scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter archive & retrieval' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D03-037` entries in `expansions/D03-expansions.md`.

## F-D03-038 — Matter dashboard (firm-wide)

- **Business objective:** Reduce risk, cycle time, and rework for 'matter dashboard (firm-wide)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter dashboard (firm-wide) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter dashboard (firm-wide)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D03-038` entries in `expansions/D03-expansions.md`.

## F-D03-039 — Matter SLA breach alerts

- **Business objective:** Reduce risk, cycle time, and rework for 'matter sla breach alerts' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter sla breach alerts scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter sla breach alerts' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D03-039` entries in `expansions/D03-expansions.md`.

## F-D03-040 — Matter inbox triage & auto-assignment

- **Business objective:** Reduce risk, cycle time, and rework for 'matter inbox triage & auto-assignment' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter inbox triage & auto-assignment scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter inbox triage & auto-assignment' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d03_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d03_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d03/items`, `GET/PUT /api/v1/d03/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d03:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D03-040` entries in `expansions/D03-expansions.md`.

---

## Cross-domain links (audit-generated, planning-level)

Candidate integration contracts. Direction OUT = this feature feeds; IN = this feature consumes.
Confirm payloads/events in detailed design (see 27-CROSS-DOMAIN-VALIDATION).

### F-D03-001 — Matter / project creation wizard
- IN ← F-D02-008 (Conflict-of-interest check) [gates]
- IN ← F-D02-014 (Engagement letter management) [spawns-matter-via]

### F-D03-008 — Deadline & SLA engine
- OUT → F-D18-012 (Deadline reminder engine) [emits-event-to]
- IN ← F-D18-012 (Deadline reminder engine) [watches]
- IN ← F-D20-013 (Deadline compliance report) [measures]

### F-D03-013 — Time entry (manual)
- OUT → F-D15-002 (Time-based billing rules) [billable-source-for]

### F-D03-015 — Timesheet approval
- OUT → F-D09-001 (Approval request composer) [routes-to]

### F-D03-016 — Matter budget vs actual
- OUT → F-D14-014 (Matter cost allocation) [budget-basis-for]

### F-D03-018 — Expense attachment to matter
- OUT → F-D14-029 (Reimbursement workflow) [claims-via]

### F-D03-023 — Related-matter linking
- IN ← F-D17-007 (Inter-office matter referral) [links]

### F-D03-033 — Client matter statement
- OUT → F-D19-004 (Matter progress view) [published-to]
- IN ← F-D19-004 (Matter progress view) [renders]

