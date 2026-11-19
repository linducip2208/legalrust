-- 0077 idempotency keys for approval mutations. Forward-only.
CREATE TABLE idempotency_keys (
  org_id CHAR(36) NOT NULL,
  idem_key VARCHAR(64) NOT NULL,
  endpoint VARCHAR(80) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (org_id, idem_key),
  CONSTRAINT fk_idem_org FOREIGN KEY (org_id) REFERENCES organizations (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
