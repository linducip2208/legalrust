-- 0015 UU PDP compliance slice: obligation register + consent lifecycle.
-- Forward-only. Tenant-scoped; no PII beyond a subject label string.
CREATE TABLE pdp_obligations (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  category VARCHAR(60) NOT NULL DEFAULT 'general',
  status VARCHAR(24) NOT NULL DEFAULT 'open',
  owner VARCHAR(120) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_pdp_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_pdp_org_status (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE consent_records (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  subject VARCHAR(120) NOT NULL,
  purpose VARCHAR(200) NOT NULL,
  status VARCHAR(24) NOT NULL DEFAULT 'granted',
  granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  withdrawn_at TIMESTAMP NULL,
  CONSTRAINT fk_consent_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_consent_org_subject (org_id, subject)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
