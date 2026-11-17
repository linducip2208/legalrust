# DAILY LOG
## 2026-11-17 (2)
- Phase 4: UserStore/CoreStore traits + memory + MySQL impls; auth service (register/login/logout/sessions/JWT-sid); RBAC extractor + authorize(); login rate-limit noted as backlog.
- Phase 6 slice: clients/matters/tasks/documents/content endpoints; sha-pinned uploads; task transition guard; audit on register/matter.
- Migration 0004 (matter.client_id nullable). Full-flow API test green on memory store; live-MySQL ignored test + CI job added.
