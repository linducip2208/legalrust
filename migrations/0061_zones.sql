-- 0061 zone (ZNT/NJOP) reference table. Forward-only.
CREATE TABLE zones (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  area_code VARCHAR(40) NOT NULL,
  area_name VARCHAR(200) NOT NULL,
  njop_minor BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_zone_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_zone_org_code (org_id, area_code),
  KEY ix_zone_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
