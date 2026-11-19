-- 0089 client referrals. Forward-only.
CREATE TABLE client_referrals (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  referrer_id BIGINT UNSIGNED NOT NULL,
  referred_id BIGINT UNSIGNED NOT NULL,
  note VARCHAR(200) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cref_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cref_from FOREIGN KEY (referrer_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_cref_to FOREIGN KEY (referred_id) REFERENCES clients (id) ON DELETE CASCADE,
  UNIQUE KEY uq_cref_pair (org_id, referrer_id, referred_id),
  KEY ix_cref_from (referrer_id),
  KEY ix_cref_to (referred_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
