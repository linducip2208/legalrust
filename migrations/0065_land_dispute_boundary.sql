-- 0065 land-dispute flags + boundary verification. Forward-only.
CREATE TABLE land_disputes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  parcel_ref VARCHAR(120) NOT NULL,
  kind VARCHAR(40) NOT NULL DEFAULT 'sengketa',
  description TEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'flagged',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ldisp_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_ldisp_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE boundary_checks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  parcel_ref VARCHAR(120) NOT NULL,
  declared_area BIGINT NOT NULL DEFAULT 0,
  measured_area BIGINT NOT NULL DEFAULT 0,
  result VARCHAR(20) NOT NULL DEFAULT 'pending',
  checked_on VARCHAR(10) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_bcheck2_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_bcheck2_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
