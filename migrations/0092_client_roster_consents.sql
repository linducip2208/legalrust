-- 0092 corporate employee roster + client consent center. Forward-only.
CREATE TABLE client_employees (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  name VARCHAR(160) NOT NULL DEFAULT '',
  role VARCHAR(120) NOT NULL DEFAULT '',
  contact VARCHAR(160) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cemp_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cemp_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  KEY ix_cemp_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE client_consents (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  channel VARCHAR(20) NOT NULL DEFAULT 'email',
  allowed TINYINT(1) NOT NULL DEFAULT 1,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_ccns_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ccns_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_ccns_channel CHECK (channel IN ('email', 'phone', 'visit', 'newsletter')),
  UNIQUE KEY uq_ccns_triple (org_id, client_id, channel),
  KEY ix_ccns_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
