-- 0002 core: clients, matters, documents, tasks
CREATE TABLE clients (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, kind VARCHAR(20) NOT NULL DEFAULT 'individual',
  display_name VARCHAR(255) NOT NULL, status VARCHAR(20) NOT NULL DEFAULT 'active',
  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  CONSTRAINT fk_clients_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  KEY ix_clients_org_status (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE matters (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, client_id BIGINT UNSIGNED NOT NULL,
  number VARCHAR(40) NOT NULL, title VARCHAR(300) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'open',
  row_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_matters_org_number (org_id, number),
  CONSTRAINT fk_matters_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_matters_client FOREIGN KEY (client_id) REFERENCES clients (id),
  KEY ix_matters_org_status (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE documents (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, matter_id BIGINT UNSIGNED NULL,
  name VARCHAR(255) NOT NULL, mime VARCHAR(120) NOT NULL, size_bytes BIGINT UNSIGNED NOT NULL,
  sha256 CHAR(64) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'active',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_docs_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_docs_matter FOREIGN KEY (matter_id) REFERENCES matters (id),
  KEY ix_docs_matter (matter_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE document_versions (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  document_id BIGINT UNSIGNED NOT NULL, version_no INT UNSIGNED NOT NULL,
  object_key VARCHAR(500) NOT NULL, sha256 CHAR(64) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uq_docver (document_id, version_no),
  CONSTRAINT fk_docver_doc FOREIGN KEY (document_id) REFERENCES documents (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE tasks (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL, matter_id BIGINT UNSIGNED NOT NULL,
  title VARCHAR(300) NOT NULL, status VARCHAR(40) NOT NULL DEFAULT 'open',
  due_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_tasks_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_tasks_matter FOREIGN KEY (matter_id) REFERENCES matters (id) ON DELETE CASCADE,
  KEY ix_tasks_matter_status (matter_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
