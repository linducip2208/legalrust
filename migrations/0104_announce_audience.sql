-- 0104 announcement audience targeting. Forward-only.
ALTER TABLE announcements ADD COLUMN audience VARCHAR(20) NOT NULL DEFAULT 'staff';
