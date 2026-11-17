# Executive Summary — LEGAL OS

## Vision
Enterprise Legal Operations & Intelligence Platform for Indonesia first, architected for
international expansion, with a 10-20 year lifecycle.

## Scope (planning phase)
Documentation and architecture only. Explicitly out of scope: production code,
migrations, deployments, vendor procurement.

## Key decisions (with rationale)
1. **Modular monolith first (Rust/Axum)** — single deployable, DDD bounded contexts,
   event-driven internally via NATS; extract services only on measured need.
2. **100% Rust application code** — backend Axum, frontend Leptos WASM SSR;
   external systems (MySQL, Redis, NATS, S3/MinIO, AI models) are not Rust and are out of scope of the rule.
3. **MySQL 8.4 LTS + InnoDB** — relational integrity first; no polymorphic associations;
   Tantivy for full-text, optional Qdrant for vectors.
4. **Provider-independent AI/OCR** — trait abstractions; OpenAI/Anthropic/local models
   behind router; OCR engines benchmarked (CER/WER/field accuracy), human-in-the-loop mandatory.
5. **AI never certifies legal validity** — AI drafts/flags only; notary/lawyer approves.
   Electronic signature uses certified PSrE for qualifying acts; notarial authentic deeds
   keep physical-presence guardrail until UUJN/UU ITE harmonization (see compliance gaps).

## Production baselines (verified 2026-11-17, stable only, no RC)
- axum **0.8.9** (MSRV 1.80) | tower-http **0.7** | tokio **1.54.x** | sqlx **0.9**
  (MSRV 1.94; pin `=0.8.6` only if toolchain frozen) | serde **1.x**
- leptos stable **0.8.23** (0.9.x is RC — NOT baseline) | MySQL **8.4 LTS** | Rust stable (current).
- Re-verify versions at implementation kickoff; record in ADR.

## Risks (top)
Regulatory disharmony (UUJN physical presence vs UU ITE e-docs; Pasal 5(4) UU ITE exclusion
for notarial deeds) caps e-sign/cyber-notary scope. UU PDP full enforcement since Oct 2024
makes notary-as-controller obligations (ROPA/DPIA/breach 72h) mandatory. OCR/AI accuracy on
Indonesian documents requires measured benchmarks before SLA claims.

## Readiness
Planning foundation complete; implementation readiness gated (see 20-PROGRESS). No readiness
claimed without evidence.
