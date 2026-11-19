-- 0036 whistleblowing intake channel. Forward-only.
CREATE TABLE whistle_reports (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  channel VARCHAR(20) NOT NULL DEFAULT 'web',
  category VARCHAR(60) NOT NULL DEFAULT 'general',
  summary TEXT NOT NULL,
  contact VARCHAR(160) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'received',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_whistle_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_whistle_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
