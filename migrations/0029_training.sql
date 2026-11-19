-- 0029 compliance training assignments. Forward-only.
CREATE TABLE training_assignments (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  assignee VARCHAR(120) NOT NULL DEFAULT '',
  due_date VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'assigned',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_training_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_training_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
