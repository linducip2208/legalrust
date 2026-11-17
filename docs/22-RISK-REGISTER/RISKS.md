# Risk Register (top; full per-feature risks in catalog)
R1 Regulatory disharmony (UUJN vs UU ITE 5(4)) limits e-akta — Mit: guardrails + counsel track; Sev High.
R2 UU PDP breach exposure (notary as controller) — Mit: consent/ROPA/DPIA/72h workflow; High.
R3 OCR accuracy below deed-grade — Mit: benchmark gates + HITL; High.
R4 AI hallucination / validity-certification misuse — Mit: refusal policy + approval gates; High.
R5 Scope creep (1,000 feats) — Mit: phased roadmap, packages queued; Medium.
R6 Secret/cost sprawl (AI/OCR/providers) — Mit: vault + metering + budgets; Medium.
R7 Data loss — Mit: encrypted backups + restore drills; High.
R8 Mobile offline conflicts — Mit: CRDT-ish rules + review; Medium.
Each risk: owner, trigger metric, mitigation, contingency. STRIDE models per domain at implementation.

## Blocker-linked entries (acceptance readiness — Nov 2026; all OPEN/BLOCKED, no legal conclusions)
Format: Likelihood / Impact / Mitigation / Owner. Evidence file cited per row.
| ID | Risk | Likelihood | Impact | Mitigation (honest status) | Owner | Evidence |
|----|------|------------|--------|----------------------------|-------|----------|
| B-001 | Launch with real client data without counsel sign-off (UUJN/ITE/PDP/PSrE exposure) | High (all checklist boxes unchecked) | Critical | STOP launch with real data; route Q1–Q6 via sign-off pack; keep conservative guardrails until written counsel opinion | CPO | `docs/33-LEGAL-REVIEW/COUNSEL-CHECKLIST.md`; `docs/33-LEGAL-REVIEW/SIGNOFF-PACK.md`; `docs/25-GAP-ANALYSIS/GAP_REGISTER.md` (G-011); `docs/24-AUDIT/MASTER_AUDIT_REPORT.md` (F6) |
| B-002 | OCR output below deed-grade relied on without measured benchmark | High (no scores exist) | High | Forbid reliance on unmeasured OCR for deeds; execute `OCR_BENCHMARK_PROTOCOL` on ID corpus + HITL gate before acceptance | OCR researcher | `docs/30-BENCHMARK-PROTOCOLS/OCR_BENCHMARK_PROTOCOL.md`; `docs/25-GAP-ANALYSIS/GAP_REGISTER.md` (G-009); `docs/24-AUDIT/MASTER_AUDIT_REPORT.md` (F5) |
| B-003 | AI hallucination/misuse without measured eval (groundedness/refusal) | High (no eval run) | High | Keep refusal + approval gates; execute `AI_EVAL_PROTOCOL` before acceptance; no validity-certification claims | AI architect | `docs/30-BENCHMARK-PROTOCOLS/AI_EVAL_PROTOCOL.md`; `docs/25-GAP-ANALYSIS/GAP_REGISTER.md` (G-010); `docs/24-AUDIT/MASTER_AUDIT_REPORT.md` (F5) |
| BL-DB-01 | Migration/login failure on live MySQL (never live-run in this env) | High (no live MySQL here) | High | Code complete but marked BLOCKED; require live-DB migration run + DB-backed login verification before production | Backend lead | `docs/13-development-progress/BLOCKERS.md`; `docs/13-development-progress/ACCEPTANCE.md` (MISSING live-DB run) |
| BL-EXT-01 | External-provider activation failure (no Dukcapil/BPN/PSrE/AI credentials) | High (no credentials) | High | Adapters interface-complete only; activation BLOCKED; no live e-sign/registry claims until credentials + verification | Integration lead | `docs/13-development-progress/BLOCKERS.md`; `docs/13-development-progress/ACCEPTANCE.md` (PARTIAL: OCR/AI live) |
| BL-LIC-01 | Distribution/licensing dispute (LICENSE intent undecided MIT vs proprietary) | Medium | Medium | LICENSE file withheld; no distribution beyond agreed scope until owner records intent | Owner | `docs/13-development-progress/BLOCKERS.md` |
