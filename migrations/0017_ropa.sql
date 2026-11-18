-- 0017 UU PDP ROPA register (records of processing activities). Forward-only.
CREATE TABLE ropa_entries (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  activity VARCHAR(200) NOT NULL,
  purpose VARCHAR(200) NOT NULL,
  data_categories VARCHAR(200) NOT NULL DEFAULT '',
  legal_basis VARCHAR(120) NOT NULL DEFAULT 'consent',
  retention VARCHAR(120) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ropa_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_ropa_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
