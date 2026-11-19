-- 0107 org feature flags (maintenance mode first). Forward-only.
CREATE TABLE org_flags (
  org_id CHAR(36) NOT NULL,
  flag VARCHAR(40) NOT NULL DEFAULT '',
  enabled TINYINT(1) NOT NULL DEFAULT 0,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (org_id, flag),
  CONSTRAINT fk_oflag_org FOREIGN KEY (org_id) REFERENCES organizations (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
