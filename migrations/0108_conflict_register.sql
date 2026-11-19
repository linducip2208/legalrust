-- 0108 conflict-of-interest register. Forward-only.
CREATE TABLE conflict_checks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  needle VARCHAR(160) NOT NULL DEFAULT '',
  clients_hits INT NOT NULL DEFAULT 0,
  matters_hits INT NOT NULL DEFAULT 0,
  parties_hits INT NOT NULL DEFAULT 0,
  verdict VARCHAR(20) NOT NULL DEFAULT 'clear',
  note VARCHAR(500) NOT NULL DEFAULT '',
  checked_by VARCHAR(160) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cck_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT chk_cck_verdict CHECK (verdict IN ('clear', 'flagged', 'waived')),
  KEY ix_cck_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
