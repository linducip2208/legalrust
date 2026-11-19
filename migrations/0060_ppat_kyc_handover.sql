-- 0060 PPAT party KYC bundle + certificate handover receipts. Forward-only.
CREATE TABLE ppat_parties (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  role VARCHAR(20) NOT NULL DEFAULT 'seller',
  name VARCHAR(160) NOT NULL,
  id_number VARCHAR(40) NOT NULL DEFAULT '',
  verified TINYINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_pparty_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_pparty_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  KEY ix_pparty_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE cert_handovers (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  handed_to VARCHAR(160) NOT NULL,
  handed_at VARCHAR(10) NOT NULL DEFAULT '',
  receipt_no VARCHAR(60) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'prepared',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_chand_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_chand_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  KEY ix_chand_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
