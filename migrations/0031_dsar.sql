-- 0031 data-subject rights requests (akses/penghapusan/perbaikan, UU PDP). Forward-only.
CREATE TABLE dsar_requests (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  subject VARCHAR(160) NOT NULL,
  kind VARCHAR(20) NOT NULL DEFAULT 'access',
  detail TEXT NOT NULL,
  due_date VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dsar_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_dsar_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
