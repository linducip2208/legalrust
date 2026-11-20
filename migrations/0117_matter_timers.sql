-- 0117 matter time-capture timers. Forward-only.
CREATE TABLE matter_timers (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  user_id VARCHAR(160) NOT NULL DEFAULT '',
  note VARCHAR(200) NOT NULL DEFAULT '',
  started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  active TINYINT(1) NOT NULL DEFAULT 1,
  stopped_minutes INT NULL,
  CONSTRAINT fk_tmr_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_tmr_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_tmr_user (org_id, user_id, active)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
