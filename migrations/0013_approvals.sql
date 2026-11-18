-- 0013 approvals engine (generic multi-step sequential approvals).
-- Forward-only. Tenant-scoped; steps ordered; history via steps + audit trail.
CREATE TABLE approval_requests (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  entity VARCHAR(24) NOT NULL,
  entity_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(200) NOT NULL,
  status VARCHAR(24) NOT NULL DEFAULT 'pending',
  created_by CHAR(36) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_appr_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_appr_org_status (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE approval_steps (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  request_id BIGINT UNSIGNED NOT NULL,
  step_no INT NOT NULL,
  approver_role VARCHAR(60) NOT NULL,
  approver_user CHAR(36) NULL,
  status VARCHAR(24) NOT NULL DEFAULT 'pending',
  comment TEXT NULL,
  decided_by CHAR(36) NULL,
  decided_at TIMESTAMP NULL,
  CONSTRAINT fk_apprstep_req FOREIGN KEY (request_id) REFERENCES approval_requests (id) ON DELETE CASCADE,
  UNIQUE KEY uq_apprstep_req_no (request_id, step_no)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
