-- 0079 quorum rounds + votes + dual-control overrides. Forward-only.
CREATE TABLE quorum_rounds (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  entity VARCHAR(20) NOT NULL DEFAULT 'matter',
  entity_id BIGINT UNSIGNED NOT NULL DEFAULT 0,
  title VARCHAR(200) NOT NULL DEFAULT '',
  required INT NOT NULL DEFAULT 1,
  members TEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_by VARCHAR(160) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_qround_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_qround_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE quorum_votes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  round_id BIGINT UNSIGNED NOT NULL,
  voter VARCHAR(160) NOT NULL DEFAULT '',
  decision VARCHAR(10) NOT NULL DEFAULT 'approve',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_qvote_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_qvote_round FOREIGN KEY (round_id) REFERENCES quorum_rounds (id) ON DELETE CASCADE,
  UNIQUE KEY uq_qvote_round_voter (round_id, voter),
  KEY ix_qvote_round (round_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE quorum_overrides (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  round_id BIGINT UNSIGNED NOT NULL,
  requested_by VARCHAR(160) NOT NULL DEFAULT '',
  approved_by VARCHAR(160) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'requested',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_qovr_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_qovr_round FOREIGN KEY (round_id) REFERENCES quorum_rounds (id) ON DELETE CASCADE,
  KEY ix_qovr_round (round_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
