-- 0102 client case messages. Forward-only.
CREATE TABLE client_messages (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  matter_id BIGINT UNSIGNED NULL,
  sender VARCHAR(20) NOT NULL DEFAULT 'client',
  body TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cmsg_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cmsg_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_cmsg_sender CHECK (sender IN ('client', 'staff')),
  KEY ix_cmsg_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
