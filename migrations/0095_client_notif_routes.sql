-- 0095 client notification routing recipients. Forward-only.
CREATE TABLE client_notif_routes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  name VARCHAR(160) NOT NULL DEFAULT '',
  channel VARCHAR(20) NOT NULL DEFAULT 'email',
  target VARCHAR(255) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cnr_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cnr_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_cnr_channel CHECK (channel IN ('email', 'phone', 'visit')),
  UNIQUE KEY uq_cnr_triple (org_id, client_id, channel, target(64)),
  KEY ix_cnr_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
