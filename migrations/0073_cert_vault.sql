-- 0073 certificate scan vault links (deed <-> document). Forward-only.
CREATE TABLE cert_links (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  doc_id BIGINT UNSIGNED NOT NULL,
  doc_name VARCHAR(255) NOT NULL DEFAULT '',
  label VARCHAR(120) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_clink_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_clink_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  CONSTRAINT fk_clink_doc FOREIGN KEY (doc_id) REFERENCES documents (id) ON DELETE CASCADE,
  UNIQUE KEY uq_clink_ppat_doc (ppat_id, doc_id),
  KEY ix_clink_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
