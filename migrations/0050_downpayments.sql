-- 0050 down-payment (uang muka) receipts. Forward-only.
CREATE TABLE down_payments (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  receipt_no VARCHAR(40) NOT NULL,
  client_name VARCHAR(160) NOT NULL,
  matter_id BIGINT UNSIGNED NULL,
  amount_minor BIGINT NOT NULL DEFAULT 0,
  issued_on VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'issued',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dp_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_dp_org_no (org_id, receipt_no),
  KEY ix_dp_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
