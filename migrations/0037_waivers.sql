-- 0037 exception / waiver workflow. Forward-only.
CREATE TABLE exception_waivers (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  justification TEXT NOT NULL,
  requested_by VARCHAR(120) NOT NULL DEFAULT '',
  expires_at VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'requested',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_waiver_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_waiver_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
