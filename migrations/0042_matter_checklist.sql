-- 0042 matter-level compliance checklist. Forward-only.
CREATE TABLE matter_checklist_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  item VARCHAR(200) NOT NULL,
  done TINYINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mcl_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mcl_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_mcl_matter (matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
