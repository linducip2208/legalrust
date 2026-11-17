# AGENTS.md — working rules for humans and automation
1. 100% Rust application code (backend + frontend). No JS/PHP/Go business logic. Native OCR libs and AI models are external infra behind Rust adapters.
2. Read before editing. Never overwrite working files blindly. One workstream owns one file set at a time.
3. No destructive migrations. New migration per change, forward-only, reviewed.
4. Every multi-step mutation runs in a DB transaction. Every endpoint enforces RBAC server-side.
5. No secrets in code or logs. No fake production data. No TODO in critical paths; errors propagate typed.
6. Mark external-dependent features BLOCKED (501 + docs), never IMPLEMENTED without live verification.
7. Commands: `cargo fmt --all`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (with `SQLX_OFFLINE=true` when no DB).
8. Update docs/13-development-progress after each work package. Progress states: NOT_STARTED PLANNED IN_PROGRESS IN_REVIEW BLOCKED IMPLEMENTED TESTED ACCEPTED.
