-- 0047 office cash book + cash advance (IOU) tracker. Forward-only.
CREATE TABLE cash_entries (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  kind VARCHAR(10) NOT NULL DEFAULT 'in',
  amount_minor BIGINT NOT NULL DEFAULT 0,
  note VARCHAR(255) NOT NULL DEFAULT '',
  occurred_on VARCHAR(10) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cash_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_cash_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE advances (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  holder VARCHAR(120) NOT NULL,
  amount_minor BIGINT NOT NULL DEFAULT 0,
  purpose VARCHAR(255) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_adv_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_adv_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
