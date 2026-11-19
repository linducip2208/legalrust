-- 0055 office document checklist + office incident log. Forward-only.
CREATE TABLE doc_checklist_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  doc_id BIGINT UNSIGNED NOT NULL,
  item VARCHAR(200) NOT NULL,
  done TINYINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dcl_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_dcl_doc FOREIGN KEY (doc_id) REFERENCES documents (id) ON DELETE CASCADE,
  KEY ix_dcl_doc (doc_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE office_incidents (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  severity VARCHAR(20) NOT NULL DEFAULT 'medium',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_oinc_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_oinc_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
