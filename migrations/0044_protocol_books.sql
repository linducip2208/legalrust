-- 0044 protocol book register + custody/handover log. Forward-only.
CREATE TABLE protocol_books (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  book_no VARCHAR(40) NOT NULL,
  book_year CHAR(4) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'open',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_pbook_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_pbook_org_no (org_id, book_no),
  KEY ix_pbook_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE protocol_handovers (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  book_id BIGINT UNSIGNED NOT NULL,
  from_holder VARCHAR(120) NOT NULL,
  to_holder VARCHAR(120) NOT NULL,
  handed_at VARCHAR(10) NOT NULL DEFAULT '',
  note VARCHAR(500) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_phand_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_phand_book FOREIGN KEY (book_id) REFERENCES protocol_books (id) ON DELETE CASCADE,
  KEY ix_phand_book (book_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
