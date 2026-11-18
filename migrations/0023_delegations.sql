-- 0023 approval delegations (out-of-office / substitute). Forward-only.
CREATE TABLE approval_delegations (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  delegator_id VARCHAR(255) NOT NULL,
  delegate_id VARCHAR(255) NOT NULL,
  active TINYINT(1) NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_deleg_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_deleg_pair (org_id, delegator_id, delegate_id),
  KEY ix_deleg_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
