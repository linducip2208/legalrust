# AI Engine PRD (planning)
Provider-independent core: `AiProvider` trait (chat/extract/embed), model router
(cost/quality/latency/data-class), token metering + per-matter allocation, prompt template
library with versioning/rollback, structured extraction (serde-validated JSON), RAG over
matter-scoped knowledge base (Tantivy + optional Qdrant vectors), citation/source tracking,
summarization/comparison/clause-risk drafting aids. **AI never certifies legal validity**;
every AI draft/flag requires human approval with disclaimer banner; blocked-topic policy
refuses validity-certification requests. Safety: prompt-injection defenses (instruction hierarchy,
untrusted-content delimiting, tool-output validation), PII redaction pre-call, tenant-scoped
retrieval, data-retention limits, full audit (prompt hash/response/model/version/cost).
Providers: OpenAI + Anthropic + local models behind same trait; keys in vault; eval harness
(grounded-QA accuracy, citation precision, refusal correctness, regression on prompt change).
