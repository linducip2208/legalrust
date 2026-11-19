-- 0039 control library (UUJN/PDP/ITE). Forward-only.
CREATE TABLE controls (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  code VARCHAR(40) NOT NULL,
  title VARCHAR(200) NOT NULL,
  area VARCHAR(60) NOT NULL DEFAULT 'general',
  frequency VARCHAR(20) NOT NULL DEFAULT 'quarterly',
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_control_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_control_org_code (org_id, code),
  KEY ix_control_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
