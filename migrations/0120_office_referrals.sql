-- 0120 inter-office matter referrals. Forward-only.
CREATE TABLE office_referrals (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  from_office_id BIGINT UNSIGNED NOT NULL,
  to_office_id BIGINT UNSIGNED NOT NULL,
  note VARCHAR(300) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'pending',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_orf_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_orf_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  CONSTRAINT fk_orf_from FOREIGN KEY (from_office_id) REFERENCES offices (id) ON DELETE CASCADE,
  CONSTRAINT fk_orf_to FOREIGN KEY (to_office_id) REFERENCES offices (id) ON DELETE CASCADE,
  KEY ix_orf_org (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
