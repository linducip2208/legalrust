-- 0009 trust (client escrow) ledger
CREATE TABLE trust_accounts (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, client_id BIGINT UNSIGNED NOT NULL,
  currency CHAR(3) NOT NULL DEFAULT 'IDR', balance BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_trust_client FOREIGN KEY (client_id) REFERENCES clients (id),
  KEY ix_trust_org (org_id),
  UNIQUE KEY uq_trust_org_client_ccy (org_id, client_id, currency)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE trust_entries (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, account_id BIGINT UNSIGNED NOT NULL,
  matter_id BIGINT UNSIGNED NULL, kind VARCHAR(20) NOT NULL,
  amount_minor BIGINT NOT NULL, ref_type VARCHAR(40) NOT NULL, ref_id BIGINT UNSIGNED NOT NULL,
  created_by CHAR(36) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_trustentry_acct FOREIGN KEY (account_id) REFERENCES trust_accounts (id) ON DELETE CASCADE,
  CONSTRAINT fk_trustentry_matter FOREIGN KEY (matter_id) REFERENCES matters (id),
  KEY ix_trustentry_acct (account_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
