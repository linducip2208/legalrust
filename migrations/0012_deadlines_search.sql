-- 0012 task deadlines (DATETIME) + saved searches (forward-only)
-- NOTE: MySQL has no `ADD COLUMN IF NOT EXISTS`. `tasks.due_at` already exists
-- as TIMESTAMP NULL since 0002, so a plain `ADD COLUMN due_at` would fail with
-- ERROR 1060 (duplicate column) on both fresh and existing databases. This
-- migration widens it to DATETIME NULL with MODIFY instead (idempotent,
-- forward-only). DATETIME is preferred for deadlines (wider range, no TZ
-- conversion on read/write).
ALTER TABLE tasks MODIFY COLUMN due_at DATETIME NULL;
CREATE TABLE IF NOT EXISTS saved_searches (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  user_id CHAR(36) NOT NULL,
  name VARCHAR(128) NOT NULL,
  `query` VARCHAR(512) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_saved_search_org_user (org_id, user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
