# API Architecture (planning)
REST v1 (`/api/v1/<domain>/...`), JSON (serde), cursor pagination, problem-details errors,
idempotency-key on POST/PATCH, optimistic locking (If-Match: row_version), RBAC scopes
(`perm:<domain>:<action>`), tenant/office claim enforcement server-side.
Auth: OIDC SSO primary, JWT access (short) + rotating refresh, MFA step-up for sensitive ops,
service accounts via OAuth2 client-credentials, HMAC request signing for webhooks.
Events: NATS subjects `legal.<tenant>.<domain>.<event>.v1`, schema-registry validated,
webhook dispatcher with retry/DLQ. OpenAPI published from code annotations at implementation;
sandbox + changelog + deprecation policy. Rate plans per tenant; abuse detection.
