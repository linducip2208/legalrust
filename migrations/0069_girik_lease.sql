-- 0069 girik/adat intake + lease terms. Forward-only.
CREATE TABLE girik_intakes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  letter_c_ref VARCHAR(120) NOT NULL,
  holder VARCHAR(160) NOT NULL DEFAULT '',
  area_minor BIGINT NOT NULL DEFAULT 0,
  village VARCHAR(120) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_girik_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_girik_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE lease_terms (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  monthly_minor BIGINT NOT NULL DEFAULT 0,
  duration_months INT NOT NULL DEFAULT 0,
  start_date VARCHAR(10) NOT NULL DEFAULT '',
  deposit_minor BIGINT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_lease_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_lease_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  UNIQUE KEY uq_lease_ppat (ppat_id),
  KEY ix_lease_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
