-- 0033 gift & hospitality register (gratifikasi). Forward-only.
CREATE TABLE gift_register (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  direction VARCHAR(10) NOT NULL DEFAULT 'received',
  description VARCHAR(255) NOT NULL,
  counterpart VARCHAR(120) NOT NULL DEFAULT '',
  given_at VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'logged',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_gift_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_gift_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
