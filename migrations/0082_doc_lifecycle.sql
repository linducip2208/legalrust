-- 0082 document lifecycle flags (hold / archive / trash + retention date). Forward-only.
CREATE TABLE doc_lifecycle (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  doc_id BIGINT UNSIGNED NOT NULL,
  on_hold TINYINT(1) NOT NULL DEFAULT 0,
  archived TINYINT(1) NOT NULL DEFAULT 0,
  trashed TINYINT(1) NOT NULL DEFAULT 0,
  retain_until VARCHAR(10) NOT NULL DEFAULT '',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dlc_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_dlc_doc FOREIGN KEY (doc_id) REFERENCES documents (id) ON DELETE CASCADE,
  UNIQUE KEY uq_dlc_org_doc (org_id, doc_id),
  KEY ix_dlc_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
