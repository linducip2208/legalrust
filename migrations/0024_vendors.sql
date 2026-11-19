-- 0024 vendor/processor register with DPA status. Forward-only.
CREATE TABLE vendor_register (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  name VARCHAR(200) NOT NULL,
  service VARCHAR(255) NOT NULL DEFAULT '',
  dpa_status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_vendor_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_vendor_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
