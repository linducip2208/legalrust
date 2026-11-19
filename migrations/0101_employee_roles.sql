-- 0101 client employee contact roles. Forward-only.
ALTER TABLE client_employees ADD COLUMN contact_role VARCHAR(20) NOT NULL DEFAULT 'other';
