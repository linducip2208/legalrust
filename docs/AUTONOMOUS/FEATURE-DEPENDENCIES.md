# Feature Dependency Order (25 domains -> implementation waves)

Rule: a domain ships after the domains it reads/writes. Auth/RBAC/tenancy first,
case core second, provider-gated transactional features after their queues exist,
analytical/portal/mobile layers last. Status context from FEATURE-REGISTRY.json.

## Wave 0 - Foundation (already live core, harden first)

- **D16 User/Role/Permission** - IMPLEMENTED core: users, roles, permissions, tokens, lockout. Precedes everything: every endpoint authorizes against it. Remaining: MFA/SSO (CONFIG), lifecycle workflows (PLANNED).
- **D22 Security & Data Protection** - IMPLEMENTED core: Argon2, JWT sessions, rate-limit, audit. Precedes everything. Remaining: KMS/HSM, WAF/SAST/DAST process work.
- **D17 Multi-Office & Tenant** - PARTIAL: single-org scoping live. Harden before cross-office features: branch registry, residency, quotas.
- **D21 API & Integration** - PARTIAL: versioned REST, tokens, inbound webhooks. Defines contracts (OpenAPI, idempotency, sandbox) that Waves 1-4 build on.

## Wave 1 - Case core (mostly live, complete the gaps)

- **D02 Client & CRM** - IMPLEMENTED master records + lead pipeline. Needs: D16, D21. Gaps: onboarding checklists, hierarchy, surveys (PLANNED).
- **D03 Project & Case Management** - IMPLEMENTED matters/tasks w/ guards. Needs: D02, D04, D18. Gaps: time tracking, budgets, kanban/gantt (PLANNED/NOT_STARTED).
- **D04 Document Management** - IMPLEMENTED upload/version/SHA-pin/shares. Needs: D03, D22. Gaps: retention/legal-hold/trash/virus-scan (PLANNED/CONFIG).
- **D18 Communication & Notification** - IMPLEMENTED in-app + SSE. Needs: D03 (matter events). Gaps: e-mail/SMS/WA providers (CONFIG), preferences/digests (PLANNED).
- **D23 Search & Discovery** - PARTIAL keyword search (global + RAG text-only). Needs: D04, D06. Gaps: OCR-text/faceted/vector (CONFIG), analytics (PLANNED).

## Wave 2 - Transactional (provider-gated; queues exist, engines pending)

- **D05 OCR & Document Intelligence** - Intake + review live; extraction 501 until engine configured. Needs: D04, D21 (engine plugin API). Unblocks D01, D13.
- **D06 Deed Generation** - PARTIAL: templates + render + guarded transitions live; signing 501. Needs: D03, D04, D02 (parties). Unblocks D10, D12, D13.
- **D09 Approval Management** - No generic engine yet (PLANNED); deed transitions cover the pattern. Needs: D06, D08, D18. Precedes D10 signing order.
- **D10 Electronic Signature** - Blocked on PSrE provider (501). Needs: D06 (documents to sign), D09 (approval evidence), D01 (signer KYC).
- **D14 Finance & Accounting** - IMPLEMENTED trust ledger + manual rails. Needs: D03 (matter costing). Gaps: GL/journal/tax (PLANNED).
- **D15 Billing & Subscription** - IMPLEMENTED manual pay + anti-overpay; gateway 501. Needs: D14, D03. Gaps: subscriptions/dunning/tax (PLANNED/CONFIG).

## Wave 3 - Domain depth (needs Waves 0-2)

- **D01 Identity & KYC** - Needs: D05 (extraction), D21 (Dukcapil/sanctions adapters), D04 (evidence vault). Intake types already accepted by OCR queue.
- **D12 Notary Office Management** - Needs: D06 (deeds), D03 (dispatch), D14 (cash book). Mostly operational checklists (PLANNED).
- **D13 PPAT & Property Legal** - Needs: D06 (AJB/APHT render), D05 (certificate OCR), D21 (BPN adapter). Mostly PLANNED worksheets/registers.
- **D11 Compliance & Risk** - Needs: D01 (KYC), D04 (retention), D22 (controls), D20 (posture reports). Largely PLANNED workflows.
- **D07 AI Legal Assistant** - Blocked on AI provider (501); metering + keyword RAG live. Needs: D04, D23, D05 (OCR text).
- **D08 Workflow Automation** - Job queue primitive live; designer/engine missing. Needs: D09, D10, D05, D18 (connectors), D21 (NATS binding decision).

## Wave 4 - Experience & scale (needs everything above)

- **D19 Client & Public Portal** - Needs: D02/D03/D04 (views), D06 (extracts), D10 (signing entry, CONFIG), D15 (checkout, CONFIG). Public forms/pricing/FAQ already live.
- **D20 Reporting & Analytics** - CSV + RBAC + audit live. Needs: per-domain event coverage from Waves 1-3 before dashboards/KPIs are meaningful.
- **D24 Mobile & Field Operations** - NOT_STARTED by design (sync-core in-memory only). Needs: D03/D04 offline queue, D05 image QC, D10 mobile signing; decide Dioxus vs PWA first.
- **D25 Enterprise Admin & Global Expansion** - Needs: Waves 0-3 stable (quotas, retention, backup, i18n all PLANNED/NOT_STARTED). Ship last.

## Cross-domain edges (selected)
- D06 -> D04 (deed PDFs archived as versioned docs), D03 (matter-bound), D02 (party data)
- D05 -> D04 (OCR queue trigger), D01/D13 (ID + certificate extraction)
- D10 -> D06 (rendered deed is the signing payload), D09 (approval before sign), D01 (signer identity)
- D09 -> D08 (approval steps), D06 (deed approval), D15 (invoice approval)
- D14/D15 -> D03 (per-matter invoices/costs), D19 (client-visible billing)
- D07 -> D04/D23 (grounding corpus), D05 (OCR text), D22 (PII redaction, audit)
- D08 -> D09/D10/D05/D14 (step connectors), D18 (notifications), D21 (webhooks/NATS)
- D01 -> D05/D21 (extraction + verification providers), D11 (CDD/EDD evidence)
- D11 -> D22/D04/D20 (controls, retention, posture reporting)
- D19 -> D02/D03/D04/D06/D10/D15 (portal mirrors core objects; no separate store)
- D20 -> all (read-only rollups; must not create a second source of truth)
- D24 -> D03/D04/D05/D18 (offline-first queues of the same primitives)
- D12/D13 -> D06/D05 (deed + certificate pipelines), D02 (party/KYC bundles)

