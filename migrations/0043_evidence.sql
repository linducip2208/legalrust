-- 0043 compliance evidence vault (links registers to evidence notes). Forward-only.
CREATE TABLE compliance_evidence (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  register_name VARCHAR(40) NOT NULL,
  register_id BIGINT UNSIGNED NOT NULL,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cev_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_cev_org (org_id),
  KEY ix_cev_register (register_name, register_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
