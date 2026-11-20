-- 0112 matter budgets + expenses (incl. panjar advances). Forward-only.
CREATE TABLE matter_budgets (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  category VARCHAR(120) NOT NULL DEFAULT '',
  amount BIGINT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mbg_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mbg_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  CONSTRAINT chk_mbg_amount CHECK (amount >= 0),
  KEY ix_mbg_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE matter_expenses (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  matter_id BIGINT UNSIGNED NOT NULL,
  entry_date VARCHAR(10) NOT NULL DEFAULT '',
  kind VARCHAR(20) NOT NULL DEFAULT 'expense',
  category VARCHAR(120) NOT NULL DEFAULT '',
  amount BIGINT NOT NULL DEFAULT 0,
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mxp_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mxp_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  CONSTRAINT chk_mxp_amount CHECK (amount > 0),
  CONSTRAINT chk_mxp_kind CHECK (kind IN ('expense', 'advance')),
  KEY ix_mxp_matter (org_id, matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
