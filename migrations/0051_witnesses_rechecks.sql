-- 0051 witness pool + client identity re-verification log. Forward-only.
-- Re-verification is a MANUAL log (Dukcapil/BPN integration remains BLOCKED
-- on external credentials); no external call is fabricated here.
CREATE TABLE witnesses (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  name VARCHAR(160) NOT NULL,
  id_number VARCHAR(40) NOT NULL DEFAULT '',
  phone VARCHAR(40) NOT NULL DEFAULT '',
  active TINYINT NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_wit_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_wit_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE rechecks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  subject VARCHAR(160) NOT NULL,
  method VARCHAR(60) NOT NULL DEFAULT 'document',
  result VARCHAR(20) NOT NULL DEFAULT 'pass',
  verified_on VARCHAR(10) NOT NULL DEFAULT '',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_recheck_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_recheck_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
