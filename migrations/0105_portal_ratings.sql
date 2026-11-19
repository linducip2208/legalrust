-- 0105 portal service ratings. Forward-only.
CREATE TABLE portal_ratings (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  ref_kind VARCHAR(20) NOT NULL DEFAULT 'matter',
  ref_id BIGINT UNSIGNED NOT NULL DEFAULT 0,
  score TINYINT NOT NULL DEFAULT 5,
  comment VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_prt_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_prt_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT chk_prt_kind CHECK (ref_kind IN ('matter', 'appointment', 'copy', 'general')),
  CONSTRAINT chk_prt_score CHECK (score BETWEEN 1 AND 5),
  KEY ix_prt_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
