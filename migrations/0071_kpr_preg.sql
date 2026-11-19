-- 0071 KPR coordination packs + protocol register. Forward-only.
CREATE TABLE kpr_packs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  bank VARCHAR(120) NOT NULL DEFAULT '',
  amount_minor BIGINT NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL DEFAULT 'applied',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_kpr_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_kpr_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  KEY ix_kpr_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE protocol_register (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  entry_date VARCHAR(10) NOT NULL DEFAULT '',
  book_ref VARCHAR(120) NOT NULL DEFAULT '',
  deed_no VARCHAR(120) NOT NULL DEFAULT '',
  party VARCHAR(160) NOT NULL DEFAULT '',
  direction VARCHAR(10) NOT NULL DEFAULT 'in',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_preg_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_preg_org (org_id),
  KEY ix_preg_deed (deed_no)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
