-- 0014 inbound webhook delivery log (observability for F-D08-013/F-D25-017).
-- Forward-only. org_id NULL when the tenant is unknown at receipt time
-- (e.g. bad signature); tenant-scoped reads filter by org_id.
CREATE TABLE webhook_deliveries (
  id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  org_id CHAR(36) NULL,
  provider VARCHAR(40) NOT NULL,
  event VARCHAR(80) NOT NULL DEFAULT '',
  status VARCHAR(24) NOT NULL DEFAULT 'accepted',
  detail VARCHAR(255) NOT NULL DEFAULT '',
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_wh_org (org_id, created_at),
  KEY ix_wh_provider (provider, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
