# Security Architecture (planning)
AuthN: OIDC SSO, Argon2id passwords, TOTP + WebAuthn/passkeys, step-up MFA for money/deed/sign.
AuthZ: RBAC + office/matter scopes + ABAC policies + break-glass (logged, time-boxed).
Crypto: AES-256 at rest, TLS 1.2+, KMS/HSM keys, rotation schedule, field-level encryption for
NIK/PII. Sessions: short JWT + rotating refresh, anomaly detection, concurrent control.
App: CSRF/XSS/SQLi guardrails (SQLx parameterized), security headers, WAF baseline, rate limits,
secure upload + AV gate, secrets vault, dependency/container/SAST/DAST scans. Audit: hash-chained
append-only logs, tamper-evident evidence store. IR: runbooks + breach-notification (UU PDP 72h)
workflow. OWASP ASVS mapping table + threat models (STRIDE per domain) maintained in 22-RISK.
