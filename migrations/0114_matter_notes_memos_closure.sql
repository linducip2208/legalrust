-- 0114 matter notes + internal memos + closure checklist. Forward-only.
CREATE TABLE matter_notes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  author VARCHAR(160) NOT NULL DEFAULT '',
  body VARCHAR(2000) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mnt_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mnt_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_mnt_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE matter_memos (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  author VARCHAR(160) NOT NULL DEFAULT '',
  body VARCHAR(2000) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mmm_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mmm_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_mmm_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE matter_closure_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(300) NOT NULL DEFAULT '',
  done TINYINT(1) NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mclo_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mclo_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_mcl_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
