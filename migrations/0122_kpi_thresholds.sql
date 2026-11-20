-- 0122 KPI alert thresholds. Forward-only.
CREATE TABLE kpi_thresholds (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  metric VARCHAR(80) NOT NULL DEFAULT '',
  op VARCHAR(10) NOT NULL DEFAULT '>=',
  threshold BIGINT NOT NULL DEFAULT 0,
  active TINYINT(1) NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_kth_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT chk_kth_op CHECK (op IN ('>=', '<=')),
  UNIQUE KEY uq_kth_metric (org_id, metric),
  KEY ix_kth_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
