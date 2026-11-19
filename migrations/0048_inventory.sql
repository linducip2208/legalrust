-- 0048 office inventory (blanko/meterai) + usage log. Forward-only.
CREATE TABLE inventory_items (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  code VARCHAR(40) NOT NULL,
  name VARCHAR(200) NOT NULL,
  qty BIGINT NOT NULL DEFAULT 0,
  unit VARCHAR(20) NOT NULL DEFAULT 'pcs',
  min_stock BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_inv_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_inv_org_code (org_id, code),
  KEY ix_inv_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE meterai_usage (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  item_id BIGINT UNSIGNED NOT NULL,
  qty BIGINT NOT NULL DEFAULT 0,
  need VARCHAR(255) NOT NULL DEFAULT '',
  used_on VARCHAR(10) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_muse_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_muse_item FOREIGN KEY (item_id) REFERENCES inventory_items (id) ON DELETE CASCADE,
  KEY ix_muse_item (item_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
