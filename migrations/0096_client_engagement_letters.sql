-- 0096 client engagement letters. Forward-only.
CREATE TABLE client_engagement_letters (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(200) NOT NULL DEFAULT '',
  body MEDIUMTEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cel_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cel_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  KEY ix_cel_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
