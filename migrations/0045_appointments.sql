-- 0045 penghadap appointment scheduler. Forward-only.
CREATE TABLE appointments (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_name VARCHAR(160) NOT NULL,
  purpose VARCHAR(255) NOT NULL DEFAULT '',
  scheduled_at VARCHAR(16) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'scheduled',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_appt_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_appt_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
