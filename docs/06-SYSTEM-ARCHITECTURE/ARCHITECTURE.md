# System Architecture (planning)
## Style
Modular monolith (single Rust binary + Leptos SSR), DDD bounded contexts per domain (D01-D25),
clean-architecture layering inside modules (handler -> service -> repo), ports/traits for
OCR/AI/storage/events. Event-driven via NATS subjects (`legal.<tenant>.<domain>.<event>.v1`);
Redis for cache/locks/rate-limit; S3/MinIO objects; Tantivy search; Qdrant optional vectors.

```mermaid
flowchart LR
  UI[Leptos SSR/WASM] --> API[Axum API + Tower]
  API --> SVC[Domain services D01-D25]
  SVC --> DB[(MySQL 8.4)]
  SVC --> OBJ[(S3/MinIO)]
  SVC --> Q[NATS]
  SVC --> C[(Redis)]
  SVC --> OCR[OCR router]
  SVC --> AI[AI router]
  Q --> W[Workers: OCR/AI/index/notify/billing]
  W --> DB
  W --> OBJ
```

## Request path
```mermaid
sequenceDiagram
  participant U as User
  participant A as Axum+Tower
  participant S as Service
  participant D as MySQL
  U->>A: HTTPS + JWT/OIDC, idempotency-key
  A->>S: validated DTO (serde)
  S->>D: tx + FK-checked writes
  S-->>Q: domain event (NATS)
```

## Workers/idempotency/retries
At-least-once delivery; consumer idempotency keys + dedupe table; exponential backoff w/ jitter;
poison messages to DLQ + error inbox; saga/compensation for cross-module ops (no 2PC).
## Extraction triggers
p95 breach sustained, team contention, blast-radius need, or regulatory isolation — then extract
behind versioned API with dual-run.
## ADR log
See 00-EXECUTIVE/DECISIONS.md. Crate baselines: axum 0.8.9, tower-http 0.7, tokio 1.54.x,
sqlx 0.9 (MSRV 1.94), serde 1.x, leptos stable 0.8.23 (no RC).
