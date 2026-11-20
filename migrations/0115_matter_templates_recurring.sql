-- 0115 matter templates + recurring scheduler. Forward-only.
CREATE TABLE matter_templates (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  name VARCHAR(160) NOT NULL DEFAULT '',
  description VARCHAR(500) NOT NULL DEFAULT '',
  tasks_json TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mtpl_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_mtpl_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE matter_recurrences (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  template_id BIGINT UNSIGNED NULL,
  title VARCHAR(300) NOT NULL DEFAULT '',
  cron VARCHAR(60) NOT NULL DEFAULT 'monthly',
  active TINYINT(1) NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mrcr_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mrcr_tpl FOREIGN KEY (template_id) REFERENCES matter_templates (id) ON DELETE SET NULL,
  KEY ix_mrcr_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
