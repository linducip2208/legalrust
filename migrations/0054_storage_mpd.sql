-- 0054 deed storage boxes + MPD protocol transfers. Forward-only.
CREATE TABLE storage_boxes (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  box_code VARCHAR(40) NOT NULL,
  location VARCHAR(120) NOT NULL DEFAULT '',
  capacity INT NOT NULL DEFAULT 0,
  status VARCHAR(20) NOT NULL DEFAULT 'available',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_sbox_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  UNIQUE KEY uq_sbox_org_code (org_id, box_code),
  KEY ix_sbox_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE mpd_transfers (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NOT NULL,
  book_id BIGINT UNSIGNED NULL,
  recipient VARCHAR(160) NOT NULL,
  transfer_date VARCHAR(10) NOT NULL DEFAULT '',
  receipt_no VARCHAR(60) NOT NULL DEFAULT '',
  status VARCHAR(20) NOT NULL DEFAULT 'prepared',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_mpd_org FOREIGN KEY (org_id) REFERENCES organizations (id),
  CONSTRAINT fk_mpd_book FOREIGN KEY (book_id) REFERENCES protocol_books (id) ON DELETE SET NULL,
  KEY ix_mpd_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
