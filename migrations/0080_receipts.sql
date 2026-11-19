-- 0080 approval decision receipt chain (integrity hash). Forward-only.
CREATE TABLE approval_receipts (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  req_id BIGINT UNSIGNED NOT NULL,
  step_no INT NOT NULL DEFAULT 0,
  decision VARCHAR(20) NOT NULL DEFAULT '',
  actor VARCHAR(160) NOT NULL DEFAULT '',
  prev_hash CHAR(64) NOT NULL DEFAULT '',
  hash CHAR(64) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_arec_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_arec_req (org_id, req_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
