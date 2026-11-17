#!/usr/bin/env python3
"""Part 4: new docs structure (00-16), progress system, 1000-workstream registry."""
from pathlib import Path
import json
from collections import Counter
R = Path(r"D:\project laravel\legalrust")
cat = json.loads((R/"docs/03-FEATURE-CATALOG"/"feature-catalog.json").read_text(encoding="utf-8"))
def w(rel, text):
    p = R/rel; p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.strip()+"\n", encoding="utf-8", newline="\n")

idx = {
 "00-overview": "Repo map + status. Planning baseline lives in docs/00-EXECUTIVE..docs/34-PLANNING-EXT (25 domains, 1000 features, audit score 81/STOP -> implementation authorized by owner directive 2026-11-17).",
 "01-product-requirements": "PRD: docs/02-MASTER-PRD + 03-FEATURE-CATALOG (1000 specified features) + 04-USER-STORIES + 05-BUSINESS-RULES.",
 "02-system-architecture": "See docs/06-SYSTEM-ARCHITECTURE + 26-ARCHITECTURE-DECISIONS (ADR-001..010). Modular monolith, DDD, Axum/SQLx/Leptos.",
 "03-database-design": "Logical: docs/07-DATABASE-ARCHITECTURE (+ERD/dictionary appendix). Physical: migrations/0001-0003. MySQL 8.4 LTS.",
 "04-api-specification": "Conventions docs/08-API-ARCHITECTURE. Implemented surface: openapi.yaml in this folder (health/version/login-501).",
 "05-ui-ux": "System docs/13-UI-UX + inventory 31-DETAILED-DESIGN/SCREEN_INVENTORY.md. Implemented: apps/web SSR shell (Phase 5 builds full UI).",
 "06-security": "Docs docs/14-SECURITY + 32-ASSURANCE (STRIDE/ASVS). Implemented: Argon2id, JWT, RBAC map, audit writer.",
 "07-ai-architecture": "Docs docs/10-AI-ENGINE + 30-BENCHMARK-PROTOCOLS/AI_EVAL_PROTOCOL.md. Implemented: provider trait, token accounting, injection guard.",
 "08-ocr-architecture": "Docs docs/09-OCR-ENGINE + OCR_BENCHMARK_PROTOCOL.md. Implemented: engine trait, confidence bands; Tesseract blocked pending native lib.",
 "09-legal-workflows": "Docs docs/12-LEGAL-WORKFLOWS + 34-PLANNING-EXT swimlanes W1-W5 (steps bound to feature IDs). Implemented: deed state machine.",
 "10-testing": "Strategy docs/16-TESTING-QA. Implemented: unit tests per crate + API tests; live-DB and benchmark suites blocked (see 13-development-progress/BLOCKERS.md).",
 "11-deployment": "See deploy/README.md + Dockerfile.server + docker-compose.yml + CI .github/workflows/ci.yml.",
 "12-user-manual": "Manuals ship with Phase 6+ UI. Portal/client flows specified in D19; no manual written for unbuilt screens (no fake docs).",
 "13-development-progress": "Live progress system (MASTER-PROGRESS, ROADMAP, DAILY-LOG, BLOCKERS, TEST-RESULTS).",
 "14-agent-workstreams": "1000-workstream registry (logical queue, bounded parallel execution) + dependency graph + execution log.",
 "15-architecture-decisions": "Ratified copy of 26-ARCHITECTURE-DECISIONS for implementers (ADR-001..010 proposed -> ratify at milestones).",
 "16-disaster-recovery": "Plan: encrypted MySQL backups + binlog, per-tenant restore tests, S3 versioning, restore drills monthly (see 17-DEVOPS + 21-OPERATIONS). Live drills pending deployment.",
}
for d, desc in idx.items():
    w(f"docs/{d}/README.md", f"# {d}\n{desc}\n")
w("docs/04-api-specification/openapi.yaml", """openapi: 3.0.3
info: { title: LEGALRUST API, version: 0.1.0 }
servers: [{ url: /api/v1 }]
paths:
  /healthz: { get: { responses: { '200': { description: ok } } } }
  /readyz: { get: { responses: { '200': { description: ready }, '501': { description: dependency blocked } } } }
  /api/v1/version: { get: { responses: { '200': { description: version } } } }
  /api/v1/auth/login: { post: { responses: { '422': { description: validation }, '501': { description: user store offline in scaffold } } } }
""")
# progress system
w("docs/13-development-progress/MASTER-PROGRESS.md", """# MASTER PROGRESS (computed from repo, not invented)
- Phase 0 repo audit: IMPLEMENTED (toolchain 1.99 stable, single-commit baseline inspected).
- Phase 1 product design: IMPLEMENTED (PRD + 1000 features + rules, pre-existing docs).
- Phase 2 database design: IN_PROGRESS (logical done; 0001-0003 migrated in code, live-DB test blocked).
- Phase 3 foundation: IMPLEMENTED (workspace, config, logging, CI, error envelope).
- Phase 4 auth: IN_PROGRESS (hash/JWT/RBAC core + tests; DB-backed login BLOCKED pending live MySQL).
- Phase 5 design system: PLANNED (SSR shell only). Phases 6-15: NOT_STARTED except specified cores (DMS metadata, workflow state machine, OCR/AI traits).
Feature states: 1000 specs PLANNED; implemented surface = foundation slice (see TEST-RESULTS).
""")
w("docs/13-development-progress/ROADMAP.md", """# ROADMAP (implementation)
P0 foundation/auth-core (done scaffold) -> P1 live-DB + login + org/RBAC wiring -> P2 DMS + projects/tasks ->
P3 OCR pipeline + review UI -> P4 deeds + approvals + e-sign guardrails -> P5 AI/RAG + billing -> P6 hardening + deploy.
Mirrors docs/18-ROADMAP phases; unblocks B-004 (authorized), next B-001..B-003 in parallel.
""")
w("docs/13-development-progress/DAILY-LOG.md", """# DAILY LOG
## 2026-11-17
- Phase 0 audit done. Workspace scaffolded: 16 crates + 3 apps + 3 migrations + CI + compose.
- Auth core (Argon2/JWT/recovery), RBAC map, storage/OCR/AI/RAG/workflow/notify/report/webhook modules with unit tests.
- Server: health/version/ready/login-501 + request-id + CORS + tracing. Web: SSR shell + render test.
- Pending: cargo test run, clippy, commit + push.
""")
w("docs/13-development-progress/BLOCKERS.md", """# BLOCKERS
- BL-DB-01: no live MySQL in this environment -> migration live-run + DB-backed login marked BLOCKED (code complete, unverified live).
- BL-EXT-01: no Dukcapil/BPN/PSrE/AI credentials -> provider adapters interface-complete, activation BLOCKED.
- BL-LIC-01: LICENSE file withheld pending owner licensing intent (MIT vs proprietary).
""")
w("docs/13-development-progress/TEST-RESULTS.md", """# TEST RESULTS
Pending first full run (updated after `cargo test --workspace`).
""")
# workstreams: map domains -> groups A-I with sizes 80/180/130/130/130/100/100/100/50
group_domains = {"A": ["D25","D17"], "B": ["D16","D03","D04","D14","D15","D08","D09"],
 "C": ["D19","D23","D24","D20"], "D": ["D02","D06","D12","D13"], "E": ["D05"], "F": ["D07"],
 "G": ["D11","D22","D10"], "H": [], "I": ["D21","D18"]}
sizes = {"A":80,"B":180,"C":130,"D":130,"E":130,"F":100,"G":100,"H":100,"I":50}
byd = {}
for f in cat: byd.setdefault(f["domain"], []).append(f)
reg = ["# AGENT-REGISTRY (1000 logical workstreams; bounded parallel execution, max 4-8 concurrent)",
 "Workstream = one feature package (or test/ops slice) with concrete deliverable. No workstream is claimed running unless in EXECUTION-LOG.",
 "", "| Workstream | Group | Deliverable | Status |", "|---|---|---|---|"]
counts = Counter()
for g, n in sizes.items():
    doms = group_domains[g]
    pool = [f for d in doms for f in byd.get(d, [])]
    for i in range(n):
        if g == "H":
            f = cat[(i*10) % len(cat)]; title = f"Test package for {f['id']} {f['title']}"
        elif g == "I" and i >= len(pool):
            title = f"Ops/docs slice I-{i+1:03d} (runbook, dashboard, or integration doc)"
            wid = f"WS-I-{i+1:03d}"; reg.append(f"| {wid} | I | {title} | NOT_STARTED |"); counts[g]+=1; continue
        else:
            f = pool[i % len(pool)]; title = f"{f['id']} {f['title']}"
        wid = f"WS-{g}-{i+1:03d}"
        reg.append(f"| {wid} | {g} | {title} | NOT_STARTED |"); counts[g]+=1
w("docs/14-agent-workstreams/MASTER-1000-AGENTS.md", f"""# MASTER 1000 WORKSTREAMS
Groups: A product/arch (80), B backend/db (180), C frontend/ux (130), D legal workflows (130), E OCR (130),
F AI/RAG (100), G security/compliance (100), H QA (100 test packages), I devops/docs (50).
Assignment: round-robin over mapped domains from the 1000-feature catalog (H = every 10th feature test package).
Execution: bounded parallelism (<=8), file ownership per workstream, integration review required.
Counts: {dict(counts)}.
""")
w("docs/14-agent-workstreams/AGENT-REGISTRY.md", "\n".join(reg))
w("docs/14-agent-workstreams/DEPENDENCY-GRAPH.md", """# DEPENDENCY GRAPH (workstreams)
Order: A-foundation -> B-auth/org -> B-clients/projects -> B-docs -> E-OCR -> D-deeds/PPAT ->
B-workflow/approval -> G-sign/compliance gates -> C-frontend per domain -> F-AI -> I-integrations/ops.
H tests ride along each package; nothing merges with failing tests. Full matrix: docs/27-CROSS-DOMAIN-VALIDATION.
""")
w("docs/14-agent-workstreams/EXECUTION-LOG.md", """# EXECUTION LOG
- 2026-11-17 Phase 0: repo+toolchain audit (single operator + deterministic scripts; no fleet claimed).
- 2026-11-17 Phase 1-3: workspace scaffold + 16 crates + 3 apps + migrations 0001-0003 + CI/compose.
""")
for f in ["MASTER-1000-AGENTS.md"]:
    pass
print("PART4 OK")
