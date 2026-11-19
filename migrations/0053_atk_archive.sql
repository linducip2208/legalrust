-- 0053 ATK kind tag + archive room location map. Forward-only.
-- kind tag is additive (default keeps existing rows valid).
ALTER TABLE inventory_items
  ADD COLUMN kind VARCHAR(20) NOT NULL DEFAULT 'general';
CREATE TABLE archive_boxes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  box_code VARCHAR(40) NOT NULL,
  location VARCHAR(120) NOT NULL DEFAULT '',
  description VARCHAR(255) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_abox_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_abox_org_code (org_id, box_code),
  KEY ix_abox_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
