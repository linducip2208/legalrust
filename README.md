# LEGALRUST — Enterprise Legal Operations Platform
Rust workspace (Axum + Tokio + SQLx + MySQL 8.4 + Leptos). Indonesia-first, expansion-ready.
Status: foundation scaffold (Phases 0-3). Planning baseline: `docs/` (25 domains, 1000 features).
Quickstart: `cp .env.example .env`, `docker compose up -d mysql`, `sqlx migrate run`, `cargo run -p legalrust-server`.
Layout: `apps/server|web|worker`, `crates/*` domain modules, `migrations/` forward-only SQL, `docs/` planning + progress.
