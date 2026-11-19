-- 0093 VIP flags + credit limits. Forward-only.
CREATE TABLE client_vips (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  tier VARCHAR(20) NOT NULL DEFAULT 'vip',
  note VARCHAR(200) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cvip_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cvip_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_cvip_tier CHECK (tier IN ('vip', 'priority', 'watch')),
  UNIQUE KEY uq_cvip_triple (org_id, client_id, tier),
  KEY ix_cvip_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE client_credit (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  limit_minor BIGINT NOT NULL DEFAULT 0,
  note VARCHAR(200) NOT NULL DEFAULT '',
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_ccred_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ccred_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  UNIQUE KEY uq_ccred_pair (org_id, client_id),
  KEY ix_ccred_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
