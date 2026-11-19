-- 0068 notary-to-PPAT handoff checklist. Forward-only.
CREATE TABLE notary_handoffs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  doc_name VARCHAR(160) NOT NULL DEFAULT '',
  received TINYINT(1) NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_nhoff_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_nhoff_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  KEY ix_nhoff_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
