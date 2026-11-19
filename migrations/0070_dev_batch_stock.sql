-- 0070 developer bulk batches + unit stock registry. Forward-only.
CREATE TABLE dev_batches (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  developer VARCHAR(160) NOT NULL DEFAULT '',
  batch_no VARCHAR(60) NOT NULL DEFAULT '',
  total_units INT NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL DEFAULT 'planned',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dbatch_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_dbatch_org_no (org_id, batch_no),
  KEY ix_dbatch_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE dev_units (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  batch_id BIGINT UNSIGNED NULL,
  unit_code VARCHAR(60) NOT NULL DEFAULT '',
  unit_type VARCHAR(40) NOT NULL DEFAULT 'rumah',
  price_minor BIGINT NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL DEFAULT 'available',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dunit_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_dunit_batch FOREIGN KEY (batch_id) REFERENCES dev_batches (id) ON DELETE SET NULL,
  UNIQUE KEY uq_dunit_org_code (org_id, unit_code),
  KEY ix_dunit_batch (batch_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
