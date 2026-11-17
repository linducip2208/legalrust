# MYSQL AUDIT — verdict: SOUND with fix applied (80/100)
Checked example DDL + dictionary: InnoDB/utf8mb4, BIGINT PKs, explicit FKs (orphan found & fixed: audit_events FK),
unique tenant-scoped numbers, composite (tenant,office,status) indexes, append-only audit, no polymorphic joins,
tenant predicate rule, row_version locking, soft-delete except logs, retention-vs-hold matrix, SQLx offline CI,
forward-only migrations. Deductions: physical ERD beyond core 12 tables deferred (-8), partition sizing unmeasured (-6),
managed-RDS 9.7 preview noted (-6). No cross-tenant leakage path in design; RLS-equivalent enforced in service layer + review.
