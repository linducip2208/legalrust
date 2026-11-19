-- 0081 approval policy snapshots (routing versioning). Forward-only.
CREATE TABLE approval_policies (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  version INT NOT NULL DEFAULT 1,
  payload MEDIUMTEXT NOT NULL,
  created_by VARCHAR(160) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_apol_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_apol_org_ver (org_id, version),
  KEY ix_apol_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
