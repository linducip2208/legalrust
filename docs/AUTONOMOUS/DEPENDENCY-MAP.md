# DEPENDENCY-MAP — LegalRust (versions from Cargo.lock, 2026-11-18)

## 1. Per-crate external deps (from each Cargo.toml; versions = locked)
| Crate | External deps (locked versions) |
|---|---|
| apps/server | axum 0.8.9, tokio 1.54.1 (full), tower/tower-http 0.5/0.7, serde 1, serde_json 1, sqlx 0.9.x (mysql), uuid 1, validator 0.20, chrono 0.4.45, anyhow 1, tracing 0.1, tokio-stream 0.1 |
| apps/worker | tokio 1.54.1 (full), tracing 0.1, sqlx 0.9.x (mysql) |
| apps/web | leptos 0.8.23 (ssr) |
| apps/mobile | serde 1, serde_json 1, uuid 1, chrono 0.4.45 |
| crates/auth | argon2 0.5.3, rand_core 0.6, jsonwebtoken 9.3.1, chrono 0.4.45, uuid 1, hex 0.4, thiserror 2, anyhow 1 |
| crates/database | sqlx 0.9.x (runtime-tokio, mysql, migrate, chrono, uuid), tokio 1.54.1 (full), async-trait 0.1, thiserror 2, serde 1, chrono, uuid 1 |
| crates/ai | hyper 1 + hyper-util 0.1 (client, http1), http 1, http-body-util 0.1, bytes 1, tokio full, async-trait, anyhow, validator 0.20 |
| crates/integrations | hmac 0.12, sha2 0.10, hex 0.4 |
| crates/document | sha2 0.10, hex 0.4, thiserror 2 |
| crates/{domain, application*, audit, notification, ocr, rag, reporting, storage, workflow, authorization, infrastructure} | serde/serde_json/uuid/chrono baseline; + thiserror/anyhow/axum/validator/async-trait/tokio/tracing(-subscriber)/sqlx per crate (see §2) |

(\* application also: axum 0.8, validator 0.20, thiserror 2, anyhow 1.)

## 2. Key locked versions (Cargo.lock spot-checks)
axum 0.8.9 · tokio 1.54.1 · sqlx 0.9.x family (0.9.0/0.9.9 entries) · jsonwebtoken 9.3.1 · argon2 0.5.3 · leptos 0.8.23 · chrono 0.4.45 · validator 0.20.x · uuid 1.x · tower-http 0.7.x · hyper 1.x · hmac 0.12 · sha2 0.10 · async-trait 0.1 · thiserror 2.x · anyhow 1.x.

## 3. Feature flags in use
- tokio: `full` (server, worker, database, ai, audit, storage). sqlx: `runtime-tokio`, `mysql` (+ `migrate`, `chrono`, `uuid` on database). leptos: `ssr`. uuid: `v4` (+ `serde` most crates). chrono: `serde` most crates. tower-http: `trace`, `cors`, `request-id`, `limit`, `set-header`. validator: `derive`. hyper-util: `tokio`. tracing-subscriber: `env-filter`, `fmt`. rand_core: `getrandom`.

## 4. Build notes (Windows / locked-exe workaround)
- `cargo test --workspace` against default `target/debug` FAILS on this machine when `legalrust-server.exe` is running/locked (`Access is denied (os error 5)` removing the exe). Workaround used by this audit: `CARGO_TARGET_DIR=target-audit` isolates all artifacts; combined with `SQLX_OFFLINE=true` (no DB needed — compile-time query checking is not used; all SQL is runtime `query()` + `DATE_FORMAT` strings, so offline builds pass).
- Read-only verification command: `$env:SQLX_OFFLINE='true'; $env:CARGO_TARGET_DIR='target-audit'; cargo test --workspace --offline` → 197 passed, 0 failed, 1 ignored.
- `unsafe_code = "forbid"` at workspace lints. No build scripts, no native link deps — OCR/AI/e-sign natives stay outside the Rust build (external infra).
