-- 0066 deed consent checks (marital + heir). Forward-only.
-- kind is VARCHAR so board/power consents can reuse this table later
-- without a new migration.
CREATE TABLE deed_consents (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  kind VARCHAR(20) NOT NULL DEFAULT 'marital',
  party VARCHAR(160) NOT NULL DEFAULT '',
  verified TINYINT(1) NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dconsent_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_dconsent_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  KEY ix_dconsent_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
