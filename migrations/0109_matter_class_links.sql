-- 0109 matter practice-area classification + related-matter links. Forward-only.
ALTER TABLE matters ADD COLUMN practice_area VARCHAR(20) NOT NULL DEFAULT 'general';
CREATE TABLE matter_links (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_a BIGINT UNSIGNED NOT NULL,
  matter_b BIGINT UNSIGNED NOT NULL,
  relation VARCHAR(40) NOT NULL DEFAULT 'related',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mlink_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mlink_a FOREIGN KEY (matter_a) REFERENCES matters (id) ON DELETE CASCADE,
  CONSTRAINT fk_mlink_b FOREIGN KEY (matter_b) REFERENCES matters (id) ON DELETE CASCADE,
  CONSTRAINT chk_mlink_rel CHECK (relation IN ('related', 'parent', 'duplicate', 'split_from')),
  UNIQUE KEY uq_mlink_pair (org_id, matter_a, matter_b),
  KEY ix_mlink_a (matter_a)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
