# RUST ARCHITECTURE AUDIT — verdict: SOUND (86/100)
Verified: modular monolith + DDD bounded contexts + clean layering (handler→service→repo), Axum extractors,
Tower middleware, serde DTOs, SQLx compile-checked queries + transactions, Tokio async + pooling, NATS workers
with idempotency/retry-DLQ, OTel instrumentation, versioned REST. Compatibility: axum 0.8.9 MSRV 1.80;
sqlx 0.9 needs Rust ≥1.94 (documented fallback =0.8.6); tokio 1.54 MSRV 1.85; Leptos 0.8.23 stable, 0.9 RC excluded.
Deductions: no physical load profile yet (-6), worker autoscale thresholds unmeasured (-4), Leptos 0.9 migration path unwritten (-4).
100%-Rust scope ruling (ADR-010): app code only; native OCR lib + infra out of scope by definition. No Dart anywhere.
