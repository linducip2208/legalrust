# AGENT_REGISTRY
| Agent | Scope | Concurrency |
|---|---|---|
| chief-architect | ADR + cross-domain coherence | 1 |
| product-manager | PRD + story map + rules | 1 |
| domain-architect-D01..D25 | single domain file only | up to 4 parallel |
| rust-backend | module contracts, NATS schemas | 1 |
| leptos-frontend | design system conformance | 1 |
| mysql-architect | DDL review, index strategy | 1 |
| ocr-researcher | benchmarks, corpus | 1 |
| ai-researcher | evals, safety | 1 |
| security-researcher | ASVS, threat models | 1 |
| qa-engineer | traceability, gates | 1 |
| documentation-engineer | index, validation | 1 |
| integration-reviewer | merge review | 1 |
Actual parallelism capped at 4-8; queue holds 1,000 packages.
