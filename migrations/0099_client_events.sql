-- 0099 client events/seminars + attendances. Forward-only.
CREATE TABLE client_events (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL DEFAULT '',
  venue VARCHAR(200) NOT NULL DEFAULT '',
  held_on VARCHAR(10) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'planned',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_clev_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_cev_org_date (org_id, held_on)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE event_attendances (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  event_id BIGINT UNSIGNED NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'invited',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_eatt_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_eatt_event FOREIGN KEY (event_id) REFERENCES client_events (id) ON DELETE CASCADE,
  CONSTRAINT fk_eatt_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  UNIQUE KEY uq_eatt_pair (org_id, event_id, client_id),
  KEY ix_eatt_event (event_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
