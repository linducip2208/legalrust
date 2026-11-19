-- 0090 opportunity pipeline + win-back workflow. Forward-only.
CREATE TABLE client_opps (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(200) NOT NULL DEFAULT '',
  amount_minor BIGINT NOT NULL DEFAULT 0,
  stage VARCHAR(20) NOT NULL DEFAULT 'prospect',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_copp_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_copp_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  KEY ix_copp_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE client_winbacks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  reason VARCHAR(200) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'identified',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cwb_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cwb_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  KEY ix_cwb_client (org_id, client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
