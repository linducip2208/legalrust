-- 0103 client deed-copy requests. Forward-only.
CREATE TABLE client_copy_requests (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  deed_id BIGINT UNSIGNED NULL,
  note VARCHAR(500) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'requested',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ccpr_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ccpr_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  KEY ix_ccpr_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
