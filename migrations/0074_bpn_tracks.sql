-- 0074 BPN coordination tracker (manual mode; auto-sync pending BPN creds). Forward-only.
CREATE TABLE bpn_tracks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  kind VARCHAR(20) NOT NULL DEFAULT 'verification',
  ref_no VARCHAR(120) NOT NULL DEFAULT '',
  office VARCHAR(120) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_bpn_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_bpn_org_kind (org_id, kind)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
