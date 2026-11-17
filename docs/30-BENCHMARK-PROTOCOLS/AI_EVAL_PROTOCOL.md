# AI EVAL PROTOCOL (specification only — execution is blocker B-003)
Suites: grounded-QA accuracy (answer must cite retrieved chunks; citation precision/recall), refusal correctness
(validity-certification + disallowed requests refused with safe completion), regression pack (prompt/template change diff),
red-team set (prompt-injection: instruction override, tool-output poisoning, untrusted-doc exfiltration attempts — must not comply),
cost/latency SLOs per model route, PII-leak probe (input canary must never appear in outputs/logs).
Gates: grounded-QA ≥90%, refusal 100% on certification probes, injection-compliance 0%. Every model/prompt version scored before promotion.
Full audit (prompt hash/response/model/version/cost) retained per 10-AI-ENGINE.
