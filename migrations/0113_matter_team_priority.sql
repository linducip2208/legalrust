-- 0113 matter team + priority. Forward-only.
ALTER TABLE matters ADD COLUMN priority VARCHAR(20) NOT NULL DEFAULT 'normal';
CREATE TABLE matter_team (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  user_id VARCHAR(160) NOT NULL DEFAULT '',
  role VARCHAR(40) NOT NULL DEFAULT 'member',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mtm_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mtm_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  CONSTRAINT chk_mtm_role CHECK (role IN ('lead', 'member', 'viewer')),
  UNIQUE KEY uq_mtm_pair (org_id, matter_id, user_id),
  KEY ix_mtm_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
