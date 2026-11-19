-- 0028 regulatory change tracker (UU PDP / peraturan sektoral). Forward-only.
CREATE TABLE regulatory_changes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  source VARCHAR(120) NOT NULL DEFAULT '',
  effective_date VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_regchange_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_regchange_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
