# Changelog
## [Unreleased]
- Phases 7-12: OCR job/review API, deed templates + rendered drafts + transition guards, AI chat-501 with token accounting, RAG keyword index/search, invoices/payments with overpayment guard, HMAC webhooks, auth rate-limit, security headers.
- Migrations 0005-0007. OpsStore (memory + MySQL). 31 tests green, clippy/fmt clean.
- Deploy: k8s.yaml, scripts/backup.sh. Acceptance self-audit: 21/31 = 68/100, not production-ready.
## [0.1.0] foundation (earlier commit)
- Workspace scaffold, auth core, RBAC map, storage/OCR/AI/RAG/workflow modules, migrations 0001-0004, CI, 26 tests green.
