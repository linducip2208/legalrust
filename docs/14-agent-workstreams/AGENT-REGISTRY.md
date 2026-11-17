# AGENT-REGISTRY (1000 logical workstreams; bounded parallel execution, max 4-8 concurrent)
Workstream = one feature package (or test/ops slice) with concrete deliverable. No workstream is claimed running unless in EXECUTION-LOG.

| Workstream | Group | Deliverable | Status |
|---|---|---|---|
| WS-A-001 | A | F-D25-001 System settings console | NOT_STARTED |
| WS-A-002 | A | F-D25-002 Tenant onboarding console (admin) | NOT_STARTED |
| WS-A-003 | A | F-D25-003 License & seat manager | NOT_STARTED |
| WS-A-004 | A | F-D25-004 Feature-flag console | NOT_STARTED |
| WS-A-005 | A | F-D25-005 Release channel manager | NOT_STARTED |
| WS-A-006 | A | F-D25-006 Maintenance-mode control | NOT_STARTED |
| WS-A-007 | A | F-D25-007 Announcement broadcaster | NOT_STARTED |
| WS-A-008 | A | F-D25-008 Global search admin | NOT_STARTED |
| WS-A-009 | A | F-D25-009 Background-job monitor | NOT_STARTED |
| WS-A-010 | A | F-D25-010 Queue-depth alerting | NOT_STARTED |
| WS-A-011 | A | F-D25-011 Worker scaling policy | NOT_STARTED |
| WS-A-012 | A | F-D25-012 Storage lifecycle admin | NOT_STARTED |
| WS-A-013 | A | F-D25-013 Index maintenance console | NOT_STARTED |
| WS-A-014 | A | F-D25-014 AI provider key vault | NOT_STARTED |
| WS-A-015 | A | F-D25-015 OCR engine admin | NOT_STARTED |
| WS-A-016 | A | F-D25-016 Integration registry admin | NOT_STARTED |
| WS-A-017 | A | F-D25-017 Webhook delivery monitor | NOT_STARTED |
| WS-A-018 | A | F-D25-018 API rate-plan admin | NOT_STARTED |
| WS-A-019 | A | F-D25-019 Audit-log viewer (global) | NOT_STARTED |
| WS-A-020 | A | F-D25-020 Retention-policy designer | NOT_STARTED |
| WS-A-021 | A | F-D25-021 Legal-hold console | NOT_STARTED |
| WS-A-022 | A | F-D25-022 Backup scheduler | NOT_STARTED |
| WS-A-023 | A | F-D25-023 Restore drill tracker | NOT_STARTED |
| WS-A-024 | A | F-D25-024 DR failover runbook | NOT_STARTED |
| WS-A-025 | A | F-D25-025 Multi-region readiness checklist | NOT_STARTED |
| WS-A-026 | A | F-D25-026 Localization (i18n) manager | NOT_STARTED |
| WS-A-027 | A | F-D25-027 New-locale onboarding | NOT_STARTED |
| WS-A-028 | A | F-D25-028 Currency & tax-locale pack | NOT_STARTED |
| WS-A-029 | A | F-D25-029 Regulatory-locale pack | NOT_STARTED |
| WS-A-030 | A | F-D25-030 Time-zone policy manager | NOT_STARTED |
| WS-A-031 | A | F-D25-031 Data-residency selector | NOT_STARTED |
| WS-A-032 | A | F-D25-032 Cross-border transfer log | NOT_STARTED |
| WS-A-033 | A | F-D25-033 Expansion market assessment | NOT_STARTED |
| WS-A-034 | A | F-D25-034 Partner onboarding (new country) | NOT_STARTED |
| WS-A-035 | A | F-D25-035 Global admin dashboard | NOT_STARTED |
| WS-A-036 | A | F-D25-036 Admin audit trail | NOT_STARTED |
| WS-A-037 | A | F-D25-037 Config change approval | NOT_STARTED |
| WS-A-038 | A | F-D25-038 System health SLO board | NOT_STARTED |
| WS-A-039 | A | F-D25-039 Capacity planner | NOT_STARTED |
| WS-A-040 | A | F-D25-040 Decommissioning workflow | NOT_STARTED |
| WS-A-041 | A | F-D17-001 Tenant provisioning | NOT_STARTED |
| WS-A-042 | A | F-D17-002 Tenant branding (logo/colors) | NOT_STARTED |
| WS-A-043 | A | F-D17-003 Tenant domain mapping | NOT_STARTED |
| WS-A-044 | A | F-D17-004 Office branch registry | NOT_STARTED |
| WS-A-045 | A | F-D17-005 Office hierarchy | NOT_STARTED |
| WS-A-046 | A | F-D17-006 Central vs office data scope | NOT_STARTED |
| WS-A-047 | A | F-D17-007 Inter-office matter referral | NOT_STARTED |
| WS-A-048 | A | F-D17-008 Inter-office document sharing | NOT_STARTED |
| WS-A-049 | A | F-D17-009 Centralized user directory | NOT_STARTED |
| WS-A-050 | A | F-D17-010 Office-local admin role | NOT_STARTED |
| WS-A-051 | A | F-D17-011 Tenant-wide settings | NOT_STARTED |
| WS-A-052 | A | F-D17-012 Office working calendar | NOT_STARTED |
| WS-A-053 | A | F-D17-013 Regional holiday table | NOT_STARTED |
| WS-A-054 | A | F-D17-014 Tenant data-residency option | NOT_STARTED |
| WS-A-055 | A | F-D17-015 Tenant backup policy | NOT_STARTED |
| WS-A-056 | A | F-D17-016 Tenant quota management | NOT_STARTED |
| WS-A-057 | A | F-D17-017 Cross-office reporting | NOT_STARTED |
| WS-A-058 | A | F-D17-018 Consolidated finance view | NOT_STARTED |
| WS-A-059 | A | F-D17-019 Office performance ranking | NOT_STARTED |
| WS-A-060 | A | F-D17-020 Tenant announcement broadcast | NOT_STARTED |
| WS-A-061 | A | F-D17-021 Tenant feature-flag control | NOT_STARTED |
| WS-A-062 | A | F-D17-022 Tenant integration registry | NOT_STARTED |
| WS-A-063 | A | F-D17-023 Office onboarding wizard | NOT_STARTED |
| WS-A-064 | A | F-D17-024 Office closure / merge | NOT_STARTED |
| WS-A-065 | A | F-D17-025 Tenant suspension | NOT_STARTED |
| WS-A-066 | A | F-D17-026 Tenant deletion (with retention) | NOT_STARTED |
| WS-A-067 | A | F-D17-027 Tenant audit trail | NOT_STARTED |
| WS-A-068 | A | F-D17-028 Office access audit | NOT_STARTED |
| WS-A-069 | A | F-D17-029 Data-migration between offices | NOT_STARTED |
| WS-A-070 | A | F-D17-030 Tenant SLA profiles | NOT_STARTED |
| WS-A-071 | A | F-D17-031 Office capacity planning | NOT_STARTED |
| WS-A-072 | A | F-D17-032 Tenant support channel | NOT_STARTED |
| WS-A-073 | A | F-D17-033 Tenant health dashboard | NOT_STARTED |
| WS-A-074 | A | F-D17-034 Tenant config export/import | NOT_STARTED |
| WS-A-075 | A | F-D17-035 Tenant API quota & plan assignment | NOT_STARTED |
| WS-A-076 | A | F-D17-036 Office document templates | NOT_STARTED |
| WS-A-077 | A | F-D17-037 Office fee localization | NOT_STARTED |
| WS-A-078 | A | F-D17-038 Tenant retention policy | NOT_STARTED |
| WS-A-079 | A | F-D17-039 Tenant incident broadcast | NOT_STARTED |
| WS-A-080 | A | F-D17-040 Tenant config drift detector | NOT_STARTED |
| WS-B-001 | B | F-D16-001 User provisioning | NOT_STARTED |
| WS-B-002 | B | F-D16-002 Bulk user import | NOT_STARTED |
| WS-B-003 | B | F-D16-003 User deprovisioning | NOT_STARTED |
| WS-B-004 | B | F-D16-004 Department & team management | NOT_STARTED |
| WS-B-005 | B | F-D16-005 Job-title & grade model | NOT_STARTED |
| WS-B-006 | B | F-D16-006 RBAC role catalog | NOT_STARTED |
| WS-B-007 | B | F-D16-007 Least-privilege role templates | NOT_STARTED |
| WS-B-008 | B | F-D16-008 Permission matrix editor | NOT_STARTED |
| WS-B-009 | B | F-D16-009 Data-scope rules (office/matter) | NOT_STARTED |
| WS-B-010 | B | F-D16-010 ABAC policy rules | NOT_STARTED |
| WS-B-011 | B | F-D16-011 Temporary elevation (break-glass) | NOT_STARTED |
| WS-B-012 | B | F-D16-012 Delegation rules | NOT_STARTED |
| WS-B-013 | B | F-D16-013 API service-account management | NOT_STARTED |
| WS-B-014 | B | F-D16-014 SSO (OIDC/SAML) integration | NOT_STARTED |
| WS-B-015 | B | F-D16-015 SCIM provisioning adapter | NOT_STARTED |
| WS-B-016 | B | F-D16-016 Password policy engine | NOT_STARTED |
| WS-B-017 | B | F-D16-017 MFA enrollment (TOTP) | NOT_STARTED |
| WS-B-018 | B | F-D16-018 WebAuthn / passkey support | NOT_STARTED |
| WS-B-019 | B | F-D16-019 Session management console | NOT_STARTED |
| WS-B-020 | B | F-D16-020 Concurrent-session control | NOT_STARTED |
| WS-B-021 | B | F-D16-021 Login anomaly detection | NOT_STARTED |
| WS-B-022 | B | F-D16-022 Account lockout policy | NOT_STARTED |
| WS-B-023 | B | F-D16-023 Password-reset workflow | NOT_STARTED |
| WS-B-024 | B | F-D16-024 New-hire onboarding checklist | NOT_STARTED |
| WS-B-025 | B | F-D16-025 Transfer / rotation workflow | NOT_STARTED |
| WS-B-026 | B | F-D16-026 Exit clearance workflow | NOT_STARTED |
| WS-B-027 | B | F-D16-027 Access review campaigns | NOT_STARTED |
| WS-B-028 | B | F-D16-028 SoD conflict detector | NOT_STARTED |
| WS-B-029 | B | F-D16-029 Permission-change audit | NOT_STARTED |
| WS-B-030 | B | F-D16-030 Role-version history | NOT_STARTED |
| WS-B-031 | B | F-D16-031 Org-chart viewer | NOT_STARTED |
| WS-B-032 | B | F-D16-032 User activity timeline | NOT_STARTED |
| WS-B-033 | B | F-D16-033 Suspended-user vault | NOT_STARTED |
| WS-B-034 | B | F-D16-034 Guest / external user handling | NOT_STARTED |
| WS-B-035 | B | F-D16-035 External client-user invitation & lifecycle | NOT_STARTED |
| WS-B-036 | B | F-D16-036 Contractor time-boxed access | NOT_STARTED |
| WS-B-037 | B | F-D16-037 Emergency access log | NOT_STARTED |
| WS-B-038 | B | F-D16-038 User directory search | NOT_STARTED |
| WS-B-039 | B | F-D16-039 License / seat usage tracker | NOT_STARTED |
| WS-B-040 | B | F-D16-040 Dormant-account reviewer | NOT_STARTED |
| WS-B-041 | B | F-D03-001 Matter / project creation wizard | NOT_STARTED |
| WS-B-042 | B | F-D03-002 Matter numbering scheme | NOT_STARTED |
| WS-B-043 | B | F-D03-003 Matter classification & practice area | NOT_STARTED |
| WS-B-044 | B | F-D03-004 Matter stage pipeline | NOT_STARTED |
| WS-B-045 | B | F-D03-005 Task breakdown & assignment | NOT_STARTED |
| WS-B-046 | B | F-D03-006 Task dependencies | NOT_STARTED |
| WS-B-047 | B | F-D03-007 Milestone tracking | NOT_STARTED |
| WS-B-048 | B | F-D03-008 Deadline & SLA engine | NOT_STARTED |
| WS-B-049 | B | F-D03-009 Court / agency deadline calculator | NOT_STARTED |
| WS-B-050 | B | F-D03-010 Calendar integration (matter deadlines) | NOT_STARTED |
| WS-B-051 | B | F-D03-011 Kanban matter board | NOT_STARTED |
| WS-B-052 | B | F-D03-012 Gantt timeline view | NOT_STARTED |
| WS-B-053 | B | F-D03-013 Time entry (manual) | NOT_STARTED |
| WS-B-054 | B | F-D03-014 Timer-based time capture | NOT_STARTED |
| WS-B-055 | B | F-D03-015 Timesheet approval | NOT_STARTED |
| WS-B-056 | B | F-D03-016 Matter budget vs actual | NOT_STARTED |
| WS-B-057 | B | F-D03-017 Cost advance (panjar) tracking | NOT_STARTED |
| WS-B-058 | B | F-D03-018 Expense attachment to matter | NOT_STARTED |
| WS-B-059 | B | F-D03-019 Matter team & staffing | NOT_STARTED |
| WS-B-060 | B | F-D03-020 Workload & capacity view | NOT_STARTED |
| WS-B-061 | B | F-D03-021 Matter priority & escalation | NOT_STARTED |
| WS-B-062 | B | F-D03-022 Sub-matter / workstream grouping | NOT_STARTED |
| WS-B-063 | B | F-D03-023 Related-matter linking | NOT_STARTED |
| WS-B-064 | B | F-D03-024 Matter templates library | NOT_STARTED |
| WS-B-065 | B | F-D03-025 Recurring matter scheduler | NOT_STARTED |
| WS-B-066 | B | F-D03-026 Matter notes & activity feed | NOT_STARTED |
| WS-B-067 | B | F-D03-027 Internal memo on matter | NOT_STARTED |
| WS-B-068 | B | F-D03-028 Matter hold / suspension | NOT_STARTED |
| WS-B-069 | B | F-D03-029 Matter reopen workflow | NOT_STARTED |
| WS-B-070 | B | F-D03-030 Matter closure checklist | NOT_STARTED |
| WS-B-071 | B | F-D03-031 Post-matter review & lessons learned | NOT_STARTED |
| WS-B-072 | B | F-D03-032 Matter profitability report | NOT_STARTED |
| WS-B-073 | B | F-D03-033 Client matter statement | NOT_STARTED |
| WS-B-074 | B | F-D03-034 Matter access control | NOT_STARTED |
| WS-B-075 | B | F-D03-035 Matter audit history | NOT_STARTED |
| WS-B-076 | B | F-D03-036 Bulk matter import | NOT_STARTED |
| WS-B-077 | B | F-D03-037 Matter archive & retrieval | NOT_STARTED |
| WS-B-078 | B | F-D03-038 Matter dashboard (firm-wide) | NOT_STARTED |
| WS-B-079 | B | F-D03-039 Matter SLA breach alerts | NOT_STARTED |
| WS-B-080 | B | F-D03-040 Matter inbox triage & auto-assignment | NOT_STARTED |
| WS-B-081 | B | F-D04-001 Document upload (multi-file) | NOT_STARTED |
| WS-B-082 | B | F-D04-002 Folder hierarchy per matter | NOT_STARTED |
| WS-B-083 | B | F-D04-003 Document versioning (major/minor) | NOT_STARTED |
| WS-B-084 | B | F-D04-004 Check-in / check-out locking | NOT_STARTED |
| WS-B-085 | B | F-D04-005 Document metadata schema | NOT_STARTED |
| WS-B-086 | B | F-D04-006 Custom metadata fields | NOT_STARTED |
| WS-B-087 | B | F-D04-007 Document retention schedule | NOT_STARTED |
| WS-B-088 | B | F-D04-008 Document legal-hold application & release | NOT_STARTED |
| WS-B-089 | B | F-D04-009 Document archiving | NOT_STARTED |
| WS-B-090 | B | F-D04-010 Secure document deletion | NOT_STARTED |
| WS-B-091 | B | F-D04-011 Bulk download (matter bundle) | NOT_STARTED |
| WS-B-092 | B | F-D04-012 Document preview (PDF/Office/image) | NOT_STARTED |
| WS-B-093 | B | F-D04-013 Full-text content indexing | NOT_STARTED |
| WS-B-094 | B | F-D04-014 Document thumbnail generation | NOT_STARTED |
| WS-B-095 | B | F-D04-015 Duplicate document detection | NOT_STARTED |
| WS-B-096 | B | F-D04-016 Virus / malware scan on upload | NOT_STARTED |
| WS-B-097 | B | F-D04-017 Document access link (internal) | NOT_STARTED |
| WS-B-098 | B | F-D04-018 Document watermarking | NOT_STARTED |
| WS-B-099 | B | F-D04-019 Document stamping (received/controlled) | NOT_STARTED |
| WS-B-100 | B | F-D04-020 Redaction tools | NOT_STARTED |
| WS-B-101 | B | F-D04-021 Comparison (doc-to-doc diff) | NOT_STARTED |
| WS-B-102 | B | F-D04-022 Document approval routing | NOT_STARTED |
| WS-B-103 | B | F-D04-023 Document expiry alerts | NOT_STARTED |
| WS-B-104 | B | F-D04-024 Template-based document creation | NOT_STARTED |
| WS-B-105 | B | F-D04-025 Scanned-document ingestion | NOT_STARTED |
| WS-B-106 | B | F-D04-026 Email-to-matter filing | NOT_STARTED |
| WS-B-107 | B | F-D04-027 Document favorites & recent list | NOT_STARTED |
| WS-B-108 | B | F-D04-028 Document activity history | NOT_STARTED |
| WS-B-109 | B | F-D04-029 Storage quota per tenant/office | NOT_STARTED |
| WS-B-110 | B | F-D04-030 S3/MinIO object lifecycle policy | NOT_STARTED |
| WS-B-111 | B | F-D04-031 Trash / recycle bin with restore | NOT_STARTED |
| WS-B-112 | B | F-D04-032 Document export manifest | NOT_STARTED |
| WS-B-113 | B | F-D04-033 Document integrity hash (SHA-256) | NOT_STARTED |
| WS-B-114 | B | F-D04-034 Offline document cache | NOT_STARTED |
| WS-B-115 | B | F-D04-035 Large-file chunked upload | NOT_STARTED |
| WS-B-116 | B | F-D04-036 Document OCR queue trigger | NOT_STARTED |
| WS-B-117 | B | F-D04-037 Document tagging | NOT_STARTED |
| WS-B-118 | B | F-D04-038 Document dashboard | NOT_STARTED |
| WS-B-119 | B | F-D04-039 Document audit trail | NOT_STARTED |
| WS-B-120 | B | F-D04-040 Protocol (notary) document segregation | NOT_STARTED |
| WS-B-121 | B | F-D14-001 Chart of accounts (PSAK-aware) | NOT_STARTED |
| WS-B-122 | B | F-D14-002 Journal entry composer | NOT_STARTED |
| WS-B-123 | B | F-D14-003 Journal approval workflow | NOT_STARTED |
| WS-B-124 | B | F-D14-004 Accounts receivable ledger | NOT_STARTED |
| WS-B-125 | B | F-D14-005 Accounts payable ledger | NOT_STARTED |
| WS-B-126 | B | F-D14-006 General ledger viewer | NOT_STARTED |
| WS-B-127 | B | F-D14-007 Trial balance report | NOT_STARTED |
| WS-B-128 | B | F-D14-008 Profit & loss report | NOT_STARTED |
| WS-B-129 | B | F-D14-009 Balance-sheet report | NOT_STARTED |
| WS-B-130 | B | F-D14-010 Cash-flow report | NOT_STARTED |
| WS-B-131 | B | F-D14-011 Client trust (escrow) ledger | NOT_STARTED |
| WS-B-132 | B | F-D14-012 Trust receipt & disbursement | NOT_STARTED |
| WS-B-133 | B | F-D14-013 Trust reconciliation | NOT_STARTED |
| WS-B-134 | B | F-D14-014 Matter cost allocation | NOT_STARTED |
| WS-B-135 | B | F-D14-015 Department cost centers | NOT_STARTED |
| WS-B-136 | B | F-D14-016 Office-level P&L | NOT_STARTED |
| WS-B-137 | B | F-D14-017 Budget planner | NOT_STARTED |
| WS-B-138 | B | F-D14-018 Budget-vs-actual monitor | NOT_STARTED |
| WS-B-139 | B | F-D14-019 Fixed-asset register | NOT_STARTED |
| WS-B-140 | B | F-D14-020 Depreciation scheduler | NOT_STARTED |
| WS-B-141 | B | F-D14-021 Bank account registry | NOT_STARTED |
| WS-B-142 | B | F-D14-022 Bank reconciliation | NOT_STARTED |
| WS-B-143 | B | F-D14-023 Petty-cash ledger | NOT_STARTED |
| WS-B-144 | B | F-D14-024 Tax invoice (faktur pajak) log | NOT_STARTED |
| WS-B-145 | B | F-D14-025 Withholding-tax (PPh 23/21) tracker | NOT_STARTED |
| WS-B-146 | B | F-D14-026 VAT (PPN) report support | NOT_STARTED |
| WS-B-147 | B | F-D14-027 E-faktur data export | NOT_STARTED |
| WS-B-148 | B | F-D14-028 Payroll interface export | NOT_STARTED |
| WS-B-149 | B | F-D14-029 Reimbursement workflow | NOT_STARTED |
| WS-B-150 | B | F-D14-030 Advance-settlement workflow | NOT_STARTED |
| WS-B-151 | B | F-D14-031 Inter-office settlement | NOT_STARTED |
| WS-B-152 | B | F-D14-032 Multi-currency support | NOT_STARTED |
| WS-B-153 | B | F-D14-033 Exchange-rate table | NOT_STARTED |
| WS-B-154 | B | F-D14-034 Year-end closing checklist | NOT_STARTED |
| WS-B-155 | B | F-D14-035 Auditor data pack | NOT_STARTED |
| WS-B-156 | B | F-D14-036 Finance dashboard | NOT_STARTED |
| WS-B-157 | B | F-D14-037 Finance audit trail | NOT_STARTED |
| WS-B-158 | B | F-D14-038 Segregation-of-duties controls | NOT_STARTED |
| WS-B-159 | B | F-D14-039 Finance archive & retention | NOT_STARTED |
| WS-B-160 | B | F-D14-040 Anomaly / duplicate-payment flag | NOT_STARTED |
| WS-B-161 | B | F-D15-001 Fee-schedule library | NOT_STARTED |
| WS-B-162 | B | F-D15-002 Time-based billing rules | NOT_STARTED |
| WS-B-163 | B | F-D15-003 Fixed-fee billing | NOT_STARTED |
| WS-B-164 | B | F-D15-004 Success-fee billing | NOT_STARTED |
| WS-B-165 | B | F-D15-005 Retainer billing | NOT_STARTED |
| WS-B-166 | B | F-D15-006 Subscription plan catalog | NOT_STARTED |
| WS-B-167 | B | F-D15-007 Plan upgrade / downgrade | NOT_STARTED |
| WS-B-168 | B | F-D15-008 Proration engine | NOT_STARTED |
| WS-B-169 | B | F-D15-009 Invoice composer | NOT_STARTED |
| WS-B-170 | B | F-D15-010 E-invoice data capture | NOT_STARTED |
| WS-B-171 | B | F-D15-011 Invoice approval workflow | NOT_STARTED |
| WS-B-172 | B | F-D15-012 Invoice delivery (email/portal) | NOT_STARTED |
| WS-B-173 | B | F-D15-013 Invoice reminder schedule | NOT_STARTED |
| WS-B-174 | B | F-D15-014 Late-fee & denda policy | NOT_STARTED |
| WS-B-175 | B | F-D15-015 Payment recording (multi-method) | NOT_STARTED |
| WS-B-176 | B | F-D15-016 Payment-gateway integration adapter | NOT_STARTED |
| WS-B-177 | B | F-D15-017 Virtual-account billing | NOT_STARTED |
| WS-B-178 | B | F-D15-018 QRIS payment support | NOT_STARTED |
| WS-B-179 | B | F-D15-019 Partial-payment handling | NOT_STARTED |
| WS-B-180 | B | F-D15-020 Overpayment / credit-memo | NOT_STARTED |
| WS-C-001 | C | F-D19-001 Client login (secure) | NOT_STARTED |
| WS-C-002 | C | F-D19-002 Client registration with KYC | NOT_STARTED |
| WS-C-003 | C | F-D19-003 Client dashboard | NOT_STARTED |
| WS-C-004 | C | F-D19-004 Matter progress view | NOT_STARTED |
| WS-C-005 | C | F-D19-005 Document request inbox | NOT_STARTED |
| WS-C-006 | C | F-D19-006 Secure document upload | NOT_STARTED |
| WS-C-007 | C | F-D19-007 Shared-document viewer | NOT_STARTED |
| WS-C-008 | C | F-D19-008 E-sign signing entry | NOT_STARTED |
| WS-C-009 | C | F-D19-009 Invoice & payment view | NOT_STARTED |
| WS-C-010 | C | F-D19-010 Online payment checkout | NOT_STARTED |
| WS-C-011 | C | F-D19-011 Appointment booking | NOT_STARTED |
| WS-C-012 | C | F-D19-012 Appointment reminders | NOT_STARTED |
| WS-C-013 | C | F-D19-013 Case-message thread | NOT_STARTED |
| WS-C-014 | C | F-D19-014 Targeted portal announcements | NOT_STARTED |
| WS-C-015 | C | F-D19-015 FAQ & help center | NOT_STARTED |
| WS-C-016 | C | F-D19-016 Service catalog & pricing | NOT_STARTED |
| WS-C-017 | C | F-D19-017 Quotation request form | NOT_STARTED |
| WS-C-018 | C | F-D19-018 Lead intake public form | NOT_STARTED |
| WS-C-019 | C | F-D19-019 Feedback & rating form | NOT_STARTED |
| WS-C-020 | C | F-D19-020 Complaint submission | NOT_STARTED |
| WS-C-021 | C | F-D19-021 Consent management view | NOT_STARTED |
| WS-C-022 | C | F-D19-022 Profile & contacts editor | NOT_STARTED |
| WS-C-023 | C | F-D19-023 Company users management | NOT_STARTED |
| WS-C-024 | C | F-D19-024 Notification preferences | NOT_STARTED |
| WS-C-025 | C | F-D19-025 Download center (receipts/copies) | NOT_STARTED |
| WS-C-026 | C | F-D19-026 Request copy of deed extract | NOT_STARTED |
| WS-C-027 | C | F-D19-027 Track request status | NOT_STARTED |
| WS-C-028 | C | F-D19-028 Multi-language portal (ID/EN) | NOT_STARTED |
| WS-C-029 | C | F-D19-029 Accessibility (WCAG) support | NOT_STARTED |
| WS-C-030 | C | F-D19-030 Portal session security | NOT_STARTED |
| WS-C-031 | C | F-D19-031 Portal audit trail | NOT_STARTED |
| WS-C-032 | C | F-D19-032 Portal announcement targeting | NOT_STARTED |
| WS-C-033 | C | F-D19-033 Portal usage analytics | NOT_STARTED |
| WS-C-034 | C | F-D19-034 Portal SLA display | NOT_STARTED |
| WS-C-035 | C | F-D19-035 Portal maintenance page | NOT_STARTED |
| WS-C-036 | C | F-D19-036 Public office directory | NOT_STARTED |
| WS-C-037 | C | F-D19-037 Public lawyer/notary profiles | NOT_STARTED |
| WS-C-038 | C | F-D19-038 Public regulation digest | NOT_STARTED |
| WS-C-039 | C | F-D19-039 Portal feedback loop | NOT_STARTED |
| WS-C-040 | C | F-D19-040 Portal data-retention notice | NOT_STARTED |
| WS-C-041 | C | F-D23-001 Global search bar | NOT_STARTED |
| WS-C-042 | C | F-D23-002 Matter search | NOT_STARTED |
| WS-C-043 | C | F-D23-003 Document full-text search | NOT_STARTED |
| WS-C-044 | C | F-D23-004 Deed full-text & faceted discovery | NOT_STARTED |
| WS-C-045 | C | F-D23-005 Client search | NOT_STARTED |
| WS-C-046 | C | F-D23-006 Contact search | NOT_STARTED |
| WS-C-047 | C | F-D23-007 Clause-library search | NOT_STARTED |
| WS-C-048 | C | F-D23-008 Template search | NOT_STARTED |
| WS-C-049 | C | F-D23-009 Knowledge-base search | NOT_STARTED |
| WS-C-050 | C | F-D23-010 Regulation search | NOT_STARTED |
| WS-C-051 | C | F-D23-011 Precedent search | NOT_STARTED |
| WS-C-052 | C | F-D23-012 Faceted filtering | NOT_STARTED |
| WS-C-053 | C | F-D23-013 Saved searches | NOT_STARTED |
| WS-C-054 | C | F-D23-014 Search suggestions (typeahead) | NOT_STARTED |
| WS-C-055 | C | F-D23-015 Typo-tolerant matching | NOT_STARTED |
| WS-C-056 | C | F-D23-016 Indonesian stemming analyzer | NOT_STARTED |
| WS-C-057 | C | F-D23-017 Synonym dictionary (legal ID) | NOT_STARTED |
| WS-C-058 | C | F-D23-018 Search relevance tuning | NOT_STARTED |
| WS-C-059 | C | F-D23-019 Search result highlighting | NOT_STARTED |
| WS-C-060 | C | F-D23-020 Preview from search results | NOT_STARTED |
| WS-C-061 | C | F-D23-021 Search access-control trimming | NOT_STARTED |
| WS-C-062 | C | F-D23-022 Search audit log | NOT_STARTED |
| WS-C-063 | C | F-D23-023 Recent & frequent items | NOT_STARTED |
| WS-C-064 | C | F-D23-024 Search analytics | NOT_STARTED |
| WS-C-065 | C | F-D23-025 Zero-result feedback capture | NOT_STARTED |
| WS-C-066 | C | F-D23-026 Bulk reindex job | NOT_STARTED |
| WS-C-067 | C | F-D23-027 Index-health monitor | NOT_STARTED |
| WS-C-068 | C | F-D23-028 Tantivy index manager | NOT_STARTED |
| WS-C-069 | C | F-D23-029 Vector (Qdrant) optional index | NOT_STARTED |
| WS-C-070 | C | F-D23-030 Hybrid (keyword+vector) ranking | NOT_STARTED |
| WS-C-071 | C | F-D23-031 Search API endpoint | NOT_STARTED |
| WS-C-072 | C | F-D23-032 Search pagination & sorting | NOT_STARTED |
| WS-C-073 | C | F-D23-033 Search export (controlled) | NOT_STARTED |
| WS-C-074 | C | F-D23-034 Cross-office scoped search | NOT_STARTED |
| WS-C-075 | C | F-D23-035 Archived-content search toggle | NOT_STARTED |
| WS-C-076 | C | F-D23-036 OCR-text search | NOT_STARTED |
| WS-C-077 | C | F-D23-037 Metadata-only search mode | NOT_STARTED |
| WS-C-078 | C | F-D23-038 Search latency SLO monitor | NOT_STARTED |
| WS-C-079 | C | F-D23-039 Search relevance eval harness | NOT_STARTED |
| WS-C-080 | C | F-D23-040 Search synonym curation workflow | NOT_STARTED |
| WS-C-081 | C | F-D24-001 Dioxus mobile feasibility report | NOT_STARTED |
| WS-C-082 | C | F-D24-002 Field task inbox (mobile) | NOT_STARTED |
| WS-C-083 | C | F-D24-003 Offline-first task queue | NOT_STARTED |
| WS-C-084 | C | F-D24-004 Offline document capture | NOT_STARTED |
| WS-C-085 | C | F-D24-005 ID photo capture guidance | NOT_STARTED |
| WS-C-086 | C | F-D24-006 On-device image QC (blur/glare) | NOT_STARTED |
| WS-C-087 | C | F-D24-007 Deferred upload & retry | NOT_STARTED |
| WS-C-088 | C | F-D24-008 Mobile KYC capture flow | NOT_STARTED |
| WS-C-089 | C | F-D24-009 Appointment map & navigation | NOT_STARTED |
| WS-C-090 | C | F-D24-010 Visit check-in / check-out | NOT_STARTED |
| WS-C-091 | C | F-D24-011 Geo-tagged evidence photos | NOT_STARTED |
| WS-C-092 | C | F-D24-012 Field expense capture | NOT_STARTED |
| WS-C-093 | C | F-D24-013 Mobile time tracking | NOT_STARTED |
| WS-C-094 | C | F-D24-014 Push task assignments | NOT_STARTED |
| WS-C-095 | C | F-D24-015 Offline approval queue with deferred sync | NOT_STARTED |
| WS-C-096 | C | F-D24-016 Mobile e-sign signing | NOT_STARTED |
| WS-C-097 | C | F-D24-017 Secure mobile document viewer | NOT_STARTED |
| WS-C-098 | C | F-D24-018 Mobile redaction guardrails | NOT_STARTED |
| WS-C-099 | C | F-D24-019 Biometric app unlock | NOT_STARTED |
| WS-C-100 | C | F-D24-020 Remote-wipe & lock | NOT_STARTED |
| WS-C-101 | C | F-D24-021 MDM policy hooks | NOT_STARTED |
| WS-C-102 | C | F-D24-022 Mobile audit trail | NOT_STARTED |
| WS-C-103 | C | F-D24-023 Battery/data-saver mode | NOT_STARTED |
| WS-C-104 | C | F-D24-024 Low-bandwidth sync mode | NOT_STARTED |
| WS-C-105 | C | F-D24-025 Conflict resolution (offline edits) | NOT_STARTED |
| WS-C-106 | C | F-D24-026 Mobile crash reporting | NOT_STARTED |
| WS-C-107 | C | F-D24-027 Mobile analytics | NOT_STARTED |
| WS-C-108 | C | F-D24-028 Field-team location dashboard | NOT_STARTED |
| WS-C-109 | C | F-D24-029 Mobile release train | NOT_STARTED |
| WS-C-110 | C | F-D24-030 App-version enforcement | NOT_STARTED |
| WS-C-111 | C | F-D24-031 Mobile accessibility | NOT_STARTED |
| WS-C-112 | C | F-D24-032 Mobile language toggle | NOT_STARTED |
| WS-C-113 | C | F-D24-033 Mobile help & offline FAQ | NOT_STARTED |
| WS-C-114 | C | F-D24-034 Field safety check-in | NOT_STARTED |
| WS-C-115 | C | F-D24-035 Mobile notification prefs | NOT_STARTED |
| WS-C-116 | C | F-D24-036 Mobile storage quota | NOT_STARTED |
| WS-C-117 | C | F-D24-037 Photo-to-PDF bundling | NOT_STARTED |
| WS-C-118 | C | F-D24-038 Field-report generator | NOT_STARTED |
| WS-C-119 | C | F-D24-039 Mobile session security | NOT_STARTED |
| WS-C-120 | C | F-D24-040 Mobile offline map pack | NOT_STARTED |
| WS-C-121 | C | F-D20-001 Standard report library | NOT_STARTED |
| WS-C-122 | C | F-D20-002 Ad-hoc report builder | NOT_STARTED |
| WS-C-123 | C | F-D20-003 Report scheduling (email) | NOT_STARTED |
| WS-C-124 | C | F-D20-004 Report export (PDF/XLSX/CSV) | NOT_STARTED |
| WS-C-125 | C | F-D20-005 Dashboard designer | NOT_STARTED |
| WS-C-126 | C | F-D20-006 Role-based dashboards | NOT_STARTED |
| WS-C-127 | C | F-D20-007 Matter KPI dashboard | NOT_STARTED |
| WS-C-128 | C | F-D20-008 Financial KPI dashboard | NOT_STARTED |
| WS-C-129 | C | F-D20-009 Deed-volume period report & export | NOT_STARTED |
| WS-C-130 | C | F-D20-010 OCR accuracy trend & benchmark report | NOT_STARTED |
| WS-D-001 | D | F-D02-001 Client master record (individual) | NOT_STARTED |
| WS-D-002 | D | F-D02-002 Client master record (corporate) | NOT_STARTED |
| WS-D-003 | D | F-D02-003 Client master record (government/institution) | NOT_STARTED |
| WS-D-004 | D | F-D02-004 Lead intake & qualification | NOT_STARTED |
| WS-D-005 | D | F-D02-005 Contact management & roles | NOT_STARTED |
| WS-D-006 | D | F-D02-006 Client hierarchy (group/subsidiary) | NOT_STARTED |
| WS-D-007 | D | F-D02-007 Client segmentation & tagging | NOT_STARTED |
| WS-D-008 | D | F-D02-008 Conflict-of-interest check | NOT_STARTED |
| WS-D-009 | D | F-D02-009 Client onboarding checklist | NOT_STARTED |
| WS-D-010 | D | F-D02-010 Client offboarding & file closure | NOT_STARTED |
| WS-D-011 | D | F-D02-011 Client communication timeline | NOT_STARTED |
| WS-D-012 | D | F-D02-012 Meeting & visit log | NOT_STARTED |
| WS-D-013 | D | F-D02-013 Quotation & engagement proposal | NOT_STARTED |
| WS-D-014 | D | F-D02-014 Engagement letter management | NOT_STARTED |
| WS-D-015 | D | F-D02-015 Client fee arrangement profiles | NOT_STARTED |
| WS-D-016 | D | F-D02-016 Client document request tracker | NOT_STARTED |
| WS-D-017 | D | F-D02-017 Client portal invitation management | NOT_STARTED |
| WS-D-018 | D | F-D02-018 Client satisfaction survey | NOT_STARTED |
| WS-D-019 | D | F-D02-019 Client referral tracking | NOT_STARTED |
| WS-D-020 | D | F-D02-020 Client lifetime value view | NOT_STARTED |
| WS-D-021 | D | F-D02-021 Pipeline & opportunity tracking | NOT_STARTED |
| WS-D-022 | D | F-D02-022 Marketing campaign tracking | NOT_STARTED |
| WS-D-023 | D | F-D02-023 Event & seminar management | NOT_STARTED |
| WS-D-024 | D | F-D02-024 Lost-client / win-back workflow | NOT_STARTED |
| WS-D-025 | D | F-D02-025 Client data merge & dedupe | NOT_STARTED |
| WS-D-026 | D | F-D02-026 Client data correction request | NOT_STARTED |
| WS-D-027 | D | F-D02-027 Client consent & preference center | NOT_STARTED |
| WS-D-028 | D | F-D02-028 Corporate client employee roster | NOT_STARTED |
| WS-D-029 | D | F-D02-029 Client risk profile | NOT_STARTED |
| WS-D-030 | D | F-D02-030 VIP / priority client flagging | NOT_STARTED |
| WS-D-031 | D | F-D02-031 Client credit limit & exposure | NOT_STARTED |
| WS-D-032 | D | F-D02-032 Client birthday / key-date reminders | NOT_STARTED |
| WS-D-033 | D | F-D02-033 Client feedback & complaint handling | NOT_STARTED |
| WS-D-034 | D | F-D02-034 Multi-contact notification routing | NOT_STARTED |
| WS-D-035 | D | F-D02-035 Client search (360 view) | NOT_STARTED |
| WS-D-036 | D | F-D02-036 Client export (controlled) | NOT_STARTED |
| WS-D-037 | D | F-D02-037 CRM dashboard per partner | NOT_STARTED |
| WS-D-038 | D | F-D02-038 CRM audit trail | NOT_STARTED |
| WS-D-039 | D | F-D02-039 Data retention for prospects | NOT_STARTED |
| WS-D-040 | D | F-D02-040 Client document expiry monitoring | NOT_STARTED |
| WS-D-041 | D | F-D06-001 Deed template library (notarial) | NOT_STARTED |
| WS-D-042 | D | F-D06-002 Clause library with versioning | NOT_STARTED |
| WS-D-043 | D | F-D06-003 Deed drafting wizard | NOT_STARTED |
| WS-D-044 | D | F-D06-004 Party (penghadap) data binding | NOT_STARTED |
| WS-D-045 | D | F-D06-005 Witness (saksi) management | NOT_STARTED |
| WS-D-046 | D | F-D06-006 Deed numbering (repertorium) | NOT_STARTED |
| WS-D-047 | D | F-D06-007 Minuta vs salinan vs kutipan tracking | NOT_STARTED |
| WS-D-048 | D | F-D06-008 Deed reading (pembacaan) minutes | NOT_STARTED |
| WS-D-049 | D | F-D06-009 Deed signing ceremony checklist | NOT_STARTED |
| WS-D-050 | D | F-D06-010 Fingerprint capture log | NOT_STARTED |
| WS-D-051 | D | F-D06-011 Deed amendment (renvoi) handling | NOT_STARTED |
| WS-D-052 | D | F-D06-012 Deed attachment (lampiran) manager | NOT_STARTED |
| WS-D-053 | D | F-D06-013 Bilingual deed support (ID/EN) | NOT_STARTED |
| WS-D-054 | D | F-D06-014 Deed schema validation | NOT_STARTED |
| WS-D-055 | D | F-D06-015 Mandatory-field completeness gate | NOT_STARTED |
| WS-D-056 | D | F-D06-016 Legal citation insertion | NOT_STARTED |
| WS-D-057 | D | F-D06-017 Deed comparison (draft vs minuta) | NOT_STARTED |
| WS-D-058 | D | F-D06-018 Deed approval before signing | NOT_STARTED |
| WS-D-059 | D | F-D06-019 Deed PDF rendering (print-ready) | NOT_STARTED |
| WS-D-060 | D | F-D06-020 Deed XML/structured archive | NOT_STARTED |
| WS-D-061 | D | F-D06-021 Deed registration to external log | NOT_STARTED |
| WS-D-062 | D | F-D06-022 Deed search by number/party/date | NOT_STARTED |
| WS-D-063 | D | F-D06-023 Deed confidentiality classification | NOT_STARTED |
| WS-D-064 | D | F-D06-024 Deed correction (akta perbaikan) | NOT_STARTED |
| WS-D-065 | D | F-D06-025 Deed cancellation workflow | NOT_STARTED |
| WS-D-066 | D | F-D06-026 Deed copy issuance log | NOT_STARTED |
| WS-D-067 | D | F-D06-027 Deed excerpt issuance | NOT_STARTED |
| WS-D-068 | D | F-D06-028 Deed legalization log | NOT_STARTED |
| WS-D-069 | D | F-D06-029 Deed delivery receipt | NOT_STARTED |
| WS-D-070 | D | F-D06-030 Deed storage location tracking | NOT_STARTED |
| WS-D-071 | D | F-D06-031 Deed protocol-box transfer scheduler | NOT_STARTED |
| WS-D-072 | D | F-D06-032 Deed template access control | NOT_STARTED |
| WS-D-073 | D | F-D06-033 Deed variable audit (who changed what) | NOT_STARTED |
| WS-D-074 | D | F-D06-034 Deed multi-party signature order | NOT_STARTED |
| WS-D-075 | D | F-D06-035 Deed fee computation hook | NOT_STARTED |
| WS-D-076 | D | F-D06-036 Deed dashboard (monthly volume) | NOT_STARTED |
| WS-D-077 | D | F-D06-037 Deed audit trail | NOT_STARTED |
| WS-D-078 | D | F-D06-038 Deed export for supervision | NOT_STARTED |
| WS-D-079 | D | F-D06-039 Deed disaster-recovery copy | NOT_STARTED |
| WS-D-080 | D | F-D06-040 Deed print & binding manifest | NOT_STARTED |
| WS-D-081 | D | F-D12-001 Repertorium (deed daily log) | NOT_STARTED |
| WS-D-082 | D | F-D12-002 Klapper (alphabetical index) | NOT_STARTED |
| WS-D-083 | D | F-D12-003 Protocol book register | NOT_STARTED |
| WS-D-084 | D | F-D12-004 Protocol custody & handover log | NOT_STARTED |
| WS-D-085 | D | F-D12-005 Monthly notary report pack | NOT_STARTED |
| WS-D-086 | D | F-D12-006 MPN supervision response pack | NOT_STARTED |
| WS-D-087 | D | F-D12-007 Office working-hours & roster | NOT_STARTED |
| WS-D-088 | D | F-D12-008 Notary leave / substitute protocol | NOT_STARTED |
| WS-D-089 | D | F-D12-009 Staff role matrix (notary office) | NOT_STARTED |
| WS-D-090 | D | F-D12-010 Penghadap appointment scheduler | NOT_STARTED |
| WS-D-091 | D | F-D12-011 Queue & waiting-room board | NOT_STARTED |
| WS-D-092 | D | F-D12-012 Service price list (layanan) | NOT_STARTED |
| WS-D-093 | D | F-D12-013 Down-payment (uang muka) receipt | NOT_STARTED |
| WS-D-094 | D | F-D12-014 Deed fee quotation | NOT_STARTED |
| WS-D-095 | D | F-D12-015 Office cash book | NOT_STARTED |
| WS-D-096 | D | F-D12-016 Office cash advance & IOU tracker | NOT_STARTED |
| WS-D-097 | D | F-D12-017 Office inventory (blanko/meterai) | NOT_STARTED |
| WS-D-098 | D | F-D12-018 Meterai inventory & usage | NOT_STARTED |
| WS-D-099 | D | F-D12-019 ATK & consumables tracker | NOT_STARTED |
| WS-D-100 | D | F-D12-020 Archive room location map | NOT_STARTED |
| WS-D-101 | D | F-D12-021 Deed storage box tracking | NOT_STARTED |
| WS-D-102 | D | F-D12-022 Protocol transfer to MPD (retired) | NOT_STARTED |
| WS-D-103 | D | F-D12-023 Client identity re-verification | NOT_STARTED |
| WS-D-104 | D | F-D12-024 Saksi pool management | NOT_STARTED |
| WS-D-105 | D | F-D12-025 Office bank-partner facility & rate sheet | NOT_STARTED |
| WS-D-106 | D | F-D12-026 Office performance dashboard | NOT_STARTED |
| WS-D-107 | D | F-D12-027 Deed volume analytics | NOT_STARTED |
| WS-D-108 | D | F-D12-028 Staff productivity view | NOT_STARTED |
| WS-D-109 | D | F-D12-029 Client waiting-time analytics | NOT_STARTED |
| WS-D-110 | D | F-D12-030 Office document checklist | NOT_STARTED |
| WS-D-111 | D | F-D12-031 Office SOP library | NOT_STARTED |
| WS-D-112 | D | F-D12-032 Office announcement board | NOT_STARTED |
| WS-D-113 | D | F-D12-033 Multi-desk task dispatch | NOT_STARTED |
| WS-D-114 | D | F-D12-034 Office audit trail | NOT_STARTED |
| WS-D-115 | D | F-D12-035 Office backup verification | NOT_STARTED |
| WS-D-116 | D | F-D12-036 Office incident log | NOT_STARTED |
| WS-D-117 | D | F-D12-037 Office visitor log | NOT_STARTED |
| WS-D-118 | D | F-D12-038 Office feedback capture | NOT_STARTED |
| WS-D-119 | D | F-D12-039 Office compliance self-check | NOT_STARTED |
| WS-D-120 | D | F-D12-040 Protocol year-end sealing & handover | NOT_STARTED |
| WS-D-121 | D | F-D13-001 PPAT deed register (akta PPAT) | NOT_STARTED |
| WS-D-122 | D | F-D13-002 Land certificate (SHM/HGB) intake | NOT_STARTED |
| WS-D-123 | D | F-D13-003 Certificate authenticity checklist | NOT_STARTED |
| WS-D-124 | D | F-D13-004 BPN verification request tracker | NOT_STARTED |
| WS-D-125 | D | F-D13-005 AJB drafting (sale & purchase) | NOT_STARTED |
| WS-D-126 | D | F-D13-006 Hibah deed support | NOT_STARTED |
| WS-D-127 | D | F-D13-007 Tukar-menukar deed support | NOT_STARTED |
| WS-D-128 | D | F-D13-008 APHT (mortgage) drafting | NOT_STARTED |
| WS-D-129 | D | F-D13-009 SKMHT drafting & expiry monitor | NOT_STARTED |
| WS-D-130 | D | F-D13-010 Royalty/lease deed support | NOT_STARTED |
| WS-E-001 | E | F-D05-001 KTP OCR extraction pipeline | NOT_STARTED |
| WS-E-002 | E | F-D05-002 KK OCR extraction pipeline | NOT_STARTED |
| WS-E-003 | E | F-D05-003 NPWP OCR extraction pipeline | NOT_STARTED |
| WS-E-004 | E | F-D05-004 Passport OCR (MRZ + visual zone) | NOT_STARTED |
| WS-E-005 | E | F-D05-005 Property certificate (SHM/SHGB) OCR | NOT_STARTED |
| WS-E-006 | E | F-D05-006 Deed (akta) OCR with structure parsing | NOT_STARTED |
| WS-E-007 | E | F-D05-007 Contract OCR with clause segmentation | NOT_STARTED |
| WS-E-008 | E | F-D05-008 General PDF OCR (native + scanned) | NOT_STARTED |
| WS-E-009 | E | F-D05-009 Multi-page document orchestration | NOT_STARTED |
| WS-E-010 | E | F-D05-010 Table extraction & cell reconstruction | NOT_STARTED |
| WS-E-011 | E | F-D05-011 Layout detection (headers/footers/columns) | NOT_STARTED |
| WS-E-012 | E | F-D05-012 Document classification model | NOT_STARTED |
| WS-E-013 | E | F-D05-013 Image enhancement pre-processing | NOT_STARTED |
| WS-E-014 | E | F-D05-014 Deskew & orientation correction | NOT_STARTED |
| WS-E-015 | E | F-D05-015 Denoising & binarization | NOT_STARTED |
| WS-E-016 | E | F-D05-016 Perspective correction (mobile photos) | NOT_STARTED |
| WS-E-017 | E | F-D05-017 Blur & glare detection gate | NOT_STARTED |
| WS-E-018 | E | F-D05-018 Duplicate scan detection | NOT_STARTED |
| WS-E-019 | E | F-D05-019 Confidence scoring per field | NOT_STARTED |
| WS-E-020 | E | F-D05-020 Field-level validation rules (ID)  | NOT_STARTED |
| WS-E-021 | E | F-D05-021 Human-in-the-loop review UI | NOT_STARTED |
| WS-E-022 | E | F-D05-022 Reviewer correction capture & learning log | NOT_STARTED |
| WS-E-023 | E | F-D05-023 OCR result versioning | NOT_STARTED |
| WS-E-024 | E | F-D05-024 Secure archival of source images | NOT_STARTED |
| WS-E-025 | E | F-D05-025 Batch OCR job scheduler | NOT_STARTED |
| WS-E-026 | E | F-D05-026 OCR cost & usage metering | NOT_STARTED |
| WS-E-027 | E | F-D05-027 OCR engine router (multi-engine) | NOT_STARTED |
| WS-E-028 | E | F-D05-028 Engine benchmark harness (CER/WER) | NOT_STARTED |
| WS-E-029 | E | F-D05-029 Indonesian document test corpus registry | NOT_STARTED |
| WS-E-030 | E | F-D05-030 Handwriting flag & routing | NOT_STARTED |
| WS-E-031 | E | F-D05-031 Stamp & signature region detection | NOT_STARTED |
| WS-E-032 | E | F-D05-032 QR / barcode decode on documents | NOT_STARTED |
| WS-E-033 | E | F-D05-033 Language detection (ID/EN) | NOT_STARTED |
| WS-E-034 | E | F-D05-034 NIK/KK cross-field consistency check | NOT_STARTED |
| WS-E-035 | E | F-D05-035 Expiry-date extraction & validation | NOT_STARTED |
| WS-E-036 | E | F-D05-036 Low-quality image rejection with guidance | NOT_STARTED |
| WS-E-037 | E | F-D05-037 OCR audit trail | NOT_STARTED |
| WS-E-038 | E | F-D05-038 OCR artifact PII redaction (images/results) | NOT_STARTED |
| WS-E-039 | E | F-D05-039 OCR retry & dead-letter queue | NOT_STARTED |
| WS-E-040 | E | F-D05-040 OCR accuracy dashboard | NOT_STARTED |
| WS-E-041 | E | F-D05-001 KTP OCR extraction pipeline | NOT_STARTED |
| WS-E-042 | E | F-D05-002 KK OCR extraction pipeline | NOT_STARTED |
| WS-E-043 | E | F-D05-003 NPWP OCR extraction pipeline | NOT_STARTED |
| WS-E-044 | E | F-D05-004 Passport OCR (MRZ + visual zone) | NOT_STARTED |
| WS-E-045 | E | F-D05-005 Property certificate (SHM/SHGB) OCR | NOT_STARTED |
| WS-E-046 | E | F-D05-006 Deed (akta) OCR with structure parsing | NOT_STARTED |
| WS-E-047 | E | F-D05-007 Contract OCR with clause segmentation | NOT_STARTED |
| WS-E-048 | E | F-D05-008 General PDF OCR (native + scanned) | NOT_STARTED |
| WS-E-049 | E | F-D05-009 Multi-page document orchestration | NOT_STARTED |
| WS-E-050 | E | F-D05-010 Table extraction & cell reconstruction | NOT_STARTED |
| WS-E-051 | E | F-D05-011 Layout detection (headers/footers/columns) | NOT_STARTED |
| WS-E-052 | E | F-D05-012 Document classification model | NOT_STARTED |
| WS-E-053 | E | F-D05-013 Image enhancement pre-processing | NOT_STARTED |
| WS-E-054 | E | F-D05-014 Deskew & orientation correction | NOT_STARTED |
| WS-E-055 | E | F-D05-015 Denoising & binarization | NOT_STARTED |
| WS-E-056 | E | F-D05-016 Perspective correction (mobile photos) | NOT_STARTED |
| WS-E-057 | E | F-D05-017 Blur & glare detection gate | NOT_STARTED |
| WS-E-058 | E | F-D05-018 Duplicate scan detection | NOT_STARTED |
| WS-E-059 | E | F-D05-019 Confidence scoring per field | NOT_STARTED |
| WS-E-060 | E | F-D05-020 Field-level validation rules (ID)  | NOT_STARTED |
| WS-E-061 | E | F-D05-021 Human-in-the-loop review UI | NOT_STARTED |
| WS-E-062 | E | F-D05-022 Reviewer correction capture & learning log | NOT_STARTED |
| WS-E-063 | E | F-D05-023 OCR result versioning | NOT_STARTED |
| WS-E-064 | E | F-D05-024 Secure archival of source images | NOT_STARTED |
| WS-E-065 | E | F-D05-025 Batch OCR job scheduler | NOT_STARTED |
| WS-E-066 | E | F-D05-026 OCR cost & usage metering | NOT_STARTED |
| WS-E-067 | E | F-D05-027 OCR engine router (multi-engine) | NOT_STARTED |
| WS-E-068 | E | F-D05-028 Engine benchmark harness (CER/WER) | NOT_STARTED |
| WS-E-069 | E | F-D05-029 Indonesian document test corpus registry | NOT_STARTED |
| WS-E-070 | E | F-D05-030 Handwriting flag & routing | NOT_STARTED |
| WS-E-071 | E | F-D05-031 Stamp & signature region detection | NOT_STARTED |
| WS-E-072 | E | F-D05-032 QR / barcode decode on documents | NOT_STARTED |
| WS-E-073 | E | F-D05-033 Language detection (ID/EN) | NOT_STARTED |
| WS-E-074 | E | F-D05-034 NIK/KK cross-field consistency check | NOT_STARTED |
| WS-E-075 | E | F-D05-035 Expiry-date extraction & validation | NOT_STARTED |
| WS-E-076 | E | F-D05-036 Low-quality image rejection with guidance | NOT_STARTED |
| WS-E-077 | E | F-D05-037 OCR audit trail | NOT_STARTED |
| WS-E-078 | E | F-D05-038 OCR artifact PII redaction (images/results) | NOT_STARTED |
| WS-E-079 | E | F-D05-039 OCR retry & dead-letter queue | NOT_STARTED |
| WS-E-080 | E | F-D05-040 OCR accuracy dashboard | NOT_STARTED |
| WS-E-081 | E | F-D05-001 KTP OCR extraction pipeline | NOT_STARTED |
| WS-E-082 | E | F-D05-002 KK OCR extraction pipeline | NOT_STARTED |
| WS-E-083 | E | F-D05-003 NPWP OCR extraction pipeline | NOT_STARTED |
| WS-E-084 | E | F-D05-004 Passport OCR (MRZ + visual zone) | NOT_STARTED |
| WS-E-085 | E | F-D05-005 Property certificate (SHM/SHGB) OCR | NOT_STARTED |
| WS-E-086 | E | F-D05-006 Deed (akta) OCR with structure parsing | NOT_STARTED |
| WS-E-087 | E | F-D05-007 Contract OCR with clause segmentation | NOT_STARTED |
| WS-E-088 | E | F-D05-008 General PDF OCR (native + scanned) | NOT_STARTED |
| WS-E-089 | E | F-D05-009 Multi-page document orchestration | NOT_STARTED |
| WS-E-090 | E | F-D05-010 Table extraction & cell reconstruction | NOT_STARTED |
| WS-E-091 | E | F-D05-011 Layout detection (headers/footers/columns) | NOT_STARTED |
| WS-E-092 | E | F-D05-012 Document classification model | NOT_STARTED |
| WS-E-093 | E | F-D05-013 Image enhancement pre-processing | NOT_STARTED |
| WS-E-094 | E | F-D05-014 Deskew & orientation correction | NOT_STARTED |
| WS-E-095 | E | F-D05-015 Denoising & binarization | NOT_STARTED |
| WS-E-096 | E | F-D05-016 Perspective correction (mobile photos) | NOT_STARTED |
| WS-E-097 | E | F-D05-017 Blur & glare detection gate | NOT_STARTED |
| WS-E-098 | E | F-D05-018 Duplicate scan detection | NOT_STARTED |
| WS-E-099 | E | F-D05-019 Confidence scoring per field | NOT_STARTED |
| WS-E-100 | E | F-D05-020 Field-level validation rules (ID)  | NOT_STARTED |
| WS-E-101 | E | F-D05-021 Human-in-the-loop review UI | NOT_STARTED |
| WS-E-102 | E | F-D05-022 Reviewer correction capture & learning log | NOT_STARTED |
| WS-E-103 | E | F-D05-023 OCR result versioning | NOT_STARTED |
| WS-E-104 | E | F-D05-024 Secure archival of source images | NOT_STARTED |
| WS-E-105 | E | F-D05-025 Batch OCR job scheduler | NOT_STARTED |
| WS-E-106 | E | F-D05-026 OCR cost & usage metering | NOT_STARTED |
| WS-E-107 | E | F-D05-027 OCR engine router (multi-engine) | NOT_STARTED |
| WS-E-108 | E | F-D05-028 Engine benchmark harness (CER/WER) | NOT_STARTED |
| WS-E-109 | E | F-D05-029 Indonesian document test corpus registry | NOT_STARTED |
| WS-E-110 | E | F-D05-030 Handwriting flag & routing | NOT_STARTED |
| WS-E-111 | E | F-D05-031 Stamp & signature region detection | NOT_STARTED |
| WS-E-112 | E | F-D05-032 QR / barcode decode on documents | NOT_STARTED |
| WS-E-113 | E | F-D05-033 Language detection (ID/EN) | NOT_STARTED |
| WS-E-114 | E | F-D05-034 NIK/KK cross-field consistency check | NOT_STARTED |
| WS-E-115 | E | F-D05-035 Expiry-date extraction & validation | NOT_STARTED |
| WS-E-116 | E | F-D05-036 Low-quality image rejection with guidance | NOT_STARTED |
| WS-E-117 | E | F-D05-037 OCR audit trail | NOT_STARTED |
| WS-E-118 | E | F-D05-038 OCR artifact PII redaction (images/results) | NOT_STARTED |
| WS-E-119 | E | F-D05-039 OCR retry & dead-letter queue | NOT_STARTED |
| WS-E-120 | E | F-D05-040 OCR accuracy dashboard | NOT_STARTED |
| WS-E-121 | E | F-D05-001 KTP OCR extraction pipeline | NOT_STARTED |
| WS-E-122 | E | F-D05-002 KK OCR extraction pipeline | NOT_STARTED |
| WS-E-123 | E | F-D05-003 NPWP OCR extraction pipeline | NOT_STARTED |
| WS-E-124 | E | F-D05-004 Passport OCR (MRZ + visual zone) | NOT_STARTED |
| WS-E-125 | E | F-D05-005 Property certificate (SHM/SHGB) OCR | NOT_STARTED |
| WS-E-126 | E | F-D05-006 Deed (akta) OCR with structure parsing | NOT_STARTED |
| WS-E-127 | E | F-D05-007 Contract OCR with clause segmentation | NOT_STARTED |
| WS-E-128 | E | F-D05-008 General PDF OCR (native + scanned) | NOT_STARTED |
| WS-E-129 | E | F-D05-009 Multi-page document orchestration | NOT_STARTED |
| WS-E-130 | E | F-D05-010 Table extraction & cell reconstruction | NOT_STARTED |
| WS-F-001 | F | F-D07-001 Conversational legal Q&A (ID) | NOT_STARTED |
| WS-F-002 | F | F-D07-002 Matter-grounded Q&A (RAG) | NOT_STARTED |
| WS-F-003 | F | F-D07-003 Document summarization | NOT_STARTED |
| WS-F-004 | F | F-D07-004 Contract clause extraction | NOT_STARTED |
| WS-F-005 | F | F-D07-005 Risk clause flagging | NOT_STARTED |
| WS-F-006 | F | F-D07-006 Obligation & deadline extraction | NOT_STARTED |
| WS-F-007 | F | F-D07-007 Document comparison (AI diff narrative) | NOT_STARTED |
| WS-F-008 | F | F-D07-008 Citation & source tracking | NOT_STARTED |
| WS-F-009 | F | F-D07-009 Legal knowledge base ingestion | NOT_STARTED |
| WS-F-010 | F | F-D07-010 Knowledge base versioning | NOT_STARTED |
| WS-F-011 | F | F-D07-011 Prompt template library | NOT_STARTED |
| WS-F-012 | F | F-D07-012 Prompt versioning & rollback | NOT_STARTED |
| WS-F-013 | F | F-D07-013 Model router (cost/quality/latency) | NOT_STARTED |
| WS-F-014 | F | F-D07-014 Token usage metering | NOT_STARTED |
| WS-F-015 | F | F-D07-015 Per-matter AI cost allocation | NOT_STARTED |
| WS-F-016 | F | F-D07-016 AI confidence & disclaimer banner | NOT_STARTED |
| WS-F-017 | F | F-D07-017 Human approval gate for AI drafts | NOT_STARTED |
| WS-F-018 | F | F-D07-018 AI-assisted deed clause drafting | NOT_STARTED |
| WS-F-019 | F | F-D07-019 AI-assisted engagement letter drafting | NOT_STARTED |
| WS-F-020 | F | F-D07-020 AI-assisted email drafting | NOT_STARTED |
| WS-F-021 | F | F-D07-021 Meeting-notes summarization | NOT_STARTED |
| WS-F-022 | F | F-D07-022 Regulation change digest | NOT_STARTED |
| WS-F-023 | F | F-D07-023 Case-law research helper | NOT_STARTED |
| WS-F-024 | F | F-D07-024 Precedent retrieval | NOT_STARTED |
| WS-F-025 | F | F-D07-025 Jurisdiction guardrail (ID-first) | NOT_STARTED |
| WS-F-026 | F | F-D07-026 Hallucination mitigation controls | NOT_STARTED |
| WS-F-027 | F | F-D07-027 Prompt-injection protection | NOT_STARTED |
| WS-F-028 | F | F-D07-028 PII redaction before model call | NOT_STARTED |
| WS-F-029 | F | F-D07-029 Local-model execution path | NOT_STARTED |
| WS-F-030 | F | F-D07-030 Model evaluation harness | NOT_STARTED |
| WS-F-031 | F | F-D07-031 Feedback (thumbs up/down + correction) | NOT_STARTED |
| WS-F-032 | F | F-D07-032 AI audit trail (prompt/response/model) | NOT_STARTED |
| WS-F-033 | F | F-D07-033 AI data retention policy | NOT_STARTED |
| WS-F-034 | F | F-D07-034 Blocked-topic policy (validity certification) | NOT_STARTED |
| WS-F-035 | F | F-D07-035 Multi-language response (ID/EN) | NOT_STARTED |
| WS-F-036 | F | F-D07-036 Voice-to-text inquiry capture | NOT_STARTED |
| WS-F-037 | F | F-D07-037 AI workspace (side panel) | NOT_STARTED |
| WS-F-038 | F | F-D07-038 AI spend & model-mix analytics | NOT_STARTED |
| WS-F-039 | F | F-D07-039 AI incident & correction log | NOT_STARTED |
| WS-F-040 | F | F-D07-040 AI red-team prompt test suite | NOT_STARTED |
| WS-F-041 | F | F-D07-001 Conversational legal Q&A (ID) | NOT_STARTED |
| WS-F-042 | F | F-D07-002 Matter-grounded Q&A (RAG) | NOT_STARTED |
| WS-F-043 | F | F-D07-003 Document summarization | NOT_STARTED |
| WS-F-044 | F | F-D07-004 Contract clause extraction | NOT_STARTED |
| WS-F-045 | F | F-D07-005 Risk clause flagging | NOT_STARTED |
| WS-F-046 | F | F-D07-006 Obligation & deadline extraction | NOT_STARTED |
| WS-F-047 | F | F-D07-007 Document comparison (AI diff narrative) | NOT_STARTED |
| WS-F-048 | F | F-D07-008 Citation & source tracking | NOT_STARTED |
| WS-F-049 | F | F-D07-009 Legal knowledge base ingestion | NOT_STARTED |
| WS-F-050 | F | F-D07-010 Knowledge base versioning | NOT_STARTED |
| WS-F-051 | F | F-D07-011 Prompt template library | NOT_STARTED |
| WS-F-052 | F | F-D07-012 Prompt versioning & rollback | NOT_STARTED |
| WS-F-053 | F | F-D07-013 Model router (cost/quality/latency) | NOT_STARTED |
| WS-F-054 | F | F-D07-014 Token usage metering | NOT_STARTED |
| WS-F-055 | F | F-D07-015 Per-matter AI cost allocation | NOT_STARTED |
| WS-F-056 | F | F-D07-016 AI confidence & disclaimer banner | NOT_STARTED |
| WS-F-057 | F | F-D07-017 Human approval gate for AI drafts | NOT_STARTED |
| WS-F-058 | F | F-D07-018 AI-assisted deed clause drafting | NOT_STARTED |
| WS-F-059 | F | F-D07-019 AI-assisted engagement letter drafting | NOT_STARTED |
| WS-F-060 | F | F-D07-020 AI-assisted email drafting | NOT_STARTED |
| WS-F-061 | F | F-D07-021 Meeting-notes summarization | NOT_STARTED |
| WS-F-062 | F | F-D07-022 Regulation change digest | NOT_STARTED |
| WS-F-063 | F | F-D07-023 Case-law research helper | NOT_STARTED |
| WS-F-064 | F | F-D07-024 Precedent retrieval | NOT_STARTED |
| WS-F-065 | F | F-D07-025 Jurisdiction guardrail (ID-first) | NOT_STARTED |
| WS-F-066 | F | F-D07-026 Hallucination mitigation controls | NOT_STARTED |
| WS-F-067 | F | F-D07-027 Prompt-injection protection | NOT_STARTED |
| WS-F-068 | F | F-D07-028 PII redaction before model call | NOT_STARTED |
| WS-F-069 | F | F-D07-029 Local-model execution path | NOT_STARTED |
| WS-F-070 | F | F-D07-030 Model evaluation harness | NOT_STARTED |
| WS-F-071 | F | F-D07-031 Feedback (thumbs up/down + correction) | NOT_STARTED |
| WS-F-072 | F | F-D07-032 AI audit trail (prompt/response/model) | NOT_STARTED |
| WS-F-073 | F | F-D07-033 AI data retention policy | NOT_STARTED |
| WS-F-074 | F | F-D07-034 Blocked-topic policy (validity certification) | NOT_STARTED |
| WS-F-075 | F | F-D07-035 Multi-language response (ID/EN) | NOT_STARTED |
| WS-F-076 | F | F-D07-036 Voice-to-text inquiry capture | NOT_STARTED |
| WS-F-077 | F | F-D07-037 AI workspace (side panel) | NOT_STARTED |
| WS-F-078 | F | F-D07-038 AI spend & model-mix analytics | NOT_STARTED |
| WS-F-079 | F | F-D07-039 AI incident & correction log | NOT_STARTED |
| WS-F-080 | F | F-D07-040 AI red-team prompt test suite | NOT_STARTED |
| WS-F-081 | F | F-D07-001 Conversational legal Q&A (ID) | NOT_STARTED |
| WS-F-082 | F | F-D07-002 Matter-grounded Q&A (RAG) | NOT_STARTED |
| WS-F-083 | F | F-D07-003 Document summarization | NOT_STARTED |
| WS-F-084 | F | F-D07-004 Contract clause extraction | NOT_STARTED |
| WS-F-085 | F | F-D07-005 Risk clause flagging | NOT_STARTED |
| WS-F-086 | F | F-D07-006 Obligation & deadline extraction | NOT_STARTED |
| WS-F-087 | F | F-D07-007 Document comparison (AI diff narrative) | NOT_STARTED |
| WS-F-088 | F | F-D07-008 Citation & source tracking | NOT_STARTED |
| WS-F-089 | F | F-D07-009 Legal knowledge base ingestion | NOT_STARTED |
| WS-F-090 | F | F-D07-010 Knowledge base versioning | NOT_STARTED |
| WS-F-091 | F | F-D07-011 Prompt template library | NOT_STARTED |
| WS-F-092 | F | F-D07-012 Prompt versioning & rollback | NOT_STARTED |
| WS-F-093 | F | F-D07-013 Model router (cost/quality/latency) | NOT_STARTED |
| WS-F-094 | F | F-D07-014 Token usage metering | NOT_STARTED |
| WS-F-095 | F | F-D07-015 Per-matter AI cost allocation | NOT_STARTED |
| WS-F-096 | F | F-D07-016 AI confidence & disclaimer banner | NOT_STARTED |
| WS-F-097 | F | F-D07-017 Human approval gate for AI drafts | NOT_STARTED |
| WS-F-098 | F | F-D07-018 AI-assisted deed clause drafting | NOT_STARTED |
| WS-F-099 | F | F-D07-019 AI-assisted engagement letter drafting | NOT_STARTED |
| WS-F-100 | F | F-D07-020 AI-assisted email drafting | NOT_STARTED |
| WS-G-001 | G | F-D11-001 UU PDP obligation register | NOT_STARTED |
| WS-G-002 | G | F-D11-002 Consent lifecycle manager | NOT_STARTED |
| WS-G-003 | G | F-D11-003 Data-subject access request (DSAR) | NOT_STARTED |
| WS-G-004 | G | F-D11-004 Data-subject deletion request | NOT_STARTED |
| WS-G-005 | G | F-D11-005 Rectification request workflow | NOT_STARTED |
| WS-G-006 | G | F-D11-006 Processing-activity record (ROPA) | NOT_STARTED |
| WS-G-007 | G | F-D11-007 DPIA workflow | NOT_STARTED |
| WS-G-008 | G | F-D11-008 Breach notification workflow (72h) | NOT_STARTED |
| WS-G-009 | G | F-D11-009 Vendor / processor register | NOT_STARTED |
| WS-G-010 | G | F-D11-010 DPA (perjanjian pemrosesan) tracker | NOT_STARTED |
| WS-G-011 | G | F-D11-011 Data-retention rule engine | NOT_STARTED |
| WS-G-012 | G | F-D11-012 Purge execution with approval | NOT_STARTED |
| WS-G-013 | G | F-D11-013 Protocol retention (25-year) scheduler | NOT_STARTED |
| WS-G-014 | G | F-D11-014 Conflict-of-interest register | NOT_STARTED |
| WS-G-015 | G | F-D11-015 AML/CFT risk scoring | NOT_STARTED |
| WS-G-016 | G | F-D11-016 Sanctions re-screening scheduler | NOT_STARTED |
| WS-G-017 | G | F-D11-017 Ethics & independence declarations | NOT_STARTED |
| WS-G-018 | G | F-D11-018 Gift & hospitality register | NOT_STARTED |
| WS-G-019 | G | F-D11-019 Whistleblowing intake channel | NOT_STARTED |
| WS-G-020 | G | F-D11-020 Incident register & triage | NOT_STARTED |
| WS-G-021 | G | F-D11-021 Corrective-action (CAPA) tracker | NOT_STARTED |
| WS-G-022 | G | F-D11-022 Regulatory change tracker | NOT_STARTED |
| WS-G-023 | G | F-D11-023 Compliance training assignments | NOT_STARTED |
| WS-G-024 | G | F-D11-024 Training completion evidence | NOT_STARTED |
| WS-G-025 | G | F-D11-025 Internal audit plan | NOT_STARTED |
| WS-G-026 | G | F-D11-026 Audit finding tracker | NOT_STARTED |
| WS-G-027 | G | F-D11-027 Control library (UUJN/PDP/ITE) | NOT_STARTED |
| WS-G-028 | G | F-D11-028 Control test scheduler | NOT_STARTED |
| WS-G-029 | G | F-D11-029 Compliance dashboard | NOT_STARTED |
| WS-G-030 | G | F-D11-030 Compliance evidence vault | NOT_STARTED |
| WS-G-031 | G | F-D11-031 Regulator inquiry response pack | NOT_STARTED |
| WS-G-032 | G | F-D11-032 Matter-level compliance checklist | NOT_STARTED |
| WS-G-033 | G | F-D11-033 Office compliance scorecard | NOT_STARTED |
| WS-G-034 | G | F-D11-034 Compliance alert feed | NOT_STARTED |
| WS-G-035 | G | F-D11-035 Policy library with ack tracking | NOT_STARTED |
| WS-G-036 | G | F-D11-036 Exception / waiver workflow | NOT_STARTED |
| WS-G-037 | G | F-D11-037 Risk register (enterprise) | NOT_STARTED |
| WS-G-038 | G | F-D11-038 Risk heatmap | NOT_STARTED |
| WS-G-039 | G | F-D11-039 Compliance audit trail | NOT_STARTED |
| WS-G-040 | G | F-D11-040 Compliance report exporter | NOT_STARTED |
| WS-G-041 | G | F-D22-001 Password hashing (Argon2id) | NOT_STARTED |
| WS-G-042 | G | F-D22-002 MFA enforcement policy | NOT_STARTED |
| WS-G-043 | G | F-D22-003 SSO hardening guide | NOT_STARTED |
| WS-G-044 | G | F-D22-004 Session timeout & rotation | NOT_STARTED |
| WS-G-045 | G | F-D22-005 JWT access/refresh design | NOT_STARTED |
| WS-G-046 | G | F-D22-006 API token vault | NOT_STARTED |
| WS-G-047 | G | F-D22-007 Encryption at rest (AES-256) | NOT_STARTED |
| WS-G-048 | G | F-D22-008 TLS enforcement (1.2+) | NOT_STARTED |
| WS-G-049 | G | F-D22-009 Key-management (KMS/HSM) hooks | NOT_STARTED |
| WS-G-050 | G | F-D22-010 Secrets rotation scheduler | NOT_STARTED |
| WS-G-051 | G | F-D22-011 Field-level encryption (NIK/PII) | NOT_STARTED |
| WS-G-052 | G | F-D22-012 PII masking in logs/traces | NOT_STARTED |
| WS-G-053 | G | F-D22-013 Backup encryption | NOT_STARTED |
| WS-G-054 | G | F-D22-014 Secure file-upload pipeline | NOT_STARTED |
| WS-G-055 | G | F-D22-015 Malware scanning gate | NOT_STARTED |
| WS-G-056 | G | F-D22-016 Rate limiting & brute-force shield | NOT_STARTED |
| WS-G-057 | G | F-D22-017 WAF rule baseline | NOT_STARTED |
| WS-G-058 | G | F-D22-018 Security headers baseline | NOT_STARTED |
| WS-G-059 | G | F-D22-019 CSRF protection | NOT_STARTED |
| WS-G-060 | G | F-D22-020 XSS output-encoding standard | NOT_STARTED |
| WS-G-061 | G | F-D22-021 SQL-injection guardrails (SQLx) | NOT_STARTED |
| WS-G-062 | G | F-D22-022 Dependency vulnerability scan | NOT_STARTED |
| WS-G-063 | G | F-D22-023 Container image scan | NOT_STARTED |
| WS-G-064 | G | F-D22-024 SAST baseline | NOT_STARTED |
| WS-G-065 | G | F-D22-025 DAST baseline | NOT_STARTED |
| WS-G-066 | G | F-D22-026 Audit-log integrity (hash chain) | NOT_STARTED |
| WS-G-067 | G | F-D22-027 Tamper-evident evidence store | NOT_STARTED |
| WS-G-068 | G | F-D22-028 Incident response runbooks | NOT_STARTED |
| WS-G-069 | G | F-D22-029 Breach containment & forensic preservation runbook | NOT_STARTED |
| WS-G-070 | G | F-D22-030 Security training tracker | NOT_STARTED |
| WS-G-071 | G | F-D22-031 Pen-test finding tracker | NOT_STARTED |
| WS-G-072 | G | F-D22-032 Threat-model registry (STRIDE) | NOT_STARTED |
| WS-G-073 | G | F-D22-033 OWASP ASVS mapping | NOT_STARTED |
| WS-G-074 | G | F-D22-034 Backup-restore security test | NOT_STARTED |
| WS-G-075 | G | F-D22-035 Data-residency controls | NOT_STARTED |
| WS-G-076 | G | F-D22-036 Secure decommissioning | NOT_STARTED |
| WS-G-077 | G | F-D22-037 Tenant isolation tests | NOT_STARTED |
| WS-G-078 | G | F-D22-038 Security dashboard | NOT_STARTED |
| WS-G-079 | G | F-D22-039 Security audit export | NOT_STARTED |
| WS-G-080 | G | F-D22-040 Security exception register | NOT_STARTED |
| WS-G-081 | G | F-D10-001 PSrE provider adapter (certified) | NOT_STARTED |
| WS-G-082 | G | F-D10-002 Uncertified e-sign path (low-risk) | NOT_STARTED |
| WS-G-083 | G | F-D10-003 Signature order orchestration | NOT_STARTED |
| WS-G-084 | G | F-D10-004 Sequential signing | NOT_STARTED |
| WS-G-085 | G | F-D10-005 Parallel signing | NOT_STARTED |
| WS-G-086 | G | F-D10-006 Signer identity binding (KYC) | NOT_STARTED |
| WS-G-087 | G | F-D10-007 Signer OTP challenge | NOT_STARTED |
| WS-G-088 | G | F-D10-008 Signer consent capture | NOT_STARTED |
| WS-G-089 | G | F-D10-009 Document hash sealing (pre-sign) | NOT_STARTED |
| WS-G-090 | G | F-D10-010 Long-term validation (LTV) artifacts | NOT_STARTED |
| WS-G-091 | G | F-D10-011 Timestamp authority (TSA) binding | NOT_STARTED |
| WS-G-092 | G | F-D10-012 Signature appearance placement | NOT_STARTED |
| WS-G-093 | G | F-D10-013 Initials on every page option | NOT_STARTED |
| WS-G-094 | G | F-D10-014 Bulk signing session | NOT_STARTED |
| WS-G-095 | G | F-D10-015 Signing link via secure URL | NOT_STARTED |
| WS-G-096 | G | F-D10-016 Signing reminder schedule | NOT_STARTED |
| WS-G-097 | G | F-D10-017 Signer decline handling | NOT_STARTED |
| WS-G-098 | G | F-D10-018 Signer reassignment | NOT_STARTED |
| WS-G-099 | G | F-D10-019 Document preview before signing | NOT_STARTED |
| WS-G-100 | G | F-D10-020 Post-sign tamper evidence | NOT_STARTED |
| WS-H-001 | H | Test package for F-D01-001 National ID (KTP-el) capture & validation | NOT_STARTED |
| WS-H-002 | H | Test package for F-D01-011 Sanctions / watchlist screening | NOT_STARTED |
| WS-H-003 | H | Test package for F-D01-021 Consent capture for data processing (UU PDP) | NOT_STARTED |
| WS-H-004 | H | Test package for F-D01-031 NIK masking & display policy | NOT_STARTED |
| WS-H-005 | H | Test package for F-D02-001 Client master record (individual) | NOT_STARTED |
| WS-H-006 | H | Test package for F-D02-011 Client communication timeline | NOT_STARTED |
| WS-H-007 | H | Test package for F-D02-021 Pipeline & opportunity tracking | NOT_STARTED |
| WS-H-008 | H | Test package for F-D02-031 Client credit limit & exposure | NOT_STARTED |
| WS-H-009 | H | Test package for F-D03-001 Matter / project creation wizard | NOT_STARTED |
| WS-H-010 | H | Test package for F-D03-011 Kanban matter board | NOT_STARTED |
| WS-H-011 | H | Test package for F-D03-021 Matter priority & escalation | NOT_STARTED |
| WS-H-012 | H | Test package for F-D03-031 Post-matter review & lessons learned | NOT_STARTED |
| WS-H-013 | H | Test package for F-D04-001 Document upload (multi-file) | NOT_STARTED |
| WS-H-014 | H | Test package for F-D04-011 Bulk download (matter bundle) | NOT_STARTED |
| WS-H-015 | H | Test package for F-D04-021 Comparison (doc-to-doc diff) | NOT_STARTED |
| WS-H-016 | H | Test package for F-D04-031 Trash / recycle bin with restore | NOT_STARTED |
| WS-H-017 | H | Test package for F-D05-001 KTP OCR extraction pipeline | NOT_STARTED |
| WS-H-018 | H | Test package for F-D05-011 Layout detection (headers/footers/columns) | NOT_STARTED |
| WS-H-019 | H | Test package for F-D05-021 Human-in-the-loop review UI | NOT_STARTED |
| WS-H-020 | H | Test package for F-D05-031 Stamp & signature region detection | NOT_STARTED |
| WS-H-021 | H | Test package for F-D06-001 Deed template library (notarial) | NOT_STARTED |
| WS-H-022 | H | Test package for F-D06-011 Deed amendment (renvoi) handling | NOT_STARTED |
| WS-H-023 | H | Test package for F-D06-021 Deed registration to external log | NOT_STARTED |
| WS-H-024 | H | Test package for F-D06-031 Deed protocol-box transfer scheduler | NOT_STARTED |
| WS-H-025 | H | Test package for F-D07-001 Conversational legal Q&A (ID) | NOT_STARTED |
| WS-H-026 | H | Test package for F-D07-011 Prompt template library | NOT_STARTED |
| WS-H-027 | H | Test package for F-D07-021 Meeting-notes summarization | NOT_STARTED |
| WS-H-028 | H | Test package for F-D07-031 Feedback (thumbs up/down + correction) | NOT_STARTED |
| WS-H-029 | H | Test package for F-D08-001 Visual workflow designer | NOT_STARTED |
| WS-H-030 | H | Test package for F-D08-011 Scheduled (cron) triggers | NOT_STARTED |
| WS-H-031 | H | Test package for F-D08-021 AI step connector | NOT_STARTED |
| WS-H-032 | H | Test package for F-D08-031 Event log per instance | NOT_STARTED |
| WS-H-033 | H | Test package for F-D09-001 Approval request composer | NOT_STARTED |
| WS-H-034 | H | Test package for F-D09-011 Approval attachment review | NOT_STARTED |
| WS-H-035 | H | Test package for F-D09-021 Approval policy versioning | NOT_STARTED |
| WS-H-036 | H | Test package for F-D09-031 Approval archive & retrieval | NOT_STARTED |
| WS-H-037 | H | Test package for F-D10-001 PSrE provider adapter (certified) | NOT_STARTED |
| WS-H-038 | H | Test package for F-D10-011 Timestamp authority (TSA) binding | NOT_STARTED |
| WS-H-039 | H | Test package for F-D10-021 Signed-document vault | NOT_STARTED |
| WS-H-040 | H | Test package for F-D10-031 E-sign webhook callbacks | NOT_STARTED |
| WS-H-041 | H | Test package for F-D11-001 UU PDP obligation register | NOT_STARTED |
| WS-H-042 | H | Test package for F-D11-011 Data-retention rule engine | NOT_STARTED |
| WS-H-043 | H | Test package for F-D11-021 Corrective-action (CAPA) tracker | NOT_STARTED |
| WS-H-044 | H | Test package for F-D11-031 Regulator inquiry response pack | NOT_STARTED |
| WS-H-045 | H | Test package for F-D12-001 Repertorium (deed daily log) | NOT_STARTED |
| WS-H-046 | H | Test package for F-D12-011 Queue & waiting-room board | NOT_STARTED |
| WS-H-047 | H | Test package for F-D12-021 Deed storage box tracking | NOT_STARTED |
| WS-H-048 | H | Test package for F-D12-031 Office SOP library | NOT_STARTED |
| WS-H-049 | H | Test package for F-D13-001 PPAT deed register (akta PPAT) | NOT_STARTED |
| WS-H-050 | H | Test package for F-D13-011 Land-split (pemecahan) tracker | NOT_STARTED |
| WS-H-051 | H | Test package for F-D13-021  girik/adat land intake | NOT_STARTED |
| WS-H-052 | H | Test package for F-D13-031 Zone (ZNT/NJOP) reference table | NOT_STARTED |
| WS-H-053 | H | Test package for F-D14-001 Chart of accounts (PSAK-aware) | NOT_STARTED |
| WS-H-054 | H | Test package for F-D14-011 Client trust (escrow) ledger | NOT_STARTED |
| WS-H-055 | H | Test package for F-D14-021 Bank account registry | NOT_STARTED |
| WS-H-056 | H | Test package for F-D14-031 Inter-office settlement | NOT_STARTED |
| WS-H-057 | H | Test package for F-D15-001 Fee-schedule library | NOT_STARTED |
| WS-H-058 | H | Test package for F-D15-011 Invoice approval workflow | NOT_STARTED |
| WS-H-059 | H | Test package for F-D15-021 Refund workflow | NOT_STARTED |
| WS-H-060 | H | Test package for F-D15-031 Revenue recognition hook | NOT_STARTED |
| WS-H-061 | H | Test package for F-D16-001 User provisioning | NOT_STARTED |
| WS-H-062 | H | Test package for F-D16-011 Temporary elevation (break-glass) | NOT_STARTED |
| WS-H-063 | H | Test package for F-D16-021 Login anomaly detection | NOT_STARTED |
| WS-H-064 | H | Test package for F-D16-031 Org-chart viewer | NOT_STARTED |
| WS-H-065 | H | Test package for F-D17-001 Tenant provisioning | NOT_STARTED |
| WS-H-066 | H | Test package for F-D17-011 Tenant-wide settings | NOT_STARTED |
| WS-H-067 | H | Test package for F-D17-021 Tenant feature-flag control | NOT_STARTED |
| WS-H-068 | H | Test package for F-D17-031 Office capacity planning | NOT_STARTED |
| WS-H-069 | H | Test package for F-D18-001 In-app notification center | NOT_STARTED |
| WS-H-070 | H | Test package for F-D18-011 Matter-event notifications | NOT_STARTED |
| WS-H-071 | H | Test package for F-D18-021 File attachment in messages | NOT_STARTED |
| WS-H-072 | H | Test package for F-D18-031 Sender identity (domain) config | NOT_STARTED |
| WS-H-073 | H | Test package for F-D19-001 Client login (secure) | NOT_STARTED |
| WS-H-074 | H | Test package for F-D19-011 Appointment booking | NOT_STARTED |
| WS-H-075 | H | Test package for F-D19-021 Consent management view | NOT_STARTED |
| WS-H-076 | H | Test package for F-D19-031 Portal audit trail | NOT_STARTED |
| WS-H-077 | H | Test package for F-D20-001 Standard report library | NOT_STARTED |
| WS-H-078 | H | Test package for F-D20-011 AI cost & quality executive report | NOT_STARTED |
| WS-H-079 | H | Test package for F-D20-021 Report version history | NOT_STARTED |
| WS-H-080 | H | Test package for F-D20-031 Embedded analytics (portal) | NOT_STARTED |
| WS-H-081 | H | Test package for F-D21-001 Public REST API (versioned) | NOT_STARTED |
| WS-H-082 | H | Test package for F-D21-011 Dukcapil connector | NOT_STARTED |
| WS-H-083 | H | Test package for F-D21-021 SSO/SCIM connectors | NOT_STARTED |
| WS-H-084 | H | Test package for F-D21-031 Integration health monitor | NOT_STARTED |
| WS-H-085 | H | Test package for F-D22-001 Password hashing (Argon2id) | NOT_STARTED |
| WS-H-086 | H | Test package for F-D22-011 Field-level encryption (NIK/PII) | NOT_STARTED |
| WS-H-087 | H | Test package for F-D22-021 SQL-injection guardrails (SQLx) | NOT_STARTED |
| WS-H-088 | H | Test package for F-D22-031 Pen-test finding tracker | NOT_STARTED |
| WS-H-089 | H | Test package for F-D23-001 Global search bar | NOT_STARTED |
| WS-H-090 | H | Test package for F-D23-011 Precedent search | NOT_STARTED |
| WS-H-091 | H | Test package for F-D23-021 Search access-control trimming | NOT_STARTED |
| WS-H-092 | H | Test package for F-D23-031 Search API endpoint | NOT_STARTED |
| WS-H-093 | H | Test package for F-D24-001 Dioxus mobile feasibility report | NOT_STARTED |
| WS-H-094 | H | Test package for F-D24-011 Geo-tagged evidence photos | NOT_STARTED |
| WS-H-095 | H | Test package for F-D24-021 MDM policy hooks | NOT_STARTED |
| WS-H-096 | H | Test package for F-D24-031 Mobile accessibility | NOT_STARTED |
| WS-H-097 | H | Test package for F-D25-001 System settings console | NOT_STARTED |
| WS-H-098 | H | Test package for F-D25-011 Worker scaling policy | NOT_STARTED |
| WS-H-099 | H | Test package for F-D25-021 Legal-hold console | NOT_STARTED |
| WS-H-100 | H | Test package for F-D25-031 Data-residency selector | NOT_STARTED |
| WS-I-001 | I | F-D21-001 Public REST API (versioned) | NOT_STARTED |
| WS-I-002 | I | F-D21-002 API key management | NOT_STARTED |
| WS-I-003 | I | F-D21-003 OAuth2 client management | NOT_STARTED |
| WS-I-004 | I | F-D21-004 Rate limiting & quotas | NOT_STARTED |
| WS-I-005 | I | F-D21-005 API usage metering | NOT_STARTED |
| WS-I-006 | I | F-D21-006 Developer portal & docs | NOT_STARTED |
| WS-I-007 | I | F-D21-007 OpenAPI spec publishing | NOT_STARTED |
| WS-I-008 | I | F-D21-008 Webhook dispatcher | NOT_STARTED |
| WS-I-009 | I | F-D21-009 Inbound webhook receiver | NOT_STARTED |
| WS-I-010 | I | F-D21-010 Event catalog (domain events) | NOT_STARTED |
| WS-I-011 | I | F-D21-011 Dukcapil connector | NOT_STARTED |
| WS-I-012 | I | F-D21-012 Tax (DJP) data adapter | NOT_STARTED |
| WS-I-013 | I | F-D21-013 BPN integration adapter | NOT_STARTED |
| WS-I-014 | I | F-D21-014 Bank / VA integration | NOT_STARTED |
| WS-I-015 | I | F-D21-015 Payment-gateway connector certification pack | NOT_STARTED |
| WS-I-016 | I | F-D21-016 PSrE e-sign adapter | NOT_STARTED |
| WS-I-017 | I | F-D21-017 SMS gateway adapter | NOT_STARTED |
| WS-I-018 | I | F-D21-018 WhatsApp gateway adapter | NOT_STARTED |
| WS-I-019 | I | F-D21-019 Email provider adapter | NOT_STARTED |
| WS-I-020 | I | F-D21-020 Cloud-storage (S3) connector | NOT_STARTED |
| WS-I-021 | I | F-D21-021 SSO/SCIM connectors | NOT_STARTED |
| WS-I-022 | I | F-D21-022 Accounting export connector | NOT_STARTED |
| WS-I-023 | I | F-D21-023 E-faktur export adapter | NOT_STARTED |
| WS-I-024 | I | F-D21-024 BI export (warehouse) feed | NOT_STARTED |
| WS-I-025 | I | F-D21-025 Mobile SDK auth endpoints | NOT_STARTED |
| WS-I-026 | I | F-D21-026 OCR engine plugin API | NOT_STARTED |
| WS-I-027 | I | F-D21-027 AI provider plugin API | NOT_STARTED |
| WS-I-028 | I | F-D21-028 Idempotency-key support | NOT_STARTED |
| WS-I-029 | I | F-D21-029 API sandbox environment | NOT_STARTED |
| WS-I-030 | I | F-D21-030 API changelog & deprecation | NOT_STARTED |
| WS-I-031 | I | F-D21-031 Integration health monitor | NOT_STARTED |
| WS-I-032 | I | F-D21-032 Retry & DLQ for webhooks | NOT_STARTED |
| WS-I-033 | I | F-D21-033 API audit trail | NOT_STARTED |
| WS-I-034 | I | F-D21-034 Partner onboarding workflow | NOT_STARTED |
| WS-I-035 | I | F-D21-035 Integration test harness | NOT_STARTED |
| WS-I-036 | I | F-D21-036 API abuse detection | NOT_STARTED |
| WS-I-037 | I | F-D21-037 Request signing (HMAC) | NOT_STARTED |
| WS-I-038 | I | F-D21-038 Field-level encryption for APIs | NOT_STARTED |
| WS-I-039 | I | F-D21-039 Tenant-scoped API tokens | NOT_STARTED |
| WS-I-040 | I | F-D21-040 API dashboard | NOT_STARTED |
| WS-I-041 | I | F-D18-001 In-app notification center | NOT_STARTED |
| WS-I-042 | I | F-D18-002 Email notification engine | NOT_STARTED |
| WS-I-043 | I | F-D18-003 SMS notification adapter | NOT_STARTED |
| WS-I-044 | I | F-D18-004 WhatsApp notification adapter | NOT_STARTED |
| WS-I-045 | I | F-D18-005 Push notification (mobile) | NOT_STARTED |
| WS-I-046 | I | F-D18-006 Notification preference center | NOT_STARTED |
| WS-I-047 | I | F-D18-007 Notification templating | NOT_STARTED |
| WS-I-048 | I | F-D18-008 Multi-language notification | NOT_STARTED |
| WS-I-049 | I | F-D18-009 Notification scheduling | NOT_STARTED |
| WS-I-050 | I | F-D18-010 Digest (daily/weekly) mode | NOT_STARTED |
