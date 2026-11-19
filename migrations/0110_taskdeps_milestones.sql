-- 0110 task dependencies + matter milestones. Forward-only.
CREATE TABLE task_deps (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  task_id BIGINT UNSIGNED NOT NULL,
  depends_on BIGINT UNSIGNED NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_tdep_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_tdep_task FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE,
  CONSTRAINT fk_tdep_on FOREIGN KEY (depends_on) REFERENCES tasks (id) ON DELETE CASCADE,
  UNIQUE KEY uq_tdep_pair (org_id, task_id, depends_on),
  KEY ix_tdep_task (task_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE matter_milestones (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(200) NOT NULL DEFAULT '',
  due_on VARCHAR(10) NOT NULL DEFAULT '',
  done TINYINT(1) NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mms_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mms_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_mms_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
