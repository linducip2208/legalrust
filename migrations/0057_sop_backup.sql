-- 0057 SOP library + backup verification log. Forward-only.
CREATE TABLE sop_docs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  code VARCHAR(40) NOT NULL,
  title VARCHAR(200) NOT NULL,
  version VARCHAR(20) NOT NULL DEFAULT '1.0',
  body MEDIUMTEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_sop_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_sop_org_code (org_id, code),
  KEY ix_sop_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE backup_checks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  checked_at VARCHAR(16) NOT NULL DEFAULT '',
  method VARCHAR(60) NOT NULL DEFAULT 'dump',
  result VARCHAR(20) NOT NULL DEFAULT 'pass',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_bcheck_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_bcheck_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
