-- 0056 bank-partner facility sheet + office feedback capture. Forward-only.
CREATE TABLE bank_facilities (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  bank_name VARCHAR(120) NOT NULL,
  facility VARCHAR(120) NOT NULL,
  rate_text VARCHAR(20) NOT NULL DEFAULT '',
  tenor VARCHAR(40) NOT NULL DEFAULT '',
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_bankf_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_bankf_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE feedbacks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  name VARCHAR(120) NOT NULL DEFAULT '',
  message TEXT NOT NULL,
  rating TINYINT NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL DEFAULT 'new',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_fb_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_fb_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
