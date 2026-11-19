-- 0049 visitor log + office announcement board. Forward-only.
-- (Number reused after superseded 0049_inventory_fix was withdrawn pre-push.)
CREATE TABLE visitors (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  name VARCHAR(160) NOT NULL,
  need VARCHAR(255) NOT NULL DEFAULT '',
  check_in VARCHAR(16) NOT NULL DEFAULT '',
  check_out VARCHAR(16) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'in',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_visitor_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_visitor_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE announcements (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  body TEXT NOT NULL,
  pinned TINYINT NOT NULL DEFAULT 0,
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_announce_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_announce_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
