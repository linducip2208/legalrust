-- 0119 office branch registry + hierarchy links. Forward-only.
CREATE TABLE offices (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  code VARCHAR(40) NOT NULL DEFAULT '',
  name VARCHAR(160) NOT NULL DEFAULT '',
  city VARCHAR(120) NOT NULL DEFAULT '',
  active TINYINT(1) NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ofc_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_ofc_code (org_id, code),
  KEY ix_ofc_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE office_links (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  child_id BIGINT UNSIGNED NOT NULL,
  parent_id BIGINT UNSIGNED NOT NULL,
  relation VARCHAR(40) NOT NULL DEFAULT 'branch',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ofl_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ofl_child FOREIGN KEY (child_id) REFERENCES offices (id) ON DELETE CASCADE,
  CONSTRAINT fk_ofl_parent FOREIGN KEY (parent_id) REFERENCES offices (id) ON DELETE CASCADE,
  UNIQUE KEY uq_ofl_pair (org_id, child_id, parent_id),
  KEY ix_ofl_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
