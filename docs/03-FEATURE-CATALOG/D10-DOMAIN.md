# D10 — Electronic Signature

Domain 10 of 25. 40 primary features. All IDs unique. Planning-only; no implementation claimed.

## Feature index

- F-D10-001 — PSrE provider adapter (certified)
- F-D10-002 — Uncertified e-sign path (low-risk)
- F-D10-003 — Signature order orchestration
- F-D10-004 — Sequential signing
- F-D10-005 — Parallel signing
- F-D10-006 — Signer identity binding (KYC)
- F-D10-007 — Signer OTP challenge
- F-D10-008 — Signer consent capture
- F-D10-009 — Document hash sealing (pre-sign)
- F-D10-010 — Long-term validation (LTV) artifacts
- F-D10-011 — Timestamp authority (TSA) binding
- F-D10-012 — Signature appearance placement
- F-D10-013 — Initials on every page option
- F-D10-014 — Bulk signing session
- F-D10-015 — Signing link via secure URL
- F-D10-016 — Signing reminder schedule
- F-D10-017 — Signer decline handling
- F-D10-018 — Signer reassignment
- F-D10-019 — Document preview before signing
- F-D10-020 — Post-sign tamper evidence
- F-D10-021 — Signed-document vault
- F-D10-022 — Signature verification page
- F-D10-023 — Verification QR code
- F-D10-024 — Revocation / invalidation log
- F-D10-025 — Provider failover routing
- F-D10-026 — E-sign cost tracking
- F-D10-027 — E-sign regulatory guardrail (akta exclusion)
- F-D10-028 — Notarial act exclusion warning
- F-D10-029 — Signing ceremony log
- F-D10-030 — Signer accessibility support
- F-D10-031 — E-sign webhook callbacks
- F-D10-032 — E-sign audit trail
- F-D10-033 — E-sign retention policy
- F-D10-034 — Provider credential vault
- F-D10-035 — E-sign dashboard
- F-D10-036 — Sandbox vs production provider mode
- F-D10-037 — Signature font/hand-drawn capture
- F-D10-038 — Signer device & IP logging
- F-D10-039 — Multi-document signing bundle
- F-D10-040 — Co-signer & witness signing flow

---

## F-D10-001 — PSrE provider adapter (certified)

- **Business objective:** Reduce risk, cycle time, and rework for 'psre provider adapter (certified)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for psre provider adapter (certified) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'psre provider adapter (certified)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-001` entries in `expansions/D10-expansions.md`.

## F-D10-002 — Uncertified e-sign path (low-risk)

- **Business objective:** Reduce risk, cycle time, and rework for 'uncertified e-sign path (low-risk)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for uncertified e-sign path (low-risk) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'uncertified e-sign path (low-risk)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-002` entries in `expansions/D10-expansions.md`.

## F-D10-003 — Signature order orchestration

- **Business objective:** Reduce risk, cycle time, and rework for 'signature order orchestration' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signature order orchestration scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signature order orchestration' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-003` entries in `expansions/D10-expansions.md`.

## F-D10-004 — Sequential signing

- **Business objective:** Reduce risk, cycle time, and rework for 'sequential signing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for sequential signing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'sequential signing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-004` entries in `expansions/D10-expansions.md`.

## F-D10-005 — Parallel signing

- **Business objective:** Reduce risk, cycle time, and rework for 'parallel signing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for parallel signing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'parallel signing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-005` entries in `expansions/D10-expansions.md`.

## F-D10-006 — Signer identity binding (KYC)

- **Business objective:** Reduce risk, cycle time, and rework for 'signer identity binding (kyc)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signer identity binding (kyc) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signer identity binding (kyc)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D10-006` entries in `expansions/D10-expansions.md`.

## F-D10-007 — Signer OTP challenge

- **Business objective:** Reduce risk, cycle time, and rework for 'signer otp challenge' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signer otp challenge scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signer otp challenge' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D10-007` entries in `expansions/D10-expansions.md`.

## F-D10-008 — Signer consent capture

- **Business objective:** Reduce risk, cycle time, and rework for 'signer consent capture' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signer consent capture scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signer consent capture' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-008` entries in `expansions/D10-expansions.md`.

## F-D10-009 — Document hash sealing (pre-sign)

- **Business objective:** Reduce risk, cycle time, and rework for 'document hash sealing (pre-sign)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for document hash sealing (pre-sign) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'document hash sealing (pre-sign)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-009` entries in `expansions/D10-expansions.md`.

## F-D10-010 — Long-term validation (LTV) artifacts

- **Business objective:** Reduce risk, cycle time, and rework for 'long-term validation (ltv) artifacts' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for long-term validation (ltv) artifacts scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'long-term validation (ltv) artifacts' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D10-010` entries in `expansions/D10-expansions.md`.

## F-D10-011 — Timestamp authority (TSA) binding

- **Business objective:** Reduce risk, cycle time, and rework for 'timestamp authority (tsa) binding' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for timestamp authority (tsa) binding scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'timestamp authority (tsa) binding' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-011` entries in `expansions/D10-expansions.md`.

## F-D10-012 — Signature appearance placement

- **Business objective:** Reduce risk, cycle time, and rework for 'signature appearance placement' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signature appearance placement scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signature appearance placement' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-012` entries in `expansions/D10-expansions.md`.

## F-D10-013 — Initials on every page option

- **Business objective:** Reduce risk, cycle time, and rework for 'initials on every page option' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for initials on every page option scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'initials on every page option' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-013` entries in `expansions/D10-expansions.md`.

## F-D10-014 — Bulk signing session

- **Business objective:** Reduce risk, cycle time, and rework for 'bulk signing session' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for bulk signing session scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'bulk signing session' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-014` entries in `expansions/D10-expansions.md`.

## F-D10-015 — Signing link via secure URL

- **Business objective:** Reduce risk, cycle time, and rework for 'signing link via secure url' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signing link via secure url scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signing link via secure url' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-015` entries in `expansions/D10-expansions.md`.

## F-D10-016 — Signing reminder schedule

- **Business objective:** Reduce risk, cycle time, and rework for 'signing reminder schedule' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signing reminder schedule scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signing reminder schedule' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D10-016` entries in `expansions/D10-expansions.md`.

## F-D10-017 — Signer decline handling

- **Business objective:** Reduce risk, cycle time, and rework for 'signer decline handling' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signer decline handling scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signer decline handling' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-017` entries in `expansions/D10-expansions.md`.

## F-D10-018 — Signer reassignment

- **Business objective:** Reduce risk, cycle time, and rework for 'signer reassignment' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signer reassignment scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signer reassignment' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-018` entries in `expansions/D10-expansions.md`.

## F-D10-019 — Document preview before signing

- **Business objective:** Reduce risk, cycle time, and rework for 'document preview before signing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for document preview before signing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'document preview before signing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-019` entries in `expansions/D10-expansions.md`.

## F-D10-020 — Post-sign tamper evidence

- **Business objective:** Reduce risk, cycle time, and rework for 'post-sign tamper evidence' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for post-sign tamper evidence scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'post-sign tamper evidence' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-020` entries in `expansions/D10-expansions.md`.

## F-D10-021 — Signed-document vault

- **Business objective:** Reduce risk, cycle time, and rework for 'signed-document vault' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signed-document vault scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signed-document vault' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-021` entries in `expansions/D10-expansions.md`.

## F-D10-022 — Signature verification page

- **Business objective:** Reduce risk, cycle time, and rework for 'signature verification page' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signature verification page scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signature verification page' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-022` entries in `expansions/D10-expansions.md`.

## F-D10-023 — Verification QR code

- **Business objective:** Reduce risk, cycle time, and rework for 'verification qr code' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for verification qr code scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'verification qr code' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-023` entries in `expansions/D10-expansions.md`.

## F-D10-024 — Revocation / invalidation log

- **Business objective:** Reduce risk, cycle time, and rework for 'revocation / invalidation log' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for revocation / invalidation log scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'revocation / invalidation log' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-024` entries in `expansions/D10-expansions.md`.

## F-D10-025 — Provider failover routing

- **Business objective:** Reduce risk, cycle time, and rework for 'provider failover routing' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for provider failover routing scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'provider failover routing' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D10-025` entries in `expansions/D10-expansions.md`.

## F-D10-026 — E-sign cost tracking

- **Business objective:** Reduce risk, cycle time, and rework for 'e-sign cost tracking' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for e-sign cost tracking scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'e-sign cost tracking' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-026` entries in `expansions/D10-expansions.md`.

## F-D10-027 — E-sign regulatory guardrail (akta exclusion)

- **Business objective:** Reduce risk, cycle time, and rework for 'e-sign regulatory guardrail (akta exclusion)' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for e-sign regulatory guardrail (akta exclusion) scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'e-sign regulatory guardrail (akta exclusion)' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-027` entries in `expansions/D10-expansions.md`.

## F-D10-028 — Notarial act exclusion warning

- **Business objective:** Reduce risk, cycle time, and rework for 'notarial act exclusion warning' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for notarial act exclusion warning scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'notarial act exclusion warning' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-028` entries in `expansions/D10-expansions.md`.

## F-D10-029 — Signing ceremony log

- **Business objective:** Reduce risk, cycle time, and rework for 'signing ceremony log' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signing ceremony log scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signing ceremony log' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-029` entries in `expansions/D10-expansions.md`.

## F-D10-030 — Signer accessibility support

- **Business objective:** Reduce risk, cycle time, and rework for 'signer accessibility support' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signer accessibility support scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signer accessibility support' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-030` entries in `expansions/D10-expansions.md`.

## F-D10-031 — E-sign webhook callbacks

- **Business objective:** Reduce risk, cycle time, and rework for 'e-sign webhook callbacks' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for e-sign webhook callbacks scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'e-sign webhook callbacks' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** S. **Estimated development effort:** 3-5 d.
- **Future opportunities:** see `F-D10-031` entries in `expansions/D10-expansions.md`.

## F-D10-032 — E-sign audit trail

- **Business objective:** Reduce risk, cycle time, and rework for 'e-sign audit trail' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for e-sign audit trail scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'e-sign audit trail' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-032` entries in `expansions/D10-expansions.md`.

## F-D10-033 — E-sign retention policy

- **Business objective:** Reduce risk, cycle time, and rework for 'e-sign retention policy' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for e-sign retention policy scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'e-sign retention policy' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** L. **Estimated development effort:** 3-5 w.
- **Future opportunities:** see `F-D10-033` entries in `expansions/D10-expansions.md`.

## F-D10-034 — Provider credential vault

- **Business objective:** Reduce risk, cycle time, and rework for 'provider credential vault' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for provider credential vault scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'provider credential vault' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-034` entries in `expansions/D10-expansions.md`.

## F-D10-035 — E-sign dashboard

- **Business objective:** Reduce risk, cycle time, and rework for 'e-sign dashboard' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for e-sign dashboard scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'e-sign dashboard' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** XL. **Estimated development effort:** 6-10 w.
- **Future opportunities:** see `F-D10-035` entries in `expansions/D10-expansions.md`.

## F-D10-036 — Sandbox vs production provider mode

- **Business objective:** Reduce risk, cycle time, and rework for 'sandbox vs production provider mode' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for sandbox vs production provider mode scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'sandbox vs production provider mode' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-036` entries in `expansions/D10-expansions.md`.

## F-D10-037 — Signature font/hand-drawn capture

- **Business objective:** Reduce risk, cycle time, and rework for 'signature font/hand-drawn capture' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signature font/hand-drawn capture scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signature font/hand-drawn capture' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P0-Critical. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-037` entries in `expansions/D10-expansions.md`.

## F-D10-038 — Signer device & IP logging

- **Business objective:** Reduce risk, cycle time, and rework for 'signer device & ip logging' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for signer device & ip logging scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'signer device & ip logging' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P2-Medium. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-038` entries in `expansions/D10-expansions.md`.

## F-D10-039 — Multi-document signing bundle

- **Business objective:** Reduce risk, cycle time, and rework for 'multi-document signing bundle' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for multi-document signing bundle scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'multi-document signing bundle' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P1-High. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-039` entries in `expansions/D10-expansions.md`.

## F-D10-040 — Co-signer & witness signing flow

- **Business objective:** Reduce risk, cycle time, and rework for 'co-signer & witness signing flow' while meeting Indonesian regulatory expectations.
- **Description:** The system provides a governed, auditable capability for co-signer & witness signing flow scoped by tenant/office, role permissions, and full event history. Rust backend (Axum/SQLx), Leptos UI, MySQL 8.4, S3/MinIO artifacts, NATS events, Redis cache, OTel traces.
- **Target user:** Matter handler / office staff; supervisors; auditors.
- **User stories:** (1) As Matter handler / office staff, I can complete 'co-signer & witness signing flow' without leaving the matter workspace. (2) As a supervisor, I can review, approve, or return the work with reasons. (3) As an auditor, I can replay who did what, when, with evidence.
- **Functional requirements:** FR1 create/update with validation; FR2 state machine (draft→in-review→approved/returned→closed); FR3 attachments with integrity hash; FR4 SLA timers + reminders; FR5 search/filter; FR6 export (controlled).
- **Non-functional requirements:** p95 API < 400ms (excl. OCR/AI); availability 99.5% modular-monolith; WCAG 2.2 AA for UI; idempotent writes; OTel instrumented.
- **Business rules:** BR1 tenant/office scoping mandatory; BR2 PII masking by role; BR3 no deletion under legal hold; BR4 financial/deed actions need approval; BR5 UU PDP consent respected.
- **Subfeatures:** SF1 guided form + validation; SF2 attachment manager; SF3 reminder/SLA config; SF4 review & approval hook; SF5 activity timeline.
- **Database entities:** `d10_item` (tenant_id, office_id, matter_id, status, payload JSON, hashes), `d10_event` (append-only history), links to `tenants/offices/matters/documents/users`.
- **API requirements:** `GET/POST /api/v1/d10/items`, `GET/PUT /api/v1/d10/items/:id`, `POST .../:id/transition`, idempotency-key, cursor pagination, RBAC + scopes.
- **UI screens:** SCR1 list + filters; SCR2 detail workspace; SCR3 guided create/edit; SCR4 review/approve panel; SCR5 history/audit tab.
- **Permission requirements:** `perm:d10:read/write/approve/export`; office-scope enforcement; break-glass logged.
- **Security considerations:** AuthN+MFA; RBAC+scope; AES-256 at rest; TLS 1.2+; PII redaction in logs; rate limits; upload AV-scan where files apply.
- **Audit requirements:** Append-only event log (actor, action, timestamp, IP, before/after hash); 10-year retention default; exportable evidence pack.
- **Dependencies:** Identity/auth, tenant/office scope, document store, notification, audit. OCR/AI hooks only where stated.
- **Acceptance criteria:** AC1 happy-path completes with valid data; AC2 invalid data rejected with field errors; AC3 unauthorized access denied; AC4 state transitions validated; AC5 audit entries complete and exportable.
- **Positive test cases:** TC-P1 valid create→approve→close; TC-P2 search/filter finds record; TC-P3 export with permission succeeds.
- **Negative test cases:** TC-N1 invalid payload → 422; TC-N2 unauthorized → 403; TC-N3 illegal transition → 409.
- **Edge cases:** EC1 duplicate submit (idempotency); EC2 large attachment; EC3 concurrent edits (optimistic locking); EC4 retention vs deletion conflict.
- **Priority:** P3-Low. **Complexity:** M. **Estimated development effort:** 1-2 w.
- **Future opportunities:** see `F-D10-040` entries in `expansions/D10-expansions.md`.

---

## Cross-domain links (audit-generated, planning-level)

Candidate integration contracts. Direction OUT = this feature feeds; IN = this feature consumes.
Confirm payloads/events in detailed design (see 27-CROSS-DOMAIN-VALIDATION).

### F-D10-001 — PSrE provider adapter (certified)
- OUT → F-D21-016 (PSrE e-sign adapter) [connects-via]

### F-D10-003 — Signature order orchestration
- IN ← F-D09-029 (E-signature binding on approval) [binds-to]

### F-D10-006 — Signer identity binding (KYC)
- OUT → F-D01-006 (Biometric face match (selfie vs ID)) [verified-via]

### F-D10-015 — Signing link via secure URL
- IN ← F-D08-018 (E-signature step connector) [hands-off-to]

### F-D10-016 — Signing reminder schedule
- OUT → F-D18-014 (E-sign event notifications) [notifies-via]

### F-D10-021 — Signed-document vault
- OUT → F-D04-009 (Document archiving) [archives-to]

### F-D10-031 — E-sign webhook callbacks
- OUT → F-D21-009 (Inbound webhook receiver) [publishes-to]

