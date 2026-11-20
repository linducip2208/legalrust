-- 0119 matter visibility (open|team) for per-matter access control. Forward-only.
ALTER TABLE matters ADD COLUMN visibility VARCHAR(20) NOT NULL DEFAULT 'open';
ALTER TABLE matters ADD CONSTRAINT chk_matter_visibility CHECK (visibility IN ('open', 'team'));
