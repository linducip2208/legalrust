-- 0076 approval digest prefs + audit freeze flag. Forward-only.
CREATE TABLE approval_prefs (
  user_id VARCHAR(160) NOT NULL PRIMARY KEY,
  digest_mode VARCHAR(10) NOT NULL DEFAULT 'instant',
  escalate_to VARCHAR(160) NOT NULL DEFAULT '',
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT chk_apref_digest CHECK (digest_mode IN ('instant', 'daily', 'off'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE approval_freeze (
  org_id CHAR(36) NOT NULL PRIMARY KEY,
  frozen TINYINT(1) NOT NULL DEFAULT 0,
  reason VARCHAR(500) NOT NULL DEFAULT '',
  set_by VARCHAR(160) NOT NULL DEFAULT '',
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_afreeze_org FOREIGN KEY (org_id) REFERENCES organizations (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
