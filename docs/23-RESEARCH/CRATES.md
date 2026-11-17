# Crate & Platform Research (verified 2026-11-17 via crates.io; re-verify at kickoff)
- axum **0.8.9** stable (MSRV 1.80). tower-http **0.7**. tokio **1.54.x** (MSRV 1.85).
- sqlx **0.9** (MSRV 1.94; fallback `=0.8.6` if toolchain frozen). serde **1.x**. serde_json 1.x.
- leptos stable **0.8.23** REQUIRED baseline; 0.9.x is RC — excluded from production path.
- tailwindcss (via leptos/tailwind integration) + SSR; toolchain: Rust stable current.
- MySQL **8.4 LTS**, InnoDB. Redis 7/8, NATS 2.x, MinIO (S3), Tantivy 0.2x, Qdrant optional.
- Dioxus Mobile: evaluated — Rust codebase, mobile targets experimental, store/API friction;
  decision: document only; choose web-first responsive + PWA, revisit native per market; NO Dart.
- OCR: ocrs (pure Rust + ONNX dep), oar-ocr (pipeline parts), Tesseract bindings = **native
  libtesseract external dep (not pure Rust)** — allowed as infra, must be declared; cloud/local
  vision fallback via AI router. Benchmarks required before engine commitment.
- AI: OpenAI + Anthropic + local (ollama/llama.cpp-class) behind trait; no training on client data
  without consent; eval harness mandatory.
Risks: Leptos 0.9 churn, sqlx MSRV jump, Tesseract native packaging, Qdrant ops cost.
