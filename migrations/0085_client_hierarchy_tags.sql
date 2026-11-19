-- 0085 client hierarchy + tags. Forward-only.
CREATE TABLE client_links (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  child_id BIGINT UNSIGNED NOT NULL,
  parent_id BIGINT UNSIGNED NOT NULL,
  relation VARCHAR(20) NOT NULL DEFAULT 'subsidiary',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_chlink_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_chlink_child FOREIGN KEY (child_id) REFERENCES clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_chlink_parent FOREIGN KEY (parent_id) REFERENCES clients (id) ON DELETE CASCADE,
  UNIQUE KEY uq_clink_pair (org_id, child_id, parent_id),
  KEY ix_clink_child (child_id),
  KEY ix_clink_parent (parent_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE client_tags (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  client_id BIGINT UNSIGNED NOT NULL,
  tag VARCHAR(60) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_ctlink_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_ctlink_client FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
  UNIQUE KEY uq_ctag_triple (org_id, client_id, tag),
  KEY ix_ctag_client (client_id),
  KEY ix_ctag_tag (org_id, tag)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
