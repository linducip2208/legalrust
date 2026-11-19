-- 0052 office working-hours, roster & leave/substitute protocol. Forward-only.
CREATE TABLE office_hours (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  weekday TINYINT UNSIGNED NOT NULL,
  open_time VARCHAR(5) NOT NULL DEFAULT '',
  close_time VARCHAR(5) NOT NULL DEFAULT '',
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_hours_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_hours_org_day (org_id, weekday),
  KEY ix_hours_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE roster_members (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  name VARCHAR(160) NOT NULL,
  role VARCHAR(60) NOT NULL DEFAULT 'staff',
  shift VARCHAR(20) NOT NULL DEFAULT 'pagi',
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_roster_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_roster_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE leaves (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  holder VARCHAR(120) NOT NULL,
  kind VARCHAR(20) NOT NULL DEFAULT 'leave',
  from_date VARCHAR(10) NOT NULL DEFAULT '',
  to_date VARCHAR(10) NOT NULL DEFAULT '',
  substitute VARCHAR(120) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'requested',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_leave_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_leave_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
