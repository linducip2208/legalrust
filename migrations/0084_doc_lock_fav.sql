-- 0084 document locks + favorites. Forward-only.
CREATE TABLE doc_locks (
  doc_id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  holder VARCHAR(160) NOT NULL DEFAULT '',
  note VARCHAR(200) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_dlock_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_dlock_doc FOREIGN KEY (doc_id) REFERENCES documents (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE doc_favs (
  user_id VARCHAR(160) NOT NULL,
  org_id CHAR(36) NOT NULL,
  doc_id BIGINT UNSIGNED NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (user_id, doc_id),
  CONSTRAINT fk_dfav_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_dfav_doc FOREIGN KEY (doc_id) REFERENCES documents (id) ON DELETE CASCADE,
  KEY ix_dfav_user (user_id, org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
