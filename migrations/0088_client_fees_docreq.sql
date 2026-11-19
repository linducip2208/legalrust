-- 0088 client fee arrangements + document request tracker. Forward-only.
CREATE TABLE client_fees (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  name VARCHAR(120) NOT NULL DEFAULT '',
  kind VARCHAR(20) NOT NULL DEFAULT 'hourly',
  amount_minor BIGINT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cfee_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cfee_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_cfee_kind CHECK (kind IN ('hourly', 'fixed', 'retainer', 'success', 'pro_bono')),
  KEY ix_cfee_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE client_doc_requests (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(200) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'requested',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cdreq_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cdreq_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  KEY ix_cdreq_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
