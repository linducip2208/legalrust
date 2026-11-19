-- 0064 land split (pemecahan) + land merge (penggabungan) trackers. Forward-only.
CREATE TABLE land_splits (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  source_parcel VARCHAR(120) NOT NULL,
  new_parcels TEXT NOT NULL,
  split_date VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'planned',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_lsplit_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_lsplit_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE land_merges (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  source_parcels TEXT NOT NULL,
  result_parcel VARCHAR(120) NOT NULL,
  merge_date VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'planned',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_lmerge_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_lmerge_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
