-- 0067 land history (riwayat tanah) + unit handover (BAST) trackers. Forward-only.
CREATE TABLE land_history (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  parcel_ref VARCHAR(120) NOT NULL,
  event VARCHAR(40) NOT NULL DEFAULT 'alas_hak',
  detail TEXT NOT NULL,
  event_date VARCHAR(10) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_lhist_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_lhist_org (org_id),
  KEY ix_lhist_parcel (parcel_ref)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE unit_handovers (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  unit_ref VARCHAR(120) NOT NULL,
  handed_from VARCHAR(160) NOT NULL DEFAULT '',
  handed_to VARCHAR(160) NOT NULL DEFAULT '',
  bast_no VARCHAR(60) NOT NULL DEFAULT '',
  handed_on VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_uho_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_uho_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
