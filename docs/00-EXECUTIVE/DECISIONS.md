# Architecture Decision Records (index)
- ADR-001 Modular monolith first; service extraction on SLO/coupling triggers.
- ADR-002 Axum + Tokio + Tower + SQLx + Serde backend baseline (versions above).
- ADR-003 Leptos 0.8.x stable baseline; SSR + WASM; no 0.9 RC in production path.
- ADR-004 MySQL 8.4 LTS, InnoDB, strict FK, no polymorphic associations.
- ADR-005 NATS (events) + Redis (cache/locks) + S3/MinIO (objects) + Tantivy (search), Qdrant optional.
- ADR-006 Dioxus Mobile evaluated, documented; no Dart; native shell if webview insufficient.
- ADR-007 OCR/AI behind traits; human review mandatory; AI never certifies validity.
- ADR-008 Tenant/office scoping in every query; legal-hold overrides deletion.
Full log lives in 06-SYSTEM-ARCHITECTURE/ADR.md.
