-- 0100 client risk profiles. Forward-only.
CREATE TABLE client_risk_profiles (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  level VARCHAR(20) NOT NULL DEFAULT 'low',
  note VARCHAR(500) NOT NULL DEFAULT '',
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_crisk_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_crisk_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_crisk_level CHECK (level IN ('low', 'medium', 'high', 'critical')),
  UNIQUE KEY uq_crisk_pair (org_id, client_id),
  KEY ix_crisk_level (org_id, level)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
