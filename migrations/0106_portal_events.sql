-- 0106 portal usage events. Forward-only.
CREATE TABLE portal_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  kind VARCHAR(20) NOT NULL DEFAULT 'view',
  ref_name VARCHAR(120) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_pev_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_pev_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_pev_kind CHECK (kind IN ('login', 'view', 'download', 'upload', 'message', 'rating', 'request')),
  KEY ix_pev_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
