#!/usr/bin/env python3
"""Scaffold LEGALRUST Rust workspace + root governance + new docs structure. Real code only."""
from pathlib import Path
R = Path(r"D:\project laravel\legalrust")
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

ED = 'edition = "2021"'
def crate_toml(name, deps, extra=""):
    d = "\n".join(f'{k} = {v}' for k, v in deps.items())
    return f"""[package]
name = "{name}"
version = "0.1.0"
{ED}

[dependencies]
{d}
{extra}"""

BASE = {"serde": '{ version = "1", features = ["derive"] }', "serde_json": '"1"'}
TOKIO = {"tokio": '{ version = "1", features = ["full"] }'}

# ---------- root ----------
w("rust-toolchain.toml", '[toolchain]\nchannel = "stable"')
w("Cargo.toml", """[workspace]
resolver = "2"
members = ["apps/*", "crates/*"]

[workspace.lints.rust]
unsafe_code = "forbid"
""")
w(".gitignore", "target/\n.env\n*.log\ndist/\n")
w(".env.example", """PORT=8080
DATABASE_URL=mysql://root:legalrust@127.0.0.1:3306/legalrust
REDIS_URL=redis://127.0.0.1:6379
S3_ENDPOINT=http://127.0.0.1:9000
S3_BUCKET=legalrust
JWT_SECRET=change-me-in-production
RUST_LOG=legalrust=debug,tower_http=info
""")
w("AGENTS.md", """# AGENTS.md — working rules for humans and automation
1. 100% Rust application code (backend + frontend). No JS/PHP/Go business logic. Native OCR libs and AI models are external infra behind Rust adapters.
2. Read before editing. Never overwrite working files blindly. One workstream owns one file set at a time.
3. No destructive migrations. New migration per change, forward-only, reviewed.
4. Every multi-step mutation runs in a DB transaction. Every endpoint enforces RBAC server-side.
5. No secrets in code or logs. No fake production data. No TODO in critical paths; errors propagate typed.
6. Mark external-dependent features BLOCKED (501 + docs), never IMPLEMENTED without live verification.
7. Commands: `cargo fmt --all`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (with `SQLX_OFFLINE=true` when no DB).
8. Update docs/13-development-progress after each work package. Progress states: NOT_STARTED PLANNED IN_PROGRESS IN_REVIEW BLOCKED IMPLEMENTED TESTED ACCEPTED.
""")
w("README.md", """# LEGALRUST — Enterprise Legal Operations Platform
Rust workspace (Axum + Tokio + SQLx + MySQL 8.4 + Leptos). Indonesia-first, expansion-ready.
Status: foundation scaffold (Phases 0-3). Planning baseline: `docs/` (25 domains, 1000 features).
Quickstart: `cp .env.example .env`, `docker compose up -d mysql`, `sqlx migrate run`, `cargo run -p legalrust-server`.
Layout: `apps/server|web|worker`, `crates/*` domain modules, `migrations/` forward-only SQL, `docs/` planning + progress.
""")
w("CHANGELOG.md", """# Changelog
## [Unreleased]
- Workspace scaffold: 16 library crates + server/worker/web apps, Axum health/version API, JWT/Argon2 auth core, RBAC map, storage/OCR/AI/RAG/workflow/notification/reporting/integration modules with unit tests.
- MySQL 8.4 migrations 0001-0003 (foundation, core, deeds/finance). Docker Compose infra. CI workflow.
- Planning baseline preserved under docs/ (1000 features, audit, scorecard 81/STOP).
""")
w("CONTRIBUTING.md", """# Contributing
Branch per workstream, conventional commits, `cargo fmt` + clippy-clean + tests green before review.
Migrations need a second reviewer. Never commit secrets. Update progress docs with every package.
""")
w("SECURITY.md", """# Security
Report vulnerabilities privately to the maintainers (see repo contacts). Controls: Argon2id, OIDC-ready JWT,
TOTP-capable auth core, RBAC+scopes enforced server-side, AES-256/KMS expected at deploy, TLS 1.2+, PII redaction,
hash-chained audit, AV-gated uploads (adapter), rate limits. Full mapping: docs/14-SECURITY + 32-ASSURANCE/ASVS_MAPPING.md.
""")
w(".github/workflows/ci.yml", """name: ci
on: [push, pull_request]
jobs:
  quality:
    runs-on: ubuntu-latest
    env: { SQLX_OFFLINE: "true" }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - run: cargo fmt --all -- --check
      - run: cargo check --workspace --locked
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace --locked
  migrate:
    runs-on: ubuntu-latest
    services:
      mysql: { image: "mysql:8.4", env: { MYSQL_ROOT_PASSWORD: legalrust, MYSQL_DATABASE: legalrust_test }, ports: ["3306:3306"], options: --health-cmd="mysqladmin ping -h localhost" --health-interval=10s --health-timeout=5s --health-retries=5 }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install sqlx-cli --no-default-features --features mysql --locked
      - run: sqlx database create --database-url mysql://root:legalrust@127.0.0.1:3306/legalrust_test && sqlx migrate run --source migrations --database-url mysql://root:legalrust@127.0.0.1:3306/legalrust_test
""")
w("docker-compose.yml", """services:
  mysql: { image: "mysql:8.4", environment: { MYSQL_ROOT_PASSWORD: legalrust, MYSQL_DATABASE: legalrust }, ports: ["3306:3306"], volumes: ["mysql-data:/var/lib/mysql"] }
  redis: { image: "redis:7-alpine", ports: ["6379:6379"] }
  minio: { image: "minio/minio:latest", command: server /data, environment: { MINIO_ROOT_USER: legalrust, MINIO_ROOT_PASSWORD: legalrust123 }, ports: ["9000:9000"] }
  nats: { image: "nats:2-alpine", ports: ["4222:4222"] }
volumes: { mysql-data: {} }
""")
print("ROOT OK")
