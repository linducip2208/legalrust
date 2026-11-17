# LEGAL OS — Documentation Index (Planning Phase)

> PLANNING ONLY. No application source code has been implemented.
> Rust 100% rule applies to future application code, not to external infra/models/OS.

| # | Folder | Document |
|---|--------|----------|
| 00 | 00-EXECUTIVE | Vision, scope, decisions, glossary |
| 01 | 01-PRODUCT-VISION | Product vision, personas, positioning |
| 02 | 02-MASTER-PRD | Master PRD + functional scope |
| 03 | 03-FEATURE-CATALOG | 25 domains x 40 features = 1,000 specs + 10,000 expansions + JSON/CSV |
| 04 | 04-USER-STORIES | Epic/story map + acceptance map |
| 05 | 05-BUSINESS-RULES | Business rules register BR-001.. |
| 06 | 06-SYSTEM-ARCHITECTURE | Modular monolith, DDD, Mermaid diagrams |
| 07 | 07-DATABASE-ARCHITECTURE | MySQL 8.4 logical design, DDL examples, index strategy |
| 08 | 08-API-ARCHITECTURE | REST conventions, auth, idempotency, events |
| 09 | 09-OCR-ENGINE | OCR PRD, engine benchmark, CER/WER methodology |
| 10 | 10-AI-ENGINE | Provider abstraction, RAG, safety, audit |
| 11 | 11-DOCUMENT-ENGINE | Lifecycle, versioning, retention, storage |
| 12 | 12-LEGAL-WORKFLOWS | Notary/PPAT/litigation/corporate workflows |
| 13 | 13-UI-UX | Design system, screens, journeys, tokens |
| 14 | 14-SECURITY | Auth, crypto, OWASP ASVS mapping, IR |
| 15 | 15-COMPLIANCE | UUJN/UU PDP/UU ITE/PPAT matrix + gaps |
| 16 | 16-TESTING-QA | Test strategy + traceability |
| 17 | 17-DEVOPS | CI/CD, envs, observability, backup |
| 18 | 18-ROADMAP | Phases, milestones, critical path |
| 19 | 19-AGENT-ORCHESTRATION | Agent registry, task queue, quality gates |
| 20 | 20-PROGRESS | Progress tracker + doc completeness |
| 21 | 21-OPERATIONS | Runbooks, SLOs, support |
| 22 | 22-RISK-REGISTER | Risks with mitigation |
| 23 | 23-RESEARCH | Crate research (verified 2026-11), OCR/AI/mobile notes |

Machine-readable catalog: `03-FEATURE-CATALOG/feature-catalog.json`, `feature-catalog.csv`,
`expansions/expansion-catalog.json`, `expansions/expansion-catalog.csv`.
Validation: see `20-PROGRESS/VALIDATION-REPORT.md`.
