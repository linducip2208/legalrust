# Agent Orchestration (planning; no agents running)
## Registry
Chief-architect, product-manager, domain-architect x25 (one scope each), rust-backend,
leptos-frontend, mysql-architect, ocr-researcher, ai-researcher, security-researcher,
qa-engineer, documentation-engineer, integration-reviewer. Max realistic concurrency: 4-8
agents; 1,000 work packages are QUEUED, not parallel (see TASK-QUEUE).
## Rules (AGENT-RULES)
Isolated scopes (one domain file per agent); never edit shared index/catalog generator outputs
directly — propose via patch files; every contribution needs validation + human/architect review.
## Merge policy / quality gates
See QUALITY-GATES.md: unique IDs, 40/domain, AC+tests present, FK discipline, no RC baselines,
evidence-required states. Concurrency limits + resource notes documented; pretending 1,000
parallel agents is forbidden.
