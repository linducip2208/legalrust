# Security Self-Audit — 2026-11-17

- **Scope (read):** `apps/server` + all crates (`crates/*`).
- **Scope (code fixes):** `crates/auth`, `crates/application` ONLY. No other Rust files touched; no migrations, no docs besides this file.
- **Method:** manual route-by-route review of `apps/server/src/lib.rs` against handlers, `grep` for SQL construction / secret handling / error mapping, then targeted hardening + unit tests.
- **Toolchain (with `$env:CARGO_TARGET_DIR='target-sec'`):**
  - `cargo fmt -p legalrust-auth -p legalrust-application` — clean.
  - `cargo check -p legalrust-auth -p legalrust-application` — clean.
  - `cargo clippy -p legalrust-auth -p legalrust-application --all-targets -- -D warnings` — clean.
  - `cargo test -p legalrust-auth -p legalrust-application` — **17/17 pass** (auth 12, application 5).
  - `cargo check -p legalrust-server` (+database, document, infrastructure, audit deps) — clean.
  - `cargo check --workspace` — **FAILS in `legalrust-ai` (pre-existing, out of scope):** `hyper::rt::Read/Write` not implemented for `tokio::net::TcpStream` (hyper/tokio version mismatch, 14× E0277). Untouched by this audit; `apps/server` does not depend on it for the audited paths.

## 1. Every `/api` route has `auth_user` + `authorize` — PASS with documented exceptions

Router: `apps/server/src/lib.rs:1986-2091`. Gate primitives: `auth_user` (`:128-174`, JWT verify + `session_live` check, API-token fallback via sha256 hash lookup) and `authorize` (`:175-187`, `Role::parse` → `can(role, perm)`).

| Route(s) | Gate | Note |
|---|---|---|
| `GET /`, `/login`, `/app`, `/healthz`, `/readyz`, `GET /api/v1/version` | none (public) | Intentional: landing/login/shell pages, liveness, version. `readyz` (`:623-633`) returns 501 without DB. |
| `POST /api/v1/auth/register`, `POST /api/v1/auth/login` | `RateLimiter` + `validator::Validate` | Intentional pre-auth; per-email lockout + global limiter (see §3). |
| `POST /api/v1/auth/logout`, `GET /api/v1/orgs/me` | `auth_user` only | Self-scoped; no permission needed. OK. |
| `POST+GET /api/v1/auth/tokens`, `DELETE /api/v1/auth/tokens/{id}` | `auth_user` only, **no `authorize`** | GAP (low): any authenticated user may mint/list/revoke **own** tokens (`:1317-1363`, scoped to `u.claims.sub`). Self-scoped so not privilege escalation, but no `Permission` check. Out of scope (`apps/server`); recommend `authorize` or explicit comment. |
| `POST /api/v1/auth/password` | `auth_user` + api-token refused (`:1378-1380`) | Good: password change blocked for API-token sessions. |
| `POST /api/v1/users/{uid}/roles` | `auth_user` + `Permission::Assign` | Correct; role parsed via allowlist. |
| `GET /api/v1/notifications`, `POST /api/v1/notifications/{id}/read` | `auth_user` only, **no `authorize`** | Acceptable: both scoped to caller (`:1431-1459`, `notify_list(&u.claims.sub)`). No cross-user access. |
| `GET /api/v1/events` (SSE) | `auth_user` only | Acceptable: broadcast stream, no per-user data beyond `notify:{kind}` strings. |
| `POST /api/v1/webhooks/incoming/{provider}` | HMAC (`verify_webhook`, integration secret), **no JWT** | Intentional: third-party callbacks can't bear JWT. Returns 501 when secret unconfigured (`:1258-1262`). Replay protection depends on `x-ts` handling inside `legalrust-integrations` — not re-verified here. |
| `GET /api/v1/shared/{token}`, `GET .../content` | **none (public)** | Intentional capability-URL design (`:1536-1592`): unguessable UUID token + expiry (`share_valid`, `:1521-1535`, 1–720h clamp `:1478`) + revoked check. Expiry parsing accepts 3 formats + epoch; unparseable → treated invalid (fail-closed). |
| All clients/matters/tasks/docs/ocr/templates/deeds/ai/rag/invoices/payments/reports/integrations/jobs/trust/audit routes | `auth_user` + `authorize(Permission::…)` | Verified one by one (`:315-321`, `:364-365`, `:402-403`, `:427-428`, `:447-448`, `:471`, `:500-501`, `:561`, `:586`, `:663-664`, `:682-683`, `:696-697`, `:723-724`, `:746-747`, `:777-778`, `:795-796`, `:807-808`, `:830-831`, `:864-865`, `:899-900`, `:922`, `:949-950`, `:969-970`, `:994-995`, `:1025-1026`, `:1044-1045`, `:1079-1080`, `:1109-1110`, `:1148-1149`, `:1165-1166`, `:1195-1196`, `:1208-1209`, `:1226-1227`, `:1294-1295`, `:1672-1673`, `:1687-1688`, `:1711-1712`, `:1729-1730`, `:1747-1748`, `:1772-1773`, `:1805-1806`, `:1598-1599`). `audit_list` (`:1626-1660`) uses an explicit role allowlist (SuperAdmin/OrgOwner/OfficeAdmin/Auditor) instead of `authorize` — equivalent. |
| `POST /api/v1/tasks/{tid}` (status patch) | `auth_user` + `Permission::Update` | `TaskPatch.status` has **no handler-level allowlist** (`:460-483`); validity relies on the store layer. Residual, out of scope. |

## 2. SQL binds / upload validation / JSON validation — PASS with minor notes

- **SQL:** all queries in `crates/database/src/mysql.rs` and `ops_mysql.rs` use `sqlx::query(..)` + chained `.bind(..)` exclusively (100+ `.bind` sites); `grep` for `format!("SELECT|INSERT|UPDATE|DELETE` in `crates/database/src` returns **zero matches**. No string interpolation of user input into SQL. Transactions: trust post/transfer use `tx` (`ops_mysql.rs:43-62`); multi-step mutations otherwise go through single-statement store calls — full TX audit of every mutation was out of scope for code changes.
- **Uploads:** `legalrust_document::validate_upload` (`crates/document/src/lib.rs:18-26`) enforces an 8-entry MIME allowlist + `size > max` (server `MAX_UPLOAD = 50 MiB`, `apps/server/src/lib.rs:32`, plus `RequestBodyLimitLayer` `:2063-2065`). `create_doc` (`:494-517`) additionally requires 64-char `sha`; `put_content` (`:554-584`) rejects size mismatch and sha256 mismatch before storing. PASS.
- **JSON:** `validator::Validate` derived + checked on `RegisterReq`, `LoginReq`, `ClientReq`, `MatterReq`, `TaskReq`, `DocReq`, `TplReq`, `TplPut`, `DeedFromTpl`, `InvReq`, `PayReq`, `TrustAcctReq/Entry/Transfer`, `PasswordReq`, `TokenReq`, `ChatReq`, `Pagination`. Manually allowlisted instead: `OcrReviewReq.decision` (`:725`), `RoleReq.role` via `Role::parse` (`:1414`), `ShareReq.hours` clamped (`:1478`), `JobReq.kind` non-empty (`:1674`). Residual (out of scope): `TaskPatch.status`, `RagIndexReq`, `RagSearchQ.q` (empty-check only), `DeedTransition.to` (parsed via `DeedState::parse`, `:932-934` — fail-closed) lack `Validate` derives; all fail closed at handler or store layer, none injectable into SQL.

## 3. Login lockout + rate limit; no internal leakage — PASS (hardened)

- **Layers:** global `RateLimiter` 20/min (`apps/server/src/lib.rs:39-63`, applied `register`/`login` `:227`,`:262`); per-email lockout 5 attempts / 5 min window returning `RateLimited` (→ 429, no user-enumeration oracle) (`:35-36`, `:102-117`, `:268-289`); in-crate `LoginGuard` (`crates/auth/src/lib.rs:211-283`) with prune/lock/reset semantics + 3 pre-existing tests.
- **Error bodies:** `AppError::Internal` displays as fixed `"internal error"` (`crates/application/src/lib.rs:26-27`) and now **renders via `public_message()`** (`:68-73`, wired into `IntoResponse` `:75-80`), so even handlers that forward `e.to_string()` of `sqlx`/`StoreError::Db` chains (15 sites, e.g. `apps/server/src/lib.rs:332`, `:639`) can never leak DB internals to the wire. Verified by new test `internal_never_leaks_db_detail` with a synthetic `SELECT … sqlx mysql` payload.
- **Residual:** `BlockedExternal(msg)` and `Locked(msg)` still echo caller-supplied text (e.g. `ocr engine unavailable (job {id}, type …, ~{bytes} …)` `:768-771`). Content is operational, not secret; accepted. `Locked` callers use static strings.

## 4. Secrets: env-only, redacted Debug, no secret in logs/tests — PASS (hardened)

- `JWT_SECRET` / `INTEGRATION_SECRET` read from env only (`apps/server/src/main.rs:13`, `:55-57`); empty JWT secret → all auth endpoints 501 via `check_secret` (`crates/auth/src/lib.rs:116-128`); `Config::from_env` tracks `jwt_secret_set` (`crates/infrastructure/src/lib.rs:9-23`). No secret literals in code; tests use in-memory `b"test-secret…"` / `dev_state` only.
- **Fixed:** `AuthError::Token` no longer embeds `jsonwebtoken` error text (`crates/auth/src/lib.rs:15-16`, `:58-74`) — prevents lib/detail leakage into logs and (via `map_store`-style forwarding) error chains.
- **Fixed:** `Session` now has a manual `Debug` impl redacting `token` (`crates/auth/src/lib.rs:100-109`); previously `#[derive(Debug)]` would print the live JWT in any debug log.
- **Fixed:** `check_secret` now also rejects secrets shorter than 32 bytes (`MIN_SECRET_LEN`, `:26`, `:122-126`).
- **Residual (out of scope, `crates/database`):** `UserRow` derives `Debug` including `pw_hash` (`crates/database/src/store.rs:11-19`). The hash is Argon2 (not plaintext) so severity is low, but it should get the same redaction treatment. `Claims` derives `Debug` with `sub/org/roles/sid` — identifiers, not secrets; acceptable.

## 5. Fixes applied (in-scope crates only)

`crates/auth/src/lib.rs`:
1. Password policy `8 → 12` chars: `MIN_PASSWORD_LEN` (`:21`), `validate_password()` (`:31-36`), enforced in `register` (`:141`); `AuthError::WeakPassword` (`:13-14`).
2. Session TTL `24h → 8h`: `SESSION_TTL_HOURS` (`:24`), `expiry()` (`:111-114`).
3. JWT secret minimum 32 bytes in `check_secret` (`:116-128`).
4. `AuthError::Token` unit variant — no jwt-error text (`:15-16`, `:64`, `:73`).
5. `Session` Debug redacts `token` (`:100-109`).

`crates/application/src/lib.rs`:
6. `AppError::public_message()` — `Internal` always renders `"internal error"` (`:68-73`); `IntoResponse` uses it (`:77`).

**New unit tests (9):** auth — `password_policy_rejects_short`, `secret_must_be_32_bytes`, `session_ttl_is_eight_hours`, `session_debug_redacts_token`, `token_error_carries_no_detail`, `register_rejects_weak_password` (+ updated `register_then_login_memory` to 12-char password / 32-byte secret); application — `internal_never_leaks_db_detail`, `validation_messages_stay_client_safe`, `blocked_external_maps_to_501`. **Total: 17/17 green** (auth 12, application 5).

## 6. Residual risks / leftovers (honest, mostly out of scope)

1. `apps/server` token endpoints (`/api/v1/auth/tokens*`) and `notify_*` lack `authorize` — self-scoped, low risk, but should get explicit `Permission` checks or design comments.
2. `UserRow::pw_hash` Debug-printable (`crates/database/src/store.rs:11-19`) — needs redaction, out of scope.
3. `TaskPatch.status` (and a few small DTOs) lack handler-level allowlists — fail closed downstream, but allowlist-at-edge would be better.
4. Webhook replay protection depends on `x-ts` enforcement inside `legalrust-integrations::verify_webhook` — not re-verified.
5. `RateLimiter` and `login_failures` are in-process `Mutex<HashMap>` — correct for single instance, but no distributed lockout across replicas.
6. `login`/`register` still enforce only `length(min = 8)` at the axum `Validate` layer (`apps/server/src/lib.rs:213-214`, `:220-221`, `:1367-1370`); the 12-char policy is enforced one layer down in `crates/auth`. Works, but the two layers disagree — server DTOs should be raised to 12 (out of scope).
7. `legalrust-ai` does not compile (`hyper`/`tokio` trait mismatch, pre-existing) — unrelated to this audit, flagged so CI owners notice.
8. Argon2 uses default params; no pepper, no `zeroize` on plaintext passwords in memory — acceptable for current threat model, noted.
