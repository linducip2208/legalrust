-- 0046 service price list + deed fee quotations. Forward-only.
CREATE TABLE price_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  code VARCHAR(40) NOT NULL,
  name VARCHAR(200) NOT NULL,
  amount_minor BIGINT NOT NULL DEFAULT 0,
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_price_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_price_org_code (org_id, code),
  KEY ix_price_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE quotations (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_name VARCHAR(160) NOT NULL,
  deed_kind VARCHAR(60) NOT NULL DEFAULT '',
  total_minor BIGINT NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_quot_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_quot_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE quotation_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  quotation_id BIGINT UNSIGNED NOT NULL,
  price_code VARCHAR(40) NOT NULL,
  qty INT NOT NULL DEFAULT 1,
  amount_minor BIGINT NOT NULL DEFAULT 0,
  CONSTRAINT fk_qitem_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_qitem_quot FOREIGN KEY (quotation_id) REFERENCES quotations (id) ON DELETE CASCADE,
  KEY ix_qitem_quot (quotation_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
