-- 0086 client onboarding/offboarding checklists + items. Forward-only.
CREATE TABLE client_checklists (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  kind VARCHAR(20) NOT NULL DEFAULT 'onboarding',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cchk_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cchk_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_cchk_kind CHECK (kind IN ('onboarding', 'offboarding')),
  KEY ix_cchk_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE client_check_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  checklist_id BIGINT UNSIGNED NOT NULL,
  label VARCHAR(200) NOT NULL DEFAULT '',
  done TINYINT(1) NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cchit_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cchit_list FOREIGN KEY (checklist_id) REFERENCES client_checklists (id) ON DELETE CASCADE,
  KEY ix_cchit_list (checklist_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
