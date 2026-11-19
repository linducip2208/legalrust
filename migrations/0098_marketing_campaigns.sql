-- 0098 marketing campaigns + client participations. Forward-only.
CREATE TABLE marketing_campaigns (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  name VARCHAR(200) NOT NULL DEFAULT '',
  channel VARCHAR(20) NOT NULL DEFAULT 'email',
  status VARCHAR(20) NOT NULL DEFAULT 'planned',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mcamp_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT chk_mcamp_channel CHECK (channel IN ('email', 'phone', 'visit', 'event', 'ads')),
  KEY ix_mcamp_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE campaign_participations (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  campaign_id BIGINT UNSIGNED NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  response VARCHAR(20) NOT NULL DEFAULT 'invited',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_cpart_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_cpart_camp FOREIGN KEY (campaign_id) REFERENCES marketing_campaigns (id) ON DELETE CASCADE,
  CONSTRAINT fk_cpart_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  UNIQUE KEY uq_cpart_pair (org_id, campaign_id, client_id),
  KEY ix_cpart_camp (campaign_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
