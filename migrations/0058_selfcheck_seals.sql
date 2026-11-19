-- 0058 compliance self-check runs + protocol year-end sealing. Forward-only.
CREATE TABLE selfcheck_runs (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  period VARCHAR(20) NOT NULL,
  passed_items INT NOT NULL DEFAULT 0,
  total_items INT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_scheck_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_scheck_org_period (org_id, period),
  KEY ix_scheck_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE year_end_seals (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  year CHAR(4) NOT NULL,
  book_count INT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'prepared',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_seal_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_seal_org_year (org_id, year),
  KEY ix_seal_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
