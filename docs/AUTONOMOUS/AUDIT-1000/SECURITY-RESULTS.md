# SECURITY-RESULTS (self-audit 2026-11-18 + recurring controls)

Method: repo-wide grep (secrets, TODO/panic, unsafe), route/RBAC review,
CORS/CSRF analysis, dependency posture, log-PII check, git hygiene.

## Findings & disposition

1. No hardcoded secrets in non-test source (1 test sentinel only, benign).
   `.env`/`.env.prod` git-ignored; examples use CHANGE-ME. → CLOSED (hygiene).
2. No `TODO/FIXME/todo!/unimplemented!/panic!` in production paths
   (panics only in tests; Mutex `unwrap()` on poison — standard). → CLOSED.
3. `unsafe_code = #![forbid]` workspace lint + clippy clean. → CLOSED.
4. CORS was `permissive` → **FIXED**: env-allowlist `CORS_ORIGINS`,
   default same-origin-only (Bearer auth, no cookies → CSRF risk low). → CLOSED.
5. JWT min length 32 enforced (`check_secret`); sessions revocable; API tokens
   hashed; brute-force lockout 5/5min; Argon2id min 12 chars. → CLOSED.
6. RBAC per endpoint incl. new approvals (Create/View/Approve gates + step
   role-match incl. notary-mismatch 403 test). Tenant scoping on all new
   queries; `inv_owner_org` used only for signed-callback mapping. → CLOSED.
7. No PII in logs (operational messages only); API keys absent from all error
   strings (tested: `api_key_never_appears_in_errors`). → CLOSED.
8. SQL injection: sqlx binds everywhere; no string-built SQL found. → CLOSED.
9. Upload pipeline: MIME allowlist + size cap + SHA-256 pin enforced. → CLOSED.
10. External pentest: **OPEN (recommended, not blocking)** — no third party
    engaged. SSRF surface limited (server-side calls only to configured
    OCR/AI/Tripay endpoints).

## Verdict

No critical/high issues open. Rating: **self-audit PASS**; external
pentest queued (see RESUME.md). Do not claim certified/100.
