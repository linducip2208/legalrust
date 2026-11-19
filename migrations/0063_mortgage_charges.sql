-- 0063 mortgage-deed details (APHT/SKMHT support + expiry monitor). Forward-only.
-- SKMHT statutory validity: 1 month (registered land) / 3 months (unregistered);
-- the monitor surfaces approaching expiries, it does not extend them.
CREATE TABLE deed_charges (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  ppat_id BIGINT UNSIGNED NOT NULL,
  kind VARCHAR(20) NOT NULL DEFAULT 'apht',
  creditor VARCHAR(160) NOT NULL DEFAULT '',
  ceiling_minor BIGINT NOT NULL DEFAULT 0,
  valid_until VARCHAR(10) NOT NULL DEFAULT '',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_charge_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_charge_ppat FOREIGN KEY (ppat_id) REFERENCES ppat_entries (id) ON DELETE CASCADE,
  UNIQUE KEY uq_charge_ppat_kind (ppat_id, kind),
  KEY ix_charge_ppat (ppat_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
