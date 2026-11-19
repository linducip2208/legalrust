-- 0111 time entries + timesheet approval. Forward-only.
CREATE TABLE time_entries (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  user_id VARCHAR(160) NOT NULL DEFAULT '',
  entry_date VARCHAR(10) NOT NULL DEFAULT '',
  minutes INT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ten_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ten_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  CONSTRAINT chk_ten_status CHECK (status IN ('draft', 'submitted', 'approved', 'rejected')),
  KEY ix_ten_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
