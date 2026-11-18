# FILE-OWNERSHIP (AUDIT-1000 wave 2026-11-18)

Single-writer rule (AGENTS.md): main executor owned all writes this wave.
Read-only subagents never edited files.

| File set | Owner (this wave) | Change |
|---|---|---|
| `migrations/0013_approvals.sql` (new) | main | approvals tables |
| `crates/database/src/ops.rs` | main | ApprovalRow structs, trait + `appr_check` |
| `crates/database/src/ops_memory.rs` | main | 5 appr impls (single-lock) |
| `crates/database/src/ops_mysql.rs` | main | 5 appr impls (single-tx) + row mappers |
| `crates/database/src/lib.rs` | main | re-exports |
| `apps/server/src/lib.rs` | main | 5 handlers + 4 routes + 3 CSV audits |
| `apps/server/tests/api.rs` | main | 3 approvals tests |
| `apps/server/static/manifest.json` + `icon-192/512.png` | main | PWA icons |
| `docs/AUTONOMOUS/FEATURE-REGISTRY.json` | main | 10 feature updates |
| `docs/AUTONOMOUS/AUDIT-1000/*` | main | this wave artifacts |
| All other files | — | untouched (no concurrent workstreams active) |

Next-wave claims must be recorded here before editing.
