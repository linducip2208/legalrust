-- 0084 per-org storage quota. Forward-only.
CREATE TABLE org_quotas (
  org_id CHAR(36) NOT NULL PRIMARY KEY,
  quota_bytes BIGINT NOT NULL DEFAULT 0,
  note VARCHAR(200) NOT NULL DEFAULT '',
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_oquota_org FOREIGN KEY (org_id) REFERENCES organizations (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
