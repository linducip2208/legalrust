# QA Strategy (planning)
Levels: unit (services/rules), integration (API+DB tx), contract (OpenAPI), DB (FK/migration),
UI (Leptos component + a11y axe), security (ASVS/pen-test), perf/load (k6-style SLOs),
OCR benchmarks (CER/WER/field-acc, corpus-versioned), AI evals (groundedness/citation/refusal),
DR drills. Every feature: AC1-5 + TC-P/N + EC mapped in catalog = traceability matrix source.
Gates: coverage thresholds set at implementation; no gate pass without evidence artifacts.
