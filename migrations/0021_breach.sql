-- 0021 breach log (UU PDP 72h notification). Forward-only.
CREATE TABLE breach_logs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  discovered_at VARCHAR(20) NOT NULL DEFAULT '',
  affected_count BIGINT NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_breach_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_breach_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
