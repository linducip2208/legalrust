-- 0027 corrective-action (CAPA) tracker. Forward-only.
CREATE TABLE capa_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  cause VARCHAR(255) NOT NULL DEFAULT '',
  action VARCHAR(255) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_capa_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_capa_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
