-- 0032 ethics & independence declarations. Forward-only.
CREATE TABLE ethics_declarations (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  declarant VARCHAR(120) NOT NULL,
  year CHAR(4) NOT NULL DEFAULT '',
  statement TEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'submitted',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ethics_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_ethics_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
