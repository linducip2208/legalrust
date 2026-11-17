# D20 — Reporting & Analytics

Domain 20 of 25. 40 primary features. All IDs unique. Planning-only; no implementation claimed.

## Feature index

- F-D20-001 — Standard report library
- F-D20-002 — Ad-hoc report builder
- F-D20-003 — Report scheduling (email)
- F-D20-004 — Report export (PDF/XLSX/CSV)
- F-D20-005 — Dashboard designer
- F-D20-006 — Role-based dashboards
- F-D20-007 — Matter KPI dashboard
- F-D20-008 — Financial KPI dashboard
- F-D20-009 — Deed-volume period report & export
- F-D20-010 — OCR accuracy trend & benchmark report
- F-D20-011 — AI cost & quality executive report
- F-D20-012 — Approval bottleneck report
- F-D20-013 — Deadline compliance report
- F-D20-014 — Client profitability report
- F-D20-015 — Staff utilization report
- F-D20-016 — Pipeline & forecast report
- F-D20-017 — Compliance posture report
- F-D20-018 — Risk posture period report
- F-D20-019 — Data-quality scorecard
- F-D20-020 — Report access control
- F-D20-021 — Report version history
- F-D20-022 — Report annotation
- F-D20-023 — Drill-down navigation
- F-D20-024 — Comparative period analysis
- F-D20-025 — Office benchmarking
- F-D20-026 — Tenant-level rollups
- F-D20-027 — Scheduled snapshot archive
- F-D20-028 — Report delivery log
- F-D20-029 — Metric dictionary
- F-D20-030 — KPI alert thresholds
- F-D20-031 — Embedded analytics (portal)
- F-D20-032 — Analytics data-mart refresh
- F-D20-033 — PII handling in reports
- F-D20-034 — Report audit trail
- F-D20-035 — Report retention policy
- F-D20-036 — Custom SQL (guardrailed) reports
- F-D20-037 — Report performance monitor
- F-D20-038 — Mobile-friendly reports
- F-D20-039 — Executive summary pack
- F-D20-040 — Board-ready export theme

---

## F-D20-001 — Standard report library

- **Business objective:** Reduce risk, cycle time, and rework for 'standard report library' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for standard report library scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'standard report library' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-001` entries in `expansions/D20-expansions.md`.

## F-D20-002 — Ad-hoc report builder

- **Business objective:** Reduce risk, cycle time, and rework for 'ad-hoc report builder' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ad-hoc report builder scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'ad-hoc report builder' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-002` entries in `expansions/D20-expansions.md`.

## F-D20-003 — Report scheduling (email)

- **Business objective:** Reduce risk, cycle time, and rework for 'report scheduling (email)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report scheduling (email) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report scheduling (email)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-003` entries in `expansions/D20-expansions.md`.

## F-D20-004 — Report export (PDF/XLSX/CSV)

- **Business objective:** Reduce risk, cycle time, and rework for 'report export (pdf/xlsx/csv)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report export (pdf/xlsx/csv) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report export (pdf/xlsx/csv)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-004` entries in `expansions/D20-expansions.md`.

## F-D20-005 — Dashboard designer

- **Business objective:** Reduce risk, cycle time, and rework for 'dashboard designer' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for dashboard designer scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'dashboard designer' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-005` entries in `expansions/D20-expansions.md`.

## F-D20-006 — Role-based dashboards

- **Business objective:** Reduce risk, cycle time, and rework for 'role-based dashboards' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for role-based dashboards scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'role-based dashboards' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D20-006` entries in `expansions/D20-expansions.md`.

## F-D20-007 — Matter KPI dashboard

- **Business objective:** Reduce risk, cycle time, and rework for 'matter kpi dashboard' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for matter kpi dashboard scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'matter kpi dashboard' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D20-007` entries in `expansions/D20-expansions.md`.

## F-D20-008 — Financial KPI dashboard

- **Business objective:** Reduce risk, cycle time, and rework for 'financial kpi dashboard' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for financial kpi dashboard scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'financial kpi dashboard' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D20-008` entries in `expansions/D20-expansions.md`.

## F-D20-009 — Deed-volume period report & export

- **Business objective:** Reduce risk, cycle time, and rework for 'deed-volume period report & export' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for deed-volume period report & export scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'deed-volume period report & export' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-009` entries in `expansions/D20-expansions.md`.

## F-D20-010 — OCR accuracy trend & benchmark report

- **Business objective:** Reduce risk, cycle time, and rework for 'ocr accuracy trend & benchmark report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ocr accuracy trend & benchmark report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'ocr accuracy trend & benchmark report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-010` entries in `expansions/D20-expansions.md`.

## F-D20-011 — AI cost & quality executive report

- **Business objective:** Reduce risk, cycle time, and rework for 'ai cost & quality executive report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ai cost & quality executive report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'ai cost & quality executive report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-011` entries in `expansions/D20-expansions.md`.

## F-D20-012 — Approval bottleneck report

- **Business objective:** Reduce risk, cycle time, and rework for 'approval bottleneck report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for approval bottleneck report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'approval bottleneck report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-012` entries in `expansions/D20-expansions.md`.

## F-D20-013 — Deadline compliance report

- **Business objective:** Reduce risk, cycle time, and rework for 'deadline compliance report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for deadline compliance report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'deadline compliance report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D20-013` entries in `expansions/D20-expansions.md`.

## F-D20-014 — Client profitability report

- **Business objective:** Reduce risk, cycle time, and rework for 'client profitability report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for client profitability report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'client profitability report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-014` entries in `expansions/D20-expansions.md`.

## F-D20-015 — Staff utilization report

- **Business objective:** Reduce risk, cycle time, and rework for 'staff utilization report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for staff utilization report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'staff utilization report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-015` entries in `expansions/D20-expansions.md`.

## F-D20-016 — Pipeline & forecast report

- **Business objective:** Reduce risk, cycle time, and rework for 'pipeline & forecast report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for pipeline & forecast report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'pipeline & forecast report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-016` entries in `expansions/D20-expansions.md`.

## F-D20-017 — Compliance posture report

- **Business objective:** Reduce risk, cycle time, and rework for 'compliance posture report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for compliance posture report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'compliance posture report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-017` entries in `expansions/D20-expansions.md`.

## F-D20-018 — Risk posture period report

- **Business objective:** Reduce risk, cycle time, and rework for 'risk posture period report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for risk posture period report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'risk posture period report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-018` entries in `expansions/D20-expansions.md`.

## F-D20-019 — Data-quality scorecard

- **Business objective:** Reduce risk, cycle time, and rework for 'data-quality scorecard' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for data-quality scorecard scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'data-quality scorecard' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-019` entries in `expansions/D20-expansions.md`.

## F-D20-020 — Report access control

- **Business objective:** Reduce risk, cycle time, and rework for 'report access control' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report access control scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report access control' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-020` entries in `expansions/D20-expansions.md`.

## F-D20-021 — Report version history

- **Business objective:** Reduce risk, cycle time, and rework for 'report version history' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report version history scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report version history' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-021` entries in `expansions/D20-expansions.md`.

## F-D20-022 — Report annotation

- **Business objective:** Reduce risk, cycle time, and rework for 'report annotation' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report annotation scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report annotation' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-022` entries in `expansions/D20-expansions.md`.

## F-D20-023 — Drill-down navigation

- **Business objective:** Reduce risk, cycle time, and rework for 'drill-down navigation' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for drill-down navigation scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'drill-down navigation' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D20-023` entries in `expansions/D20-expansions.md`.

## F-D20-024 — Comparative period analysis

- **Business objective:** Reduce risk, cycle time, and rework for 'comparative period analysis' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for comparative period analysis scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'comparative period analysis' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-024` entries in `expansions/D20-expansions.md`.

## F-D20-025 — Office benchmarking

- **Business objective:** Reduce risk, cycle time, and rework for 'office benchmarking' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for office benchmarking scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'office benchmarking' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-025` entries in `expansions/D20-expansions.md`.

## F-D20-026 — Tenant-level rollups

- **Business objective:** Reduce risk, cycle time, and rework for 'tenant-level rollups' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for tenant-level rollups scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'tenant-level rollups' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-026` entries in `expansions/D20-expansions.md`.

## F-D20-027 — Scheduled snapshot archive

- **Business objective:** Reduce risk, cycle time, and rework for 'scheduled snapshot archive' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for scheduled snapshot archive scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'scheduled snapshot archive' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-027` entries in `expansions/D20-expansions.md`.

## F-D20-028 — Report delivery log

- **Business objective:** Reduce risk, cycle time, and rework for 'report delivery log' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report delivery log scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report delivery log' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-028` entries in `expansions/D20-expansions.md`.

## F-D20-029 — Metric dictionary

- **Business objective:** Reduce risk, cycle time, and rework for 'metric dictionary' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for metric dictionary scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'metric dictionary' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-029` entries in `expansions/D20-expansions.md`.

## F-D20-030 — KPI alert thresholds

- **Business objective:** Reduce risk, cycle time, and rework for 'kpi alert thresholds' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for kpi alert thresholds scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'kpi alert thresholds' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-030` entries in `expansions/D20-expansions.md`.

## F-D20-031 — Embedded analytics (portal)

- **Business objective:** Reduce risk, cycle time, and rework for 'embedded analytics (portal)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for embedded analytics (portal) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'embedded analytics (portal)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-031` entries in `expansions/D20-expansions.md`.

## F-D20-032 — Analytics data-mart refresh

- **Business objective:** Reduce risk, cycle time, and rework for 'analytics data-mart refresh' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for analytics data-mart refresh scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'analytics data-mart refresh' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-032` entries in `expansions/D20-expansions.md`.

## F-D20-033 — PII handling in reports

- **Business objective:** Reduce risk, cycle time, and rework for 'pii handling in reports' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for pii handling in reports scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'pii handling in reports' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-033` entries in `expansions/D20-expansions.md`.

## F-D20-034 — Report audit trail

- **Business objective:** Reduce risk, cycle time, and rework for 'report audit trail' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report audit trail scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report audit trail' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D20-034` entries in `expansions/D20-expansions.md`.

## F-D20-035 — Report retention policy

- **Business objective:** Reduce risk, cycle time, and rework for 'report retention policy' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report retention policy scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report retention policy' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D20-035` entries in `expansions/D20-expansions.md`.

## F-D20-036 — Custom SQL (guardrailed) reports

- **Business objective:** Reduce risk, cycle time, and rework for 'custom sql (guardrailed) reports' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for custom sql (guardrailed) reports scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'custom sql (guardrailed) reports' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D20-036` entries in `expansions/D20-expansions.md`.

## F-D20-037 — Report performance monitor

- **Business objective:** Reduce risk, cycle time, and rework for 'report performance monitor' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for report performance monitor scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'report performance monitor' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-037` entries in `expansions/D20-expansions.md`.

## F-D20-038 — Mobile-friendly reports

- **Business objective:** Reduce risk, cycle time, and rework for 'mobile-friendly reports' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for mobile-friendly reports scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'mobile-friendly reports' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-038` entries in `expansions/D20-expansions.md`.

## F-D20-039 — Executive summary pack

- **Business objective:** Reduce risk, cycle time, and rework for 'executive summary pack' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for executive summary pack scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'executive summary pack' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D20-039` entries in `expansions/D20-expansions.md`.

## F-D20-040 — Board-ready export theme

- **Business objective:** Reduce risk, cycle time, and rework for 'board-ready export theme' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for board-ready export theme scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'board-ready export theme' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d20_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d20_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d20/items`, `GET/PUT /api/v1/d20/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d20:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D20-040` entries in `expansions/D20-expansions.md`.

---

## Cross-domain links (audit-generated, planning-level)

Candidate integration contracts. Direction OUT = this feature feeds; IN = this feature consumes.
Confirm payloads/events in detailed design (see 27-CROSS-DOMAIN-VALIDATION).

### F-D20-001 — Standard report library
- IN ← F-D12-005 (Monthly notary report pack) [reported-in]

### F-D20-002 — Ad-hoc report builder
- IN ← F-D24-038 (Field-report generator) [exports-to]

### F-D20-010 — OCR accuracy trend & benchmark report
- IN ← F-D05-022 (Reviewer correction capture & learning log) [feeds-metrics]

### F-D20-012 — Approval bottleneck report
- OUT → F-D09-017 (Approval dashboard) [drills-to]
- IN ← F-D09-017 (Approval dashboard) [analyzed-in]

### F-D20-013 — Deadline compliance report
- OUT → F-D03-008 (Deadline & SLA engine) [measures]

### F-D20-020 — Report access control
- IN ← F-D16-009 (Data-scope rules (office/matter)) [constrains]

