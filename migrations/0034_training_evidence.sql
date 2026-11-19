-- 0034 training completion evidence. Forward-only.
CREATE TABLE training_evidence (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  training_id BIGINT UNSIGNED NOT NULL,
  note VARCHAR(500) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_tev_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_tev_training FOREIGN KEY (training_id) REFERENCES training_assignments (id) ON DELETE CASCADE,
  KEY ix_tev_training (training_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
