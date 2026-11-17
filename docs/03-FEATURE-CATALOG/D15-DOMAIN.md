# D15 — Billing & Subscription

Domain 15 of 25. 40 primary features. All IDs unique. Planning-only; no implementation claimed.

## Feature index

- F-D15-001 — Fee-schedule library
- F-D15-002 — Time-based billing rules
- F-D15-003 — Fixed-fee billing
- F-D15-004 — Success-fee billing
- F-D15-005 — Retainer billing
- F-D15-006 — Subscription plan catalog
- F-D15-007 — Plan upgrade / downgrade
- F-D15-008 — Proration engine
- F-D15-009 — Invoice composer
- F-D15-010 — E-invoice data capture
- F-D15-011 — Invoice approval workflow
- F-D15-012 — Invoice delivery (email/portal)
- F-D15-013 — Invoice reminder schedule
- F-D15-014 — Late-fee & denda policy
- F-D15-015 — Payment recording (multi-method)
- F-D15-016 — Payment-gateway integration adapter
- F-D15-017 — Virtual-account billing
- F-D15-018 — QRIS payment support
- F-D15-019 — Partial-payment handling
- F-D15-020 — Overpayment / credit-memo
- F-D15-021 — Refund workflow
- F-D15-022 — Dunning workflow
- F-D15-023 — Collection task assignment
- F-D15-024 — Billing dispute handling
- F-D15-025 — Client billing profile
- F-D15-026 — Discount & voucher engine
- F-D15-027 — Tax on invoice (PPN/PPh)
- F-D15-028 — Invoice cancellation (credit note)
- F-D15-029 — Billing dashboard
- F-D15-030 — AR aging report
- F-D15-031 — Revenue recognition hook
- F-D15-032 — Payout to partner (profit share)
- F-D15-033 — Office-level revenue split
- F-D15-034 — Billing audit trail
- F-D15-035 — Invoice PDF rendering
- F-D15-036 — Payment receipt rendering
- F-D15-037 — Subscription renewal scheduler
- F-D15-038 — Churn / win-back tracker
- F-D15-039 — Billing API for integrations
- F-D15-040 — Billing data retention

---

## F-D15-001 — Fee-schedule library

- **Business objective:** Reduce risk, cycle time, and rework for 'fee-schedule library' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for fee-schedule library scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'fee-schedule library' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-001` entries in `expansions/D15-expansions.md`.

## F-D15-002 — Time-based billing rules

- **Business objective:** Reduce risk, cycle time, and rework for 'time-based billing rules' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for time-based billing rules scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'time-based billing rules' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-002` entries in `expansions/D15-expansions.md`.

## F-D15-003 — Fixed-fee billing

- **Business objective:** Reduce risk, cycle time, and rework for 'fixed-fee billing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for fixed-fee billing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'fixed-fee billing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-003` entries in `expansions/D15-expansions.md`.

## F-D15-004 — Success-fee billing

- **Business objective:** Reduce risk, cycle time, and rework for 'success-fee billing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for success-fee billing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'success-fee billing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-004` entries in `expansions/D15-expansions.md`.

## F-D15-005 — Retainer billing

- **Business objective:** Reduce risk, cycle time, and rework for 'retainer billing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for retainer billing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'retainer billing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-005` entries in `expansions/D15-expansions.md`.

## F-D15-006 — Subscription plan catalog

- **Business objective:** Reduce risk, cycle time, and rework for 'subscription plan catalog' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for subscription plan catalog scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'subscription plan catalog' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-006` entries in `expansions/D15-expansions.md`.

## F-D15-007 — Plan upgrade / downgrade

- **Business objective:** Reduce risk, cycle time, and rework for 'plan upgrade / downgrade' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for plan upgrade / downgrade scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'plan upgrade / downgrade' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-007` entries in `expansions/D15-expansions.md`.

## F-D15-008 — Proration engine

- **Business objective:** Reduce risk, cycle time, and rework for 'proration engine' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for proration engine scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'proration engine' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-008` entries in `expansions/D15-expansions.md`.

## F-D15-009 — Invoice composer

- **Business objective:** Reduce risk, cycle time, and rework for 'invoice composer' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for invoice composer scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'invoice composer' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-009` entries in `expansions/D15-expansions.md`.

## F-D15-010 — E-invoice data capture

- **Business objective:** Reduce risk, cycle time, and rework for 'e-invoice data capture' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for e-invoice data capture scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'e-invoice data capture' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-010` entries in `expansions/D15-expansions.md`.

## F-D15-011 — Invoice approval workflow

- **Business objective:** Reduce risk, cycle time, and rework for 'invoice approval workflow' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for invoice approval workflow scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'invoice approval workflow' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-011` entries in `expansions/D15-expansions.md`.

## F-D15-012 — Invoice delivery (email/portal)

- **Business objective:** Reduce risk, cycle time, and rework for 'invoice delivery (email/portal)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for invoice delivery (email/portal) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'invoice delivery (email/portal)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-012` entries in `expansions/D15-expansions.md`.

## F-D15-013 — Invoice reminder schedule

- **Business objective:** Reduce risk, cycle time, and rework for 'invoice reminder schedule' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for invoice reminder schedule scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'invoice reminder schedule' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-013` entries in `expansions/D15-expansions.md`.

## F-D15-014 — Late-fee & denda policy

- **Business objective:** Reduce risk, cycle time, and rework for 'late-fee & denda policy' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for late-fee & denda policy scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'late-fee & denda policy' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-014` entries in `expansions/D15-expansions.md`.

## F-D15-015 — Payment recording (multi-method)

- **Business objective:** Reduce risk, cycle time, and rework for 'payment recording (multi-method)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for payment recording (multi-method) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'payment recording (multi-method)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-015` entries in `expansions/D15-expansions.md`.

## F-D15-016 — Payment-gateway integration adapter

- **Business objective:** Reduce risk, cycle time, and rework for 'payment-gateway integration adapter' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for payment-gateway integration adapter scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'payment-gateway integration adapter' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-016` entries in `expansions/D15-expansions.md`.

## F-D15-017 — Virtual-account billing

- **Business objective:** Reduce risk, cycle time, and rework for 'virtual-account billing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for virtual-account billing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'virtual-account billing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-017` entries in `expansions/D15-expansions.md`.

## F-D15-018 — QRIS payment support

- **Business objective:** Reduce risk, cycle time, and rework for 'qris payment support' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for qris payment support scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'qris payment support' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-018` entries in `expansions/D15-expansions.md`.

## F-D15-019 — Partial-payment handling

- **Business objective:** Reduce risk, cycle time, and rework for 'partial-payment handling' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for partial-payment handling scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'partial-payment handling' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-019` entries in `expansions/D15-expansions.md`.

## F-D15-020 — Overpayment / credit-memo

- **Business objective:** Reduce risk, cycle time, and rework for 'overpayment / credit-memo' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for overpayment / credit-memo scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'overpayment / credit-memo' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-020` entries in `expansions/D15-expansions.md`.

## F-D15-021 — Refund workflow

- **Business objective:** Reduce risk, cycle time, and rework for 'refund workflow' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for refund workflow scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'refund workflow' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-021` entries in `expansions/D15-expansions.md`.

## F-D15-022 — Dunning workflow

- **Business objective:** Reduce risk, cycle time, and rework for 'dunning workflow' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for dunning workflow scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'dunning workflow' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-022` entries in `expansions/D15-expansions.md`.

## F-D15-023 — Collection task assignment

- **Business objective:** Reduce risk, cycle time, and rework for 'collection task assignment' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for collection task assignment scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'collection task assignment' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-023` entries in `expansions/D15-expansions.md`.

## F-D15-024 — Billing dispute handling

- **Business objective:** Reduce risk, cycle time, and rework for 'billing dispute handling' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for billing dispute handling scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'billing dispute handling' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-024` entries in `expansions/D15-expansions.md`.

## F-D15-025 — Client billing profile

- **Business objective:** Reduce risk, cycle time, and rework for 'client billing profile' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for client billing profile scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'client billing profile' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-025` entries in `expansions/D15-expansions.md`.

## F-D15-026 — Discount & voucher engine

- **Business objective:** Reduce risk, cycle time, and rework for 'discount & voucher engine' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for discount & voucher engine scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'discount & voucher engine' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-026` entries in `expansions/D15-expansions.md`.

## F-D15-027 — Tax on invoice (PPN/PPh)

- **Business objective:** Reduce risk, cycle time, and rework for 'tax on invoice (ppn/pph)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for tax on invoice (ppn/pph) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'tax on invoice (ppn/pph)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-027` entries in `expansions/D15-expansions.md`.

## F-D15-028 — Invoice cancellation (credit note)

- **Business objective:** Reduce risk, cycle time, and rework for 'invoice cancellation (credit note)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for invoice cancellation (credit note) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'invoice cancellation (credit note)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-028` entries in `expansions/D15-expansions.md`.

## F-D15-029 — Billing dashboard

- **Business objective:** Reduce risk, cycle time, and rework for 'billing dashboard' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for billing dashboard scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'billing dashboard' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-029` entries in `expansions/D15-expansions.md`.

## F-D15-030 — AR aging report

- **Business objective:** Reduce risk, cycle time, and rework for 'ar aging report' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for ar aging report scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'ar aging report' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-030` entries in `expansions/D15-expansions.md`.

## F-D15-031 — Revenue recognition hook

- **Business objective:** Reduce risk, cycle time, and rework for 'revenue recognition hook' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for revenue recognition hook scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'revenue recognition hook' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-031` entries in `expansions/D15-expansions.md`.

## F-D15-032 — Payout to partner (profit share)

- **Business objective:** Reduce risk, cycle time, and rework for 'payout to partner (profit share)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for payout to partner (profit share) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'payout to partner (profit share)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-032` entries in `expansions/D15-expansions.md`.

## F-D15-033 — Office-level revenue split

- **Business objective:** Reduce risk, cycle time, and rework for 'office-level revenue split' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for office-level revenue split scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'office-level revenue split' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-033` entries in `expansions/D15-expansions.md`.

## F-D15-034 — Billing audit trail

- **Business objective:** Reduce risk, cycle time, and rework for 'billing audit trail' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for billing audit trail scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'billing audit trail' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-034` entries in `expansions/D15-expansions.md`.

## F-D15-035 — Invoice PDF rendering

- **Business objective:** Reduce risk, cycle time, and rework for 'invoice pdf rendering' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for invoice pdf rendering scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'invoice pdf rendering' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-035` entries in `expansions/D15-expansions.md`.

## F-D15-036 — Payment receipt rendering

- **Business objective:** Reduce risk, cycle time, and rework for 'payment receipt rendering' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for payment receipt rendering scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'payment receipt rendering' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D15-036` entries in `expansions/D15-expansions.md`.

## F-D15-037 — Subscription renewal scheduler

- **Business objective:** Reduce risk, cycle time, and rework for 'subscription renewal scheduler' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for subscription renewal scheduler scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'subscription renewal scheduler' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-037` entries in `expansions/D15-expansions.md`.

## F-D15-038 — Churn / win-back tracker

- **Business objective:** Reduce risk, cycle time, and rework for 'churn / win-back tracker' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for churn / win-back tracker scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'churn / win-back tracker' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D15-038` entries in `expansions/D15-expansions.md`.

## F-D15-039 — Billing API for integrations

- **Business objective:** Reduce risk, cycle time, and rework for 'billing api for integrations' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for billing api for integrations scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'billing api for integrations' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D15-039` entries in `expansions/D15-expansions.md`.

## F-D15-040 — Billing data retention

- **Business objective:** Reduce risk, cycle time, and rework for 'billing data retention' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for billing data retention scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'billing data retention' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d15_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d15_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d15/items`, `GET/PUT /api/v1/d15/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d15:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D15-040` entries in `expansions/D15-expansions.md`.

---

## Cross-domain links (audit-generated, planning-level)

Candidate integration contracts. Direction OUT = this feature feeds; IN = this feature consumes.
Confirm payloads/events in detailed design (see 27-CROSS-DOMAIN-VALIDATION).

### F-D15-001 — Fee-schedule library
- IN ← F-D06-035 (Deed fee computation hook) [prices-from]

### F-D15-002 — Time-based billing rules
- IN ← F-D03-013 (Time entry (manual)) [billable-source-for]

### F-D15-010 — E-invoice data capture
- OUT → F-D21-023 (E-faktur export adapter) [exports-via]

### F-D15-011 — Invoice approval workflow
- OUT → F-D09-001 (Approval request composer) [routes-to]

### F-D15-015 — Payment recording (multi-method)
- OUT → F-D19-010 (Online payment checkout) [settles]
- IN ← F-D14-011 (Client trust (escrow) ledger) [settles-into]

### F-D15-037 — Subscription renewal scheduler
- OUT → F-D18-015 (Payment notifications) [notified-by]

