-- 0062 PBB status verification log (manual; external check BLOCKED). Forward-only.
CREATE TABLE pbb_checks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  nop VARCHAR(40) NOT NULL,
  tax_year CHAR(4) NOT NULL DEFAULT '',
  result VARCHAR(20) NOT NULL DEFAULT 'unknown',
  checked_on VARCHAR(10) NOT NULL DEFAULT '',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_pbb_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_pbb_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
