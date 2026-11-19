-- 0075 retention rules + approval-gated purge requests. Forward-only.
CREATE TABLE retention_rules (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  entity VARCHAR(40) NOT NULL DEFAULT '',
  retain_years INT NOT NULL DEFAULT 5,
  action VARCHAR(20) NOT NULL DEFAULT 'archive',
  active TINYINT(1) NOT NULL DEFAULT 1,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_rrule_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_rrule_org_entity (org_id, entity),
  KEY ix_rrule_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE purge_requests (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  entity VARCHAR(40) NOT NULL DEFAULT '',
  entity_id BIGINT UNSIGNED NOT NULL DEFAULT 0,
  reason VARCHAR(500) NOT NULL DEFAULT '',
  requested_by VARCHAR(160) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'requested',
  decided_by VARCHAR(160) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_preq_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_preq_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
