-- 0030 enterprise risk register. Forward-only.
CREATE TABLE risk_register (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  category VARCHAR(60) NOT NULL DEFAULT 'operational',
  likelihood TINYINT UNSIGNED NOT NULL DEFAULT 1,
  impact TINYINT UNSIGNED NOT NULL DEFAULT 1,
  owner VARCHAR(120) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_risk_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_risk_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
