-- 0083 document metadata schema + values (tags included). Forward-only.
CREATE TABLE meta_fields (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  field_key VARCHAR(60) NOT NULL DEFAULT '',
  kind VARCHAR(10) NOT NULL DEFAULT 'text',
  required TINYINT(1) NOT NULL DEFAULT 0,
  note VARCHAR(200) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mfield_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT chk_mfield_kind CHECK (kind IN ('text', 'number', 'date', 'tag')),
  UNIQUE KEY uq_mfield_org_key (org_id, field_key),
  KEY ix_mfield_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE doc_meta (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  doc_id BIGINT UNSIGNED NOT NULL,
  field_key VARCHAR(60) NOT NULL DEFAULT '',
  field_value TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dmeta_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_dmeta_doc FOREIGN KEY (doc_id) REFERENCES documents (id) ON DELETE CASCADE,
  KEY ix_dmeta_doc (org_id, doc_id),
  KEY ix_dmeta_tag (org_id, field_key, field_value(64))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
