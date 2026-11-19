-- 0078 approval routing rules (amount + risk). Forward-only.
CREATE TABLE approval_routes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  kind VARCHAR(10) NOT NULL DEFAULT 'amount',
  entity VARCHAR(20) NOT NULL DEFAULT 'invoice',
  min_value BIGINT NOT NULL DEFAULT 0,
  role VARCHAR(60) NOT NULL DEFAULT '',
  priority INT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_aroute_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT chk_aroute_kind CHECK (kind IN ('amount', 'risk')),
  KEY ix_aroute_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
