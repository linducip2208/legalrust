-- 0091 client data correction requests. Forward-only.
CREATE TABLE client_corrections (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  field VARCHAR(20) NOT NULL DEFAULT 'display_name',
  old_value VARCHAR(255) NOT NULL DEFAULT '',
  new_value VARCHAR(255) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'requested',
  requested_by VARCHAR(160) NOT NULL DEFAULT '',
  decided_by VARCHAR(160) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ccor_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ccor_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_ccor_field CHECK (field IN ('display_name', 'kind', 'status')),
  KEY ix_ccor_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
