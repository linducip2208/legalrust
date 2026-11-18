-- 0016 outbound webhook endpoints. Forward-only.
CREATE TABLE webhook_endpoints (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  url VARCHAR(500) NOT NULL,
  secret VARCHAR(255) NOT NULL DEFAULT '',
  events VARCHAR(500) NOT NULL DEFAULT '*',
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_whep_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_whep_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
ALTER TABLE webhook_deliveries
  ADD COLUMN direction VARCHAR(8) NOT NULL DEFAULT 'in',
  ADD KEY ix_wh_dir (direction, created_at);
