-- 0116 matter parent (sub-matter / workstream grouping). Forward-only.
ALTER TABLE matters ADD COLUMN parent_id BIGINT UNSIGNED NULL;
ALTER TABLE matters ADD CONSTRAINT fk_matter_parent FOREIGN KEY (parent_id) REFERENCES matters (id) ON DELETE SET NULL;
CREATE INDEX ix_matter_parent ON matters (org_id, parent_id);
