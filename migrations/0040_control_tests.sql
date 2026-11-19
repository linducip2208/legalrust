-- 0040 control test log (evidence of control operation). Forward-only.
CREATE TABLE control_tests (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  control_id BIGINT UNSIGNED NOT NULL,
  tested_at VARCHAR(10) NOT NULL DEFAULT '',
  result VARCHAR(20) NOT NULL DEFAULT 'pass',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ctest_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ctest_control FOREIGN KEY (control_id) REFERENCES controls (id) ON DELETE CASCADE,
  KEY ix_ctest_control (control_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
