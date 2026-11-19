-- 0072 AJB draft store. Forward-only.
CREATE TABLE ajb_drafts (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  template_name VARCHAR(120) NOT NULL DEFAULT '',
  body MEDIUMTEXT NOT NULL,
  price_minor BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ajbd_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ajbd_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  KEY ix_ajbd_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
