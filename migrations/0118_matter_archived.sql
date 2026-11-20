-- 0118 matter archive flag. Forward-only.
ALTER TABLE matters ADD COLUMN archived TINYINT(1) NOT NULL DEFAULT 0;
CREATE INDEX ix_matter_archived ON matters (org_id, archived);
