# OWASP ASVS MAPPING (planning → verification activities at implementation)
- 2.x AuthN (OIDC/MFA/WebAuthn) → D16/D22, verify: auth flows + MFA bypass tests. - 3.x Session (rotation/anomaly) → D16/D24.
- 4.x Access (RBAC/scopes/ABAC/break-glass) → D16+D17+every domain; verify: horizontal/vertical traversal tests per endpoint (150).
- 5.x Validation (serde/DTO/SQLx params) → D08/D21; verify: fuzz + injection suites. - 6.x Crypto (AES-256/KMS/TLS1.2+) → D22/D01.
- 7.x Errors/logging (PII redaction, no stack leaks) → D22/D05. - 8.x Data protection (retention/hold/purge approvals) → D11/D04.
- 9.x Comms (TLS, webhooks HMAC) → D21/D18. - 10.x Malicious (AV-scan, upload gates) → D04. - 11.x Business logic (COI, quorum, trust-nonnegative) → D02/D09/D14.
- 12.x Files (integrity hash, lifecycle) → D04/D06. - 13.x API (rate-limit, versioning, sandbox) → D21. - 14.x Config (secrets, flags) → D25.
Each maps to QA suites in 16-TESTING-QA; pen-test + DAST at hardening phase.
