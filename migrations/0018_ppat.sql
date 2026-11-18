-- 0018 PPAT deed register. Forward-only.
CREATE TABLE ppat_entries (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  number VARCHAR(60) NOT NULL,
  deed_kind VARCHAR(60) NOT NULL DEFAULT 'ajb',
  party_seller VARCHAR(200) NOT NULL DEFAULT '',
  party_buyer VARCHAR(200) NOT NULL DEFAULT '',
  object_desc VARCHAR(255) NOT NULL DEFAULT '',
  status VARCHAR(40) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ppat_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_ppat_org_number (org_id, number),
  KEY ix_ppat_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
