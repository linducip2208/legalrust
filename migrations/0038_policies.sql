-- 0038 policy library with acknowledgement tracking. Forward-only.
CREATE TABLE policies (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  title VARCHAR(200) NOT NULL,
  version VARCHAR(20) NOT NULL DEFAULT '1.0',
  body MEDIUMTEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'draft',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_policy_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_policy_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE policy_acks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  policy_id BIGINT UNSIGNED NOT NULL,
  user_id CHAR(36) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ack_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ack_policy FOREIGN KEY (policy_id) REFERENCES policies (id) ON DELETE CASCADE,
  UNIQUE KEY uq_ack_policy_user (policy_id, user_id),
  KEY ix_ack_policy (policy_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
